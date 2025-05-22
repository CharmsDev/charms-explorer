use clap::{Parser, Subcommand};
use std::error::Error;

mod config;
mod commands;

/// Charms Explorer Database Management CLI
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Command to execute
    #[command(subcommand)]
    command: Commands,
}

/// Available commands for database management
#[derive(Subcommand)]
enum Commands {
    /// Create a new database
    Create {
        /// Database name
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Run database migrations
    Migrate {
        /// Number of migrations to run (all if not specified)
        #[arg(short, long)]
        steps: Option<u32>,
    },
    /// Reset database (drop all tables and run migrations)
    Reset,
    /// Show database status
    Status,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Parse command line arguments
    let cli = Cli::parse();

    // Execute command
    match cli.command {
        Commands::Create { name } => {
            commands::create::execute(name).await?;
        }
        Commands::Migrate { steps } => {
            commands::migrate::execute(steps).await?;
        }
        Commands::Reset => {
            commands::migrate::reset().await?;
        }
        Commands::Status => {
            commands::migrate::status().await?;
        }
    }

    Ok(())
}
