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
