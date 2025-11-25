//! # sea-orm-tracing
//!
//! OpenTelemetry-compatible tracing instrumentation for SeaORM database operations.
//!
//! This crate provides transparent tracing for all SeaORM database queries, automatically
//! creating spans with proper parent-child relationships that integrate with your existing
//! tracing infrastructure (like HTTP request spans from axum or actix-web).
//!
//! ## Features
//!
//! - **Automatic Instrumentation**: All queries executed through `TracedConnection` are traced
//! - **OpenTelemetry Compatible**: Spans include semantic conventions for database operations
//! - **Proper Span Nesting**: Database spans appear as children of HTTP request spans
//! - **SQL Visibility**: Optionally include the actual SQL statement in spans
//! - **Performance Metrics**: Query duration, row counts, and error tracking
//! - **Zero Config**: Works out of the box with sensible defaults
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use sea_orm::Database;
//! use sea_orm_tracing::TracedConnection;
//!
//! // Wrap your existing connection
//! let db = Database::connect("postgres://localhost/mydb").await?;
//! let traced_db = TracedConnection::from(db);
//!
//! // Use it exactly like a normal DatabaseConnection
//! let users = Users::find().all(&traced_db).await?;
//! ```
//!
//! ## Configuration
//!
//! ```rust,ignore
//! use sea_orm_tracing::{TracedConnection, TracingConfig};
//!
//! let config = TracingConfig::default()
//!     .with_statement_logging(true)  // Include SQL in spans (default: false for security)
//!     .with_parameter_logging(false) // Include query parameters (default: false)
//!     .with_slow_query_threshold(Duration::from_millis(100));
//!
//! let traced_db = TracedConnection::new(db, config);
//! ```
//!
//! ## Span Attributes
//!
//! The following OpenTelemetry semantic convention attributes are recorded:
//!
//! | Attribute | Description |
//! |-----------|-------------|
//! | `db.system` | Always "postgresql", "mysql", or "sqlite" |
//! | `db.operation` | SQL operation (SELECT, INSERT, UPDATE, DELETE) |
//! | `db.sql.table` | Target table name (when detectable) |
//! | `db.statement` | Full SQL query (when enabled) |
//! | `db.rows_affected` | Number of rows returned/affected |
//! | `otel.status_code` | "OK" or "ERROR" |
//! | `error.message` | Error details (on failure) |

mod config;
mod connection;
mod parser;

pub use config::TracingConfig;
pub use connection::{TracedConnection, TracingExt};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{TracedConnection, TracingConfig, TracingExt};
}
