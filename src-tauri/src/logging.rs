//! Best-effort file logging via `tracing` + `tracing-appender`.

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;

/// Initialize tracing to `<logs_dir>/agentpet.log`.
///
/// Returns a [`WorkerGuard`] that MUST be kept alive for the lifetime of the app
/// (dropping it flushes and stops the non-blocking writer). On any failure this
/// returns `None` and logging is effectively disabled — it never panics, so the
/// app can still start (app-logging spec: "日志失败不阻塞应用").
pub fn init(logs_dir: &Path) -> Option<WorkerGuard> {
    if std::fs::create_dir_all(logs_dir).is_err() {
        return None;
    }

    let file_appender = tracing_appender::rolling::never(logs_dir, "agentpet.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let init_result = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_target(false)
        .with_writer(non_blocking)
        .try_init();

    match init_result {
        Ok(()) => Some(guard),
        Err(_) => None,
    }
}
