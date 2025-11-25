//! Example showing sea-orm-tracing integration with Axum.
//!
//! This demonstrates how database spans automatically nest under HTTP request spans.
//!
//! Run with: cargo run --example axum

use std::sync::Arc;

use sea_orm::Database;
use sea_orm_tracing::prelude::*;

// This example shows the pattern, but won't compile without axum dependencies.
// Add these to Cargo.toml to run:
//
// [dev-dependencies]
// axum = "0.7"
// tower-http = { version = "0.5", features = ["trace"] }

fn main() {
    println!(
        r#"
This example demonstrates the integration pattern with Axum.

Your setup would look like:

```rust
use axum::{{Router, routing::get, extract::State}};
use sea_orm::Database;
use sea_orm_tracing::prelude::*;
use tower_http::trace::TraceLayer;
use std::sync::Arc;

// Application state with traced database
struct AppState {{
    db: TracedConnection,
}}

// Handler - database spans are automatically children of HTTP span
async fn get_users(State(state): State<Arc<AppState>>) -> String {{
    // This query creates a span that's a child of the HTTP request span
    let users = Users::find()
        .all(&state.db)
        .await
        .unwrap();

    format!("Found {{}} users", users.len())
}}

#[tokio::main]
async fn main() {{
    // Initialize tracing with OpenTelemetry
    tracing_subscriber::fmt::init();

    // Connect with tracing
    let db = Database::connect("postgres://localhost/mydb")
        .await
        .unwrap()
        .with_tracing_config(
            TracingConfig::default()
                .with_statement_logging(true)  // See SQL in dev
                .with_database_name("mydb")
        );

    let state = Arc::new(AppState {{ db }});

    let app = Router::new()
        .route("/users", get(get_users))
        .layer(TraceLayer::new_for_http())  // HTTP request spans
        .with_state(state);

    // Start server...
}}
```

The resulting trace hierarchy looks like:

    HTTP GET /users (200 OK) - 45ms
    └── db.query SELECT users - 12ms
        ├── db.system: postgresql
        ├── db.operation: SELECT
        ├── db.sql.table: users
        └── db.rows_affected: 42

This automatic nesting happens because sea-orm-tracing uses
`tracing::Instrument` which picks up the current span context.
"#
    );
}
