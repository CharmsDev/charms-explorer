use log::{debug, error, info, warn};

/// Initialize the logger
pub fn init_logger() {
    env_logger::init();
}

/// Log an informational message
pub fn log_info(message: &str) {
    info!("{}", message);
}

/// Log a debug message
pub fn log_debug(message: &str) {
    debug!("{}", message);
}

/// Log a warning message
pub fn log_warning(message: &str) {
    warn!("{}", message);
}

/// Log an error message
pub fn log_error(message: &str) {
    error!("{}", message);
}

/// Log connection details for Bitcoin client
pub fn log_bitcoin_connection_details(
    host: &str,
    port: &str,
    username: &str,
    password: &str,
    network: &str,
) {
    info!(
        "Bitcoin connection details for {}: http://{}:{}@{}:{}",
        network, username, password, host, port
    );
}

/// Log database connection details
pub fn log_database_connection_details(url: &str) {
    info!("Database connection details: {}", url);
}
