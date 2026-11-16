//! A small log of what the call-watching parts are doing, so "why did it not name anyone?"
//! has an answer. One line per change (repeats are dropped), kept under ~200 KB, in
//! `~/Library/Logs/Graphite/capture.log`. No meeting content: only states and counts.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LAST: Mutex<Option<std::collections::HashMap<String, String>>> = Mutex::new(None);
const MAX_BYTES: u64 = 200_000;

pub fn path() -> Option<PathBuf> {
    Some(crate::platform::log_dir()?.join("capture.log"))
}

/// Record `state` for `who` (e.g. who = "names", state = "Accessibility not allowed"),
/// only when it differs from the last thing `who` said.
pub fn note(who: &str, state: &str) {
    {
        let mut guard = LAST.lock().unwrap_or_else(|e| e.into_inner());
        let last = guard.get_or_insert_with(Default::default);
        if last.get(who).map(String::as_str) == Some(state) {
            return;
        }
        last.insert(who.to_string(), state.to_string());
    }
    let Some(p) = path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::metadata(&p).map(|m| m.len() > MAX_BYTES).unwrap_or(false) {
        let _ = std::fs::rename(&p, p.with_extension("old"));
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        let _ = writeln!(f, "{} [{}] {}", chrono::Local::now().format("%H:%M:%S"), who, state);
    }
}

/// The last `n` lines, for the diagnostics report.
pub fn tail(n: usize) -> String {
    let Some(p) = path() else { return String::new() };
    let text = std::fs::read_to_string(p).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}
