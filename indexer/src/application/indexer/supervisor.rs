//! Generic task supervisor: spawn a worker, re-spawn on panic, give up on
//! cancellation. Used by `network_manager` to keep background workers like
//! the mempool processor alive even when they panic — fixing the silent
//! task-death that caused the bloque 946,620 incident.

use std::future::Future;
use std::time::Duration;

use crate::utils::logging;

/// Backoff between restarts after a panic. Constant on purpose — we'd rather
/// log a noisy "restarted N times" than mask the real cause with exponential
/// silence.
const RESTART_BACKOFF: Duration = Duration::from_secs(30);

/// Drive a long-running worker. Panics are caught and the worker is
/// re-spawned after a backoff. Clean exits (worker returns `Ok(())`)
/// terminate the supervisor.
///
/// `name` is purely for logs; pick something the operator can grep for.
pub async fn supervise<F, Fut>(name: &str, make_task: F)
where
    F: Fn() -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    supervise_with_backoff(name, make_task, RESTART_BACKOFF).await
}

async fn supervise_with_backoff<F, Fut>(name: &str, make_task: F, backoff: Duration)
where
    F: Fn() -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    let mut restarts: u64 = 0;
    loop {
        let handle = tokio::spawn(make_task());
        match handle.await {
            Ok(()) => {
                logging::log_info(&format!("[{}] supervised task exited cleanly", name));
                return;
            }
            Err(join_err) if join_err.is_panic() => {
                restarts += 1;
                logging::log_error(&format!(
                    "[{}] supervised task panicked (restart #{}). Backing off {}s.",
                    name,
                    restarts,
                    backoff.as_secs()
                ));
                tokio::time::sleep(backoff).await;
            }
            Err(_cancelled) => {
                logging::log_info(&format!("[{}] supervised task cancelled", name));
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Duration as StdDuration;

    /// The first 2 spawns panic; the 3rd exits cleanly. Supervisor must call
    /// the factory 3 times and then return.
    #[tokio::test]
    async fn restarts_on_panic_then_stops_on_clean_exit() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_inner = calls.clone();

        let factory = move || {
            let n = calls_inner.fetch_add(1, Ordering::SeqCst);
            async move {
                if n < 2 {
                    panic!("synthetic panic {n}");
                }
            }
        };

        supervise_with_backoff("test", factory, StdDuration::from_millis(5)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    /// Clean exit on the very first try → factory called exactly once.
    #[tokio::test]
    async fn clean_exit_first_try_does_not_restart() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_inner = calls.clone();
        let factory = move || {
            calls_inner.fetch_add(1, Ordering::SeqCst);
            async move {}
        };
        supervise_with_backoff("test", factory, StdDuration::from_millis(5)).await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
