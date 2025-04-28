use charms_indexer::utils::logging;
use migration::{Migrator, MigratorTrait};
use sea_orm::Database;
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger
    logging::init_logger();

    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    logging::log_info("Running database migrations...");

    let connection = Database::connect(&database_url).await?;

    Migrator::up(&connection, None).await?;

    logging::log_info("Migrations completed successfully!");

    Ok(())
}
