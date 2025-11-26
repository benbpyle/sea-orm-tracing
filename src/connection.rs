//! Traced database connection wrapper.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use sea_orm::{
    AccessMode, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr,
    ExecResult, IsolationLevel, QueryResult, Statement, StreamTrait, TransactionError,
    TransactionTrait,
};
use tracing::{field, Instrument, Span};

use crate::config::TracingConfig;
use crate::parser::ParsedSql;

/// A traced wrapper around SeaORM's `DatabaseConnection`.
///
/// This wrapper implements `ConnectionTrait`, `StreamTrait`, and `TransactionTrait`,
/// making it a drop-in replacement for `DatabaseConnection`. All database operations
/// are automatically instrumented with tracing spans.
///
/// # Span Nesting
///
/// Spans created by `TracedConnection` automatically become children of the current
/// tracing span context. This means if you're using tracing middleware in your web
/// framework (e.g., `tower-http`'s `TraceLayer`), database spans will appear nested
/// under HTTP request spans in your traces.
///
/// # Example
///
/// ```rust,ignore
/// use sea_orm::Database;
/// use sea_orm_tracing::TracedConnection;
///
/// let db = Database::connect("postgres://localhost/mydb").await?;
/// let traced = TracedConnection::from(db);
///
/// // All queries are now traced
/// let users = Users::find().all(&traced).await?;
/// ```
#[derive(Debug, Clone)]
pub struct TracedConnection {
    inner: DatabaseConnection,
    config: Arc<TracingConfig>,
}

impl TracedConnection {
    /// Create a new traced connection with the given configuration.
    pub fn new(connection: DatabaseConnection, config: TracingConfig) -> Self {
        Self {
            inner: connection,
            config: Arc::new(config),
        }
    }

    /// Create a new traced connection with default configuration.
    pub fn wrap(connection: DatabaseConnection) -> Self {
        Self::new(connection, TracingConfig::default())
    }

    /// Get a reference to the underlying `DatabaseConnection`.
    pub fn inner(&self) -> &DatabaseConnection {
        &self.inner
    }

    /// Get the tracing configuration.
    pub fn config(&self) -> &TracingConfig {
        &self.config
    }

    /// Consume the wrapper and return the inner `DatabaseConnection`.
    pub fn into_inner(self) -> DatabaseConnection {
        self.inner
    }

    /// Get the database backend name for span attributes.
    fn db_system(&self) -> &'static str {
        match self.inner.get_database_backend() {
            DbBackend::Postgres => "postgresql",
            DbBackend::MySql => "mysql",
            DbBackend::Sqlite => "sqlite",
        }
    }

    /// Create a tracing span for a database operation.
    fn create_span(&self, stmt: &Statement) -> Span {
        let parsed = ParsedSql::parse(&stmt.sql);
        let span_name = parsed.span_name();
        let db_system = self.db_system();

        let span = tracing::info_span!(
            "db.query",
            otel.name = %span_name,
            db.system = %db_system,
            db.operation = %parsed.operation.as_str(),
            db.sql.table = field::Empty,
            db.statement = field::Empty,
            db.rows_affected = field::Empty,
            db.duration_ms = field::Empty,
            db.name = field::Empty,
            server.address = field::Empty,
            server.port = field::Empty,
            peer.service = field::Empty,
            otel.status_code = field::Empty,
            error.message = field::Empty,
            slow_query = field::Empty,
        );

        // Record table if available
        if let Some(table) = &parsed.table {
            span.record("db.sql.table", table.as_str());
        }

        // Record database name if configured
        if let Some(db_name) = &self.config.database_name {
            span.record("db.name", db_name.as_str());
        }

        // Record server address and port for X-Ray service map
        if let Some(addr) = &self.config.server_address {
            span.record("server.address", addr.as_str());
        }
        if let Some(port) = self.config.server_port {
            span.record("server.port", port as i64);
        }

        // Record peer service for X-Ray trace map node naming
        if let Some(peer) = &self.config.peer_service {
            span.record("peer.service", peer.as_str());
        }

        // Record SQL statement if configured
        if self.config.log_statements {
            span.record("db.statement", stmt.sql.as_str());
        }

        span
    }

    /// Record the result of a database operation in the span.
    fn record_result<T, E: std::fmt::Display>(
        &self,
        span: &Span,
        result: &Result<T, E>,
        start: Instant,
        row_count: Option<u64>,
    ) {
        let duration_ms = start.elapsed().as_millis() as i64;
        span.record("db.duration_ms", duration_ms);

        // Record row count if available and configured
        if self.config.record_row_counts {
            if let Some(count) = row_count {
                span.record("db.rows_affected", count);
            }
        }

        // Check for slow query
        if start.elapsed() > self.config.slow_query_threshold {
            span.record("slow_query", true);
            let threshold_ms = self.config.slow_query_threshold.as_millis() as i64;
            tracing::warn!(
                parent: span,
                duration_ms = duration_ms,
                threshold_ms = threshold_ms,
                "Slow query detected"
            );
        }

        match result {
            Ok(_) => {
                span.record("otel.status_code", "OK");
            }
            Err(e) => {
                span.record("otel.status_code", "ERROR");
                span.record("error.message", e.to_string().as_str());
                tracing::error!(
                    parent: span,
                    error = %e,
                    "Database query failed"
                );
            }
        }
    }
}

