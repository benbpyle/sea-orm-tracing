# sea-orm-tracing

[![Crates.io](https://img.shields.io/crates/v/sea-orm-tracing.svg)](https://crates.io/crates/sea-orm-tracing)
[![Documentation](https://docs.rs/sea-orm-tracing/badge.svg)](https://docs.rs/sea-orm-tracing)
[![License](https://img.shields.io/crates/l/sea-orm-tracing.svg)](LICENSE)

OpenTelemetry-compatible tracing instrumentation for [SeaORM](https://github.com/SeaQL/sea-orm) database operations.

## Features

- **Automatic Instrumentation**: All queries executed through `TracedConnection` are traced
- **OpenTelemetry Compatible**: Spans include semantic conventions for database operations
- **Proper Span Nesting**: Database spans appear as children of HTTP request spans (works with tower-http, actix-web, etc.)
- **SQL Visibility**: Optionally include the actual SQL statement in spans
- **Performance Metrics**: Query duration, row counts, slow query detection, and error tracking
- **Zero Config**: Works out of the box with sensible defaults

## Quick Start

```rust
use sea_orm::Database;
use sea_orm_tracing::prelude::*;

// Connect to your database
let db = Database::connect("postgres://localhost/mydb").await?;

// Wrap with tracing - that's it!
let traced_db = db.with_tracing();

// Use it exactly like a normal DatabaseConnection
let users = Users::find().all(&traced_db).await?;
```

## Configuration

```rust
use sea_orm_tracing::{TracedConnection, TracingConfig};
use std::time::Duration;

let db = Database::connect("postgres://localhost/mydb").await?;

// Development: log everything
let traced_db = TracedConnection::new(db, TracingConfig::development());

// Production: minimal overhead
let traced_db = TracedConnection::new(db, TracingConfig::production());

// Custom configuration
let config = TracingConfig::default()
    .with_statement_logging(true)      // Include SQL in spans
    .with_parameter_logging(false)     // Don't log parameters (security)
    .with_slow_query_threshold(Duration::from_millis(100))
    .with_database_name("users_db");   // Useful for multi-db setups

let traced_db = TracedConnection::new(db, config);
```

## Span Attributes

The following [OpenTelemetry semantic convention](https://opentelemetry.io/docs/specs/semconv/database/) attributes are recorded:

| Attribute | Description | Example |
|-----------|-------------|---------|
| `db.system` | Database type | `postgresql`, `mysql`, `sqlite` |
| `db.operation` | SQL operation | `SELECT`, `INSERT`, `UPDATE`, `DELETE` |
| `db.sql.table` | Target table name | `users` |
| `db.statement` | Full SQL query (when enabled) | `SELECT * FROM users WHERE id = $1` |
| `db.rows_affected` | Number of rows returned/affected | `42` |
| `db.duration_ms` | Query execution time in milliseconds | `12` |
| `otel.status_code` | Result status | `OK` or `ERROR` |
| `error.message` | Error details (on failure) | `relation "users" does not exist` |
| `slow_query` | Whether query exceeded threshold | `true` |

## Integration with Web Frameworks

The magic of `sea-orm-tracing` is that database spans automatically become children of whatever span is currently active. This means if you're using tracing middleware in your web framework, you get perfect span hierarchies:

### Axum

```rust
use axum::{Router, routing::get, extract::State};
use tower_http::trace::TraceLayer;
use sea_orm_tracing::prelude::*;
use std::sync::Arc;

struct AppState {
    db: TracedConnection,
}

async fn get_users(State(state): State<Arc<AppState>>) -> String {
    // This span is automatically a child of the HTTP request span!
    let users = Users::find().all(&state.db).await.unwrap();
    format!("Found {} users", users.len())
}

let app = Router::new()
    .route("/users", get(get_users))
    .layer(TraceLayer::new_for_http())  // Creates HTTP request spans
    .with_state(Arc::new(AppState { db: traced_db }));
```

Resulting trace:
```
HTTP GET /users (200 OK) - 45ms
└── db.query SELECT users - 12ms
    ├── db.system: postgresql
    ├── db.operation: SELECT
    ├── db.sql.table: users
    └── db.rows_affected: 42
```

### Actix-web

```rust
use actix_web::{web, App, HttpServer};
use tracing_actix_web::TracingLogger;

HttpServer::new(move || {
    App::new()
        .wrap(TracingLogger::default())
        .app_data(web::Data::new(traced_db.clone()))
        .route("/users", web::get().to(get_users))
})
```

## Security Considerations

By default, `sea-orm-tracing` does **not** log SQL statements or parameters, as these may contain sensitive data. Use the configuration options carefully:

```rust
// SAFE for production
TracingConfig::production()

// Only enable in development/debugging
TracingConfig::development()

// Custom: log SQL but not parameters
TracingConfig::default()
    .with_statement_logging(true)
    .with_parameter_logging(false)
```

## Comparison with sqlx-tracing

This crate is inspired by [sqlx-tracing](https://docs.rs/sqlx-tracing) but designed specifically for SeaORM:

| Feature | sea-orm-tracing | sqlx-tracing |
|---------|-----------------|--------------|
| ORM Support | SeaORM entities & relations | Raw SQL only |
| Table Detection | Automatic from SQL parsing | Manual |
| Configuration | Builder pattern | Similar |
| Span Nesting | Via `tracing::Instrument` | Similar |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
