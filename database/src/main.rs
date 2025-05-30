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

    info!("Starting Charms Explorer Database Service");

    // Create database if it doesn't exist
    info!("Ensuring database exists...");
    commands::create::execute(None).await?;

    // Run all pending migrations
    info!("Running migrations...");
    commands::migrate::execute(None).await?;

    info!("Database setup completed successfully");

    // Keep the service running
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        info!("Database service is running...");
    }
}
