use std::env;
use std::error::Error;
use tracing::error;

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,
    /// Database host
    pub host: String,
    /// Database port
    pub port: String,
    /// Database name
    pub name: String,
    /// Database user
    pub user: String,
    /// Database password
    pub password: String,
}

impl DatabaseConfig {
    /// Load database configuration from environment variables
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        // Get database URL from environment
        let url = match env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) => {
                error!("DATABASE_URL environment variable not set");
                return Err("DATABASE_URL environment variable not set".into());
            }
        };

        // Parse database URL to extract components
        let parts: Vec<&str> = url.split("://").collect();
        if parts.len() != 2 {
            error!("Invalid DATABASE_URL format");
            return Err("Invalid DATABASE_URL format".into());
        }

        let auth_and_path: Vec<&str> = parts[1].split('@').collect();
        if auth_and_path.len() != 2 {
            error!("Invalid DATABASE_URL format");
            return Err("Invalid DATABASE_URL format".into());
        }

        let user_pass: Vec<&str> = auth_and_path[0].split(':').collect();
        if user_pass.len() != 2 {
            error!("Invalid DATABASE_URL format");
            return Err("Invalid DATABASE_URL format".into());
        }

        let host_port_db: Vec<&str> = auth_and_path[1].split('/').collect();
        if host_port_db.len() != 2 {
            error!("Invalid DATABASE_URL format");
            return Err("Invalid DATABASE_URL format".into());
        }

        let host_port: Vec<&str> = host_port_db[0].split(':').collect();
        if host_port.len() != 2 {
            error!("Invalid DATABASE_URL format");
            return Err("Invalid DATABASE_URL format".into());
        }

        Ok(Self {
            url: url.clone(),
            host: host_port[0].to_string(),
            port: host_port[1].to_string(),
            name: host_port_db[1].to_string(),
            user: user_pass[0].to_string(),
            password: user_pass[1].to_string(),
        })
    }
}
