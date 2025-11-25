//! Configuration for tracing behavior.

use std::time::Duration;

/// Configuration options for database tracing.
///
/// # Example
///
/// ```rust
/// use sea_orm_tracing::TracingConfig;
/// use std::time::Duration;
///
/// let config = TracingConfig::default()
///     .with_statement_logging(true)
///     .with_slow_query_threshold(Duration::from_millis(100));
/// ```
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Whether to include the SQL statement in spans.
    /// Default: `false` (for security - prevents accidental credential logging)
    pub log_statements: bool,

    /// Whether to include query parameters in spans.
    /// Default: `false` (parameters may contain sensitive data)
    pub log_parameters: bool,

    /// Threshold for logging slow queries at WARN level.
    /// Queries exceeding this duration will be logged with additional context.
    /// Default: 500ms
    pub slow_query_threshold: Duration,

    /// Whether to record the number of rows affected/returned.
    /// Default: `true`
    pub record_row_counts: bool,

    /// Target name for tracing events.
    /// Default: "sea_orm_tracing"
    pub target: &'static str,

    /// Custom database name to include in spans (useful for multi-database setups).
    /// Default: `None`
    pub database_name: Option<String>,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            log_statements: false,
            log_parameters: false,
            slow_query_threshold: Duration::from_millis(500),
            record_row_counts: true,
            target: "sea_orm_tracing",
            database_name: None,
        }
    }
}

impl TracingConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable SQL statement logging in spans.
    ///
    /// **Security Warning**: Enabling this may expose sensitive data in your traces
    /// if your queries contain credentials or PII in the SQL text itself.
    pub fn with_statement_logging(mut self, enabled: bool) -> Self {
        self.log_statements = enabled;
        self
    }

    /// Enable or disable parameter logging in spans.
    ///
    /// **Security Warning**: Query parameters often contain user input and
    /// potentially sensitive data. Only enable in development or controlled environments.
    pub fn with_parameter_logging(mut self, enabled: bool) -> Self {
        self.log_parameters = enabled;
        self
    }

    /// Set the threshold for slow query warnings.
    ///
    /// Queries taking longer than this duration will be logged at WARN level
    /// with the `slow_query` field set to `true`.
    pub fn with_slow_query_threshold(mut self, threshold: Duration) -> Self {
        self.slow_query_threshold = threshold;
        self
    }

    /// Enable or disable row count recording.
    pub fn with_row_count_recording(mut self, enabled: bool) -> Self {
        self.record_row_counts = enabled;
        self
    }

    /// Set a custom tracing target name.
    pub fn with_target(mut self, target: &'static str) -> Self {
        self.target = target;
        self
    }

    /// Set a database name to include in spans.
    ///
    /// Useful when your application connects to multiple databases.
    pub fn with_database_name(mut self, name: impl Into<String>) -> Self {
        self.database_name = Some(name.into());
        self
    }

    /// Create a development-friendly configuration with full logging enabled.
    ///
    /// **Warning**: Do not use in production as it logs all SQL and parameters.
    pub fn development() -> Self {
        Self {
            log_statements: true,
            log_parameters: true,
            slow_query_threshold: Duration::from_millis(100),
            record_row_counts: true,
            target: "sea_orm_tracing",
            database_name: None,
        }
    }

    /// Create a production-safe configuration with minimal overhead.
    pub fn production() -> Self {
        Self {
            log_statements: false,
            log_parameters: false,
            slow_query_threshold: Duration::from_secs(1),
            record_row_counts: true,
            target: "sea_orm_tracing",
            database_name: None,
        }
    }
}
