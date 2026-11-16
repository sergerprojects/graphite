//! A small always-on-top pop-up that appears when a calendar meeting with a video
//! link is about to start ("Maya / Owen 1:1  10:00-10:30   [Join Meet & start notes ▾]").
//! One click opens the video link and starts a recording for that event. Turn it
//! off in Settings, Calendar.

use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, Wry};
use tauri_plugin_store::StoreExt;

use crate::calendar::CalendarEvent;

const LABEL: &str = "meeting-alert";
const STORE_KEY: &str = "meeting_alert";
const WIDTH: f64 = 560.0;
const HEIGHT: f64 = 215.0; // room for the dropdown under the bar

static CURRENT: Mutex<Option<CalendarEvent>> = Mutex::new(None);

pub fn enabled(app: &AppHandle<Wry>) -> bool {
    app.store(crate::calendar::ical::STORE_FILE)
        .ok()
        .and_then(|st| st.get(STORE_KEY))
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

/// Only http(s) links are ever handed to `open`.
pub fn safe_link(url: &str) -> Option<&str> {
    let u = url.trim();
    (u.starts_with("https://") || u.starts_with("http://")).then_some(u)
}

#[derive(Serialize)]
pub struct AlertInfo {
    pub event: CalendarEvent,
    pub recording: bool,
}

/// Show the pop-up for this event. Returns false when it could not be shown.
pub fn show(app: &AppHandle<Wry>, event: &CalendarEvent) -> bool {
    if !enabled(app) {
        return false;
    }
    *CURRENT.lock().unwrap() = Some(event.clone());
    if app.get_webview_window(LABEL).is_some() {
        let _ = app.emit_to(LABEL, "meeting-alert-changed", ());
        return true;
    }
    let mut builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("meeting-alert.html".into()))
        .title("Graphite")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .resizable(false)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .focused(false)
        .shadow(false);
    let monitor = app
        .get_webview_window("main")
        .and_then(|w| w.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let scale = m.scale_factor();
        let pos = m.position().to_logical::<f64>(scale);
        let size = m.size().to_logical::<f64>(scale);
        builder = builder.position(pos.x + size.width - WIDTH - 16.0, pos.y + 44.0);
    }
    match builder.build() {
        Ok(_) => true,
        Err(e) => {
            log::warn!("meeting-alert: could not open window: {e}");
            false
        }
    }
}

fn close(app: &AppHandle<Wry>) {
    *CURRENT.lock().unwrap() = None;
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.close();
    }
}

/// Close the pop-up when it is for `event_id` (or for any event when None).
/// Returns true when a pop-up was closed.
pub fn close_for(app: &AppHandle<Wry>, event_id: Option<&str>) -> bool {
    let matches = {
        let cur = CURRENT.lock().unwrap();
        match (&*cur, event_id) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(e), Some(id)) => e.id == id,
        }
    };
    if matches {
        close(app);
    }
    matches
}

fn open_link(event: &CalendarEvent) {
    if let Some(url) = event.video_url.as_deref().and_then(safe_link) {
        if let Err(e) = crate::platform::open(url) {
            log::warn!("meeting-alert: could not open {url}: {e}");
        }
    }
}

#[tauri::command]
pub async fn meeting_alert_info() -> Option<AlertInfo> {
    let event = CURRENT.lock().unwrap().clone()?;
    Some(AlertInfo { event, recording: crate::audio::recording_commands::is_recording().await })
}

/// Join the call; with `record` also start notes for this event in Graphite.
#[tauri::command]
pub async fn meeting_alert_join(app: AppHandle<Wry>, record: bool) {
    let event = CURRENT.lock().unwrap().clone();
    close(&app);
    let Some(event) = event else { return };
    open_link(&event);
    if record && !crate::audio::recording_commands::is_recording().await {
        crate::tray::focus_main_window(&app);
        let payload = serde_json::json!({ "event": &event, "title": &event.summary });
        if let Err(e) = app.emit(crate::calendar::autorecord::RECORD_REQUEST_EVENT, payload) {
            log::warn!("meeting-alert: could not start recording: {e}");
        }
    }
}

#[tauri::command]
pub async fn meeting_alert_open_graphite(app: AppHandle<Wry>) {
    close(&app);
    crate::tray::focus_main_window(&app);
}

/// X button: forget this meeting for the session (no auto-record, no re-prompt).
#[tauri::command]
pub async fn meeting_alert_dismiss(app: AppHandle<Wry>) {
    let event = CURRENT.lock().unwrap().clone();
    close(&app);
    if let Some(e) = event {
        crate::calendar::calendar_dismiss_event(e.id).await;
    }
}

/// Close without remembering anything (recording started, meeting over).
#[tauri::command]
pub async fn meeting_alert_close(app: AppHandle<Wry>) {
    close(&app);
}

#[tauri::command]
pub async fn get_meeting_alert(app: AppHandle<Wry>) -> bool {
    enabled(&app)
}

#[tauri::command]
pub async fn set_meeting_alert(app: AppHandle<Wry>, enabled: bool) -> Result<(), String> {
    let store = app
        .store(crate::calendar::ical::STORE_FILE)
        .map_err(|e| format!("Failed to access store: {e}"))?;
    store.set(STORE_KEY, serde_json::json!(enabled));
    store.save().map_err(|e| format!("Failed to save store: {e}"))?;
    if !enabled {
        close(&app);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_web_links_are_opened() {
        assert_eq!(safe_link(" https://meet.google.com/abc-defg-hij "), Some("https://meet.google.com/abc-defg-hij"));
        assert_eq!(safe_link("http://example.com/x"), Some("http://example.com/x"));
        assert_eq!(safe_link("file:///etc/passwd"), None);
        assert_eq!(safe_link("-a Calculator"), None);
        assert_eq!(safe_link("javascript:alert(1)"), None);
    }
}
