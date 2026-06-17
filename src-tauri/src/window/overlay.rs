//! Overlay window helpers.

use tauri::WebviewWindow;

/// Re-assert always-on-top for the overlay right after creation.
///
/// M0 only re-applies the Tauri flag; M6 will extend this with a Win32
/// `SetWindowPos(HWND_TOPMOST)` fallback and periodic re-apply (§20.1 / §26.7).
pub fn reapply_topmost(overlay: &WebviewWindow) {
    if let Err(e) = overlay.set_always_on_top(true) {
        tracing::warn!("overlay set_always_on_top failed: {e}");
    }
}
