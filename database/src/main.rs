use std::error::Error;
use tracing::info;

mod commands;
mod config;
mod migration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenv::dotenv().ok();

    info!("Starting Charms Explorer Database Migration");

    // Create database if it doesn't exist
    info!("Ensuring database exists...");
    commands::create::execute(None).await?;

    // Run all pending migrations
    info!("Running migrations...");
    commands::migrate::execute(None).await?;

    info!("Database migration completed successfully");
    
    Ok(())
}
