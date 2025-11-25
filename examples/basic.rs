//! Basic example showing how to use sea-orm-tracing.
//!
//! Run with: cargo run --example basic

use sea_orm::Database;
use sea_orm_tracing::prelude::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,sea_orm_tracing=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Connect to database
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/test".into());

    tracing::info!("Connecting to database...");

    let db = Database::connect(&database_url).await?;

    // Option 1: Simple wrapping with defaults
    let traced_db = TracedConnection::from(db);

    // Option 2: Using the extension trait (more fluent)
    // let traced_db = db.with_tracing();

    // Option 3: With custom configuration
    // let traced_db = db.with_tracing_config(
    //     TracingConfig::default()
    //         .with_statement_logging(true)
    //         .with_slow_query_threshold(Duration::from_millis(100))
    // );

    // Option 4: Development config (logs everything)
    // let traced_db = TracedConnection::new(db, TracingConfig::development());

    // All queries through traced_db are now instrumented!
    // Example query (would work with actual entities):
    //
    // let users = Users::find()
    //     .filter(users::Column::Active.eq(true))
    //     .all(&traced_db)
    //     .await?;

    tracing::info!("Database connection established with tracing enabled");

    // You can also access the inner connection if needed
    let _inner = traced_db.inner();

    Ok(())
}
