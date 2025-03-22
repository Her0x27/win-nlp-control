use log::{debug, info, warn, error};

/// Logs a debug message.
#[inline]
pub fn log_debug(message: &str) {
    debug!("{}", message);
}

/// Logs an info message.
#[inline]
pub fn log_info(message: &str) {
    info!("{}", message);
}

/// Logs a warning message.
#[inline]
pub fn log_warn(message: &str) {
    warn!("{}", message);
}

/// Logs an error message.
#[inline]
pub fn log_error(message: &str) {
    error!("{}", message);
}
