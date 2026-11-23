use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Start a background heartbeat that emits "scheduler-tick" every 30 seconds.
/// The frontend bot scheduler listens to this event for fixed-time trigger checks,
/// ensuring reliable timing even when macOS throttles WebView timers.
pub fn start_heartbeat(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let _ = app.emit("scheduler-tick", ());
        }
    });
}