impl From<DatabaseConnection> for TracedConnection {
    fn from(connection: DatabaseConnection) -> Self {
        Self::wrap(connection)
    }
}

impl AsRef<DatabaseConnection> for TracedConnection {
    fn as_ref(&self) -> &DatabaseConnection {
        &self.inner
    }
}

#[async_trait]
impl ConnectionTrait for TracedConnection {
    fn get_database_backend(&self) -> DbBackend {
        self.inner.get_database_backend()
    }

    async fn execute(&self, stmt: Statement) -> Result<ExecResult, DbErr> {
        let span = self.create_span(&stmt);
        let start = Instant::now();

        let result = self
            .inner
            .execute(stmt)
            .instrument(span.clone())
            .await;

        let row_count = result.as_ref().ok().map(|r| r.rows_affected());
        self.record_result(&span, &result, start, row_count);

        result
    }

    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        let stmt = Statement::from_string(self.get_database_backend(), sql);
        let span = self.create_span(&stmt);
        let start = Instant::now();

        let result = self
            .inner
            .execute_unprepared(sql)
            .instrument(span.clone())
            .await;

        let row_count = result.as_ref().ok().map(|r| r.rows_affected());
        self.record_result(&span, &result, start, row_count);

        result
    }

    async fn query_one(&self, stmt: Statement) -> Result<Option<QueryResult>, DbErr> {
        let span = self.create_span(&stmt);
        let start = Instant::now();

        let result = self
            .inner
            .query_one(stmt)
            .instrument(span.clone())
            .await;

        let row_count = result.as_ref().ok().map(|opt| if opt.is_some() { 1 } else { 0 });
        self.record_result(&span, &result, start, row_count);

        result
    }

    async fn query_all(&self, stmt: Statement) -> Result<Vec<QueryResult>, DbErr> {
        let span = self.create_span(&stmt);
        let start = Instant::now();

        let result = self
            .inner
            .query_all(stmt)
            .instrument(span.clone())
            .await;

        let row_count = result.as_ref().ok().map(|rows| rows.len() as u64);
        self.record_result(&span, &result, start, row_count);

        result
    }

    fn support_returning(&self) -> bool {
        self.inner.support_returning()
    }

    fn is_mock_connection(&self) -> bool {
        self.inner.is_mock_connection()
    }
}

#[async_trait]
impl StreamTrait for TracedConnection {
    type Stream<'a> = <DatabaseConnection as StreamTrait>::Stream<'a>;

    fn stream<'a>(
        &'a self,
        stmt: Statement,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Stream<'a>, DbErr>> + 'a + Send>> {
        let span = self.create_span(&stmt);
        let start = Instant::now();
        let config = self.config.clone();

        Box::pin(async move {
            let result = self.inner.stream(stmt).instrument(span.clone()).await;

            // Record basic result info (we can't know row count for streams)
            let duration_ms = start.elapsed().as_millis() as i64;
            span.record("db.duration_ms", duration_ms);

            if start.elapsed() > config.slow_query_threshold {
                span.record("slow_query", true);
            }

            match &result {
                Ok(_) => {
                    span.record("otel.status_code", "OK");
                }
                Err(e) => {
                    span.record("otel.status_code", "ERROR");
                    span.record("error.message", e.to_string().as_str());
                }
            }

            result
        })
    }
}

