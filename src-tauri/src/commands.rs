//! Tauri commands.
//!
//! Writes flow through commands into [`AppState`]; the backend then broadcasts
//! the change to every window via an event (design D3).

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::app_state::AppState;

/// Event name for the health round-trip broadcast.
pub const HEALTH_EVENT: &str = "agentpet://health";

#[derive(Clone, Serialize)]
pub struct HealthPayload {
    pub ok: bool,
    pub tick: u64,
    pub ts_ms: u64,
}

/// Health check + event-bus self-test: bump the authoritative counter and
/// broadcast it to all windows. Proves the command -> backend -> event path.
#[tauri::command]
pub fn health(app: AppHandle, state: State<'_, AppState>) -> HealthPayload {
    let tick = {
        let mut ticks = state
            .health_ticks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *ticks += 1;
        *ticks
    };

    let ts_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let payload = HealthPayload {
        ok: true,
        tick,
        ts_ms,
    };

    if let Err(e) = app.emit(HEALTH_EVENT, payload.clone()) {
        tracing::warn!("failed to emit health event: {e}");
    }
    payload
}
