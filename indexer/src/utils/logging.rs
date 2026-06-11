//! Logging facade.
//!
//! Internally writes through `tracing`; the `log` crate calls inside our
//! dependencies are bridged via `tracing-subscriber`'s `log` feature so
//! everything ends up in the same pipeline.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialise the global tracing subscriber.
///
/// Reads `RUST_LOG` for filter directives (e.g. `RUST_LOG=info,sqlx=warn`).
/// Defaults to `info` for the indexer, `warn` for noisy DB drivers.
/// Also installs `tracing-log`'s `LogTracer` so transitive `log::...` calls
/// from dependencies (sqlx, reqwest, …) flow through the same pipeline.
pub fn init_logger() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,sqlx=warn")
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false))
        .init();

    let _ = tracing_log::LogTracer::init();
}

/// Plain informational message. Prefer `tracing::info!(...)` macros with
/// structured fields in new code.
pub fn log_info(message: &str) {
    tracing::info!("{}", message);
}

pub fn log_debug(message: &str) {
    tracing::debug!("{}", message);
}

pub fn log_warning(message: &str) {
    tracing::warn!("{}", message);
}

pub fn log_error(message: &str) {
    tracing::error!("{}", message);
}