#[async_trait]
impl TransactionTrait for TracedConnection {
    async fn begin(&self) -> Result<DatabaseTransaction, DbErr> {
        let span = tracing::info_span!(
            "db.transaction",
            otel.name = "BEGIN",
            db.system = %self.db_system(),
            db.operation = "BEGIN",
            otel.status_code = field::Empty,
            error.message = field::Empty,
        );

        let result = self.inner.begin().instrument(span.clone()).await;

        match &result {
            Ok(_) => {
                span.record("otel.status_code", "OK");
            }
            Err(e) => {
                span.record("otel.status_code", "ERROR");
                span.record("error.message", e.to_string().as_str());
            }
        }

        result
    }

    async fn begin_with_config(
        &self,
        isolation_level: Option<IsolationLevel>,
        access_mode: Option<AccessMode>,
    ) -> Result<DatabaseTransaction, DbErr> {
        let span = tracing::info_span!(
            "db.transaction",
            otel.name = "BEGIN",
            db.system = %self.db_system(),
            db.operation = "BEGIN",
            db.transaction.isolation_level = ?isolation_level,
            db.transaction.access_mode = ?access_mode,
            otel.status_code = field::Empty,
            error.message = field::Empty,
        );

        let result = self
            .inner
            .begin_with_config(isolation_level, access_mode)
            .instrument(span.clone())
            .await;

        match &result {
            Ok(_) => {
                span.record("otel.status_code", "OK");
            }
            Err(e) => {
                span.record("otel.status_code", "ERROR");
                span.record("error.message", e.to_string().as_str());
            }
        }

        result
    }

    async fn transaction<F, T, E>(&self, callback: F) -> Result<T, TransactionError<E>>
    where
        F: for<'c> FnOnce(
                &'c DatabaseTransaction,
            ) -> Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'c>>
            + Send,
        T: Send,
        E: std::fmt::Display + std::fmt::Debug + Send,
    {
        let span = tracing::info_span!(
            "db.transaction",
            otel.name = "TRANSACTION",
            db.system = %self.db_system(),
            db.operation = "TRANSACTION",
            otel.status_code = field::Empty,
            error.message = field::Empty,
        );

        let result = self
            .inner
            .transaction(callback)
            .instrument(span.clone())
            .await;

        match &result {
            Ok(_) => {
                span.record("otel.status_code", "OK");
            }
            Err(e) => {
                span.record("otel.status_code", "ERROR");
                span.record("error.message", format!("{:?}", e).as_str());
            }
        }

        result
    }

    async fn transaction_with_config<F, T, E>(
        &self,
        callback: F,
        isolation_level: Option<IsolationLevel>,
        access_mode: Option<AccessMode>,
    ) -> Result<T, TransactionError<E>>
    where
        F: for<'c> FnOnce(
                &'c DatabaseTransaction,
            ) -> Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'c>>
            + Send,
        T: Send,
        E: std::fmt::Display + std::fmt::Debug + Send,
    {
        let span = tracing::info_span!(
            "db.transaction",
            otel.name = "TRANSACTION",
            db.system = %self.db_system(),
            db.operation = "TRANSACTION",
            db.transaction.isolation_level = ?isolation_level,
            db.transaction.access_mode = ?access_mode,
            otel.status_code = field::Empty,
            error.message = field::Empty,
        );

        let result = self
            .inner
            .transaction_with_config(callback, isolation_level, access_mode)
            .instrument(span.clone())
            .await;

        match &result {
            Ok(_) => {
                span.record("otel.status_code", "OK");
            }
            Err(e) => {
                span.record("otel.status_code", "ERROR");
                span.record("error.message", format!("{:?}", e).as_str());
            }
        }

        result
    }
}

/// Extension trait for easy wrapping of database connections.
pub trait TracingExt {
    /// Wrap this connection with tracing instrumentation.
    fn with_tracing(self) -> TracedConnection;

    /// Wrap this connection with custom tracing configuration.
    fn with_tracing_config(self, config: TracingConfig) -> TracedConnection;
}

impl TracingExt for DatabaseConnection {
    fn with_tracing(self) -> TracedConnection {
        TracedConnection::wrap(self)
    }

    fn with_tracing_config(self, config: TracingConfig) -> TracedConnection {
        TracedConnection::new(self, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = TracingConfig::default()
            .with_statement_logging(true)
            .with_database_name("test_db");

        assert!(config.log_statements);
        assert_eq!(config.database_name, Some("test_db".to_string()));
    }

    #[test]
    fn test_development_config() {
        let config = TracingConfig::development();
        assert!(config.log_statements);
        assert!(config.log_parameters);
    }

    #[test]
    fn test_production_config() {
        let config = TracingConfig::production();
        assert!(!config.log_statements);
        assert!(!config.log_parameters);
    }
}
