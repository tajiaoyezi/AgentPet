//! Authoritative backend state shared across windows.
//!
//! Each window is an isolated webview; the frontend never shares state directly
//! across windows. All cross-window state lives here, is mutated only via Tauri
//! commands, and is broadcast back to windows via events.

use std::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    /// Monotonic counter incremented by the `health` command.
    pub health_ticks: Mutex<u64>,
}
