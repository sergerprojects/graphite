//! Things a first-time teammate needs: "is everything set up?", "is there a newer
//! version?", "install the voice models", "copy a diagnostics report".

use serde::Serialize;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Wry};
use tauri_plugin_store::StoreExt;

/// Where new versions are announced. Change this one line if the project moves.
/// Where the newest release lives; GitHub redirects it to .../releases/tag/<version>.
/// (The JSON API is rate-limited per network, which an office shares; this is not.)
pub const LATEST_RELEASE_URL: &str = "https://github.com/sergerprojects/graphite/releases/latest";
pub const INSTALL_COMMAND: &str =
    "curl -fsSL https://raw.githubusercontent.com/sergerprojects/graphite/main/scripts/install-release.sh | bash";
const MODEL_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet293_LM.onnx";
const MODEL_FILE: &str = "wespeaker_en_voxceleb_resnet293_LM.onnx";

static REMEMBER_VOICES: AtomicBool = AtomicBool::new(false);
const REMEMBER_KEY: &str = "remember_voices";

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
}

/// Whether Graphite may save confirmed voices for the next meeting (off until switched on).
pub fn remember_voices() -> bool {
    REMEMBER_VOICES.load(Ordering::Relaxed)
}

pub fn load_remember_voices(app: &AppHandle<Wry>) {
    let on = app
        .store(crate::calendar::ical::STORE_FILE)
        .ok()
        .and_then(|st| st.get(REMEMBER_KEY))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    REMEMBER_VOICES.store(on, Ordering::Relaxed);
}

#[tauri::command]
pub async fn get_remember_voices() -> bool {
    remember_voices()
}

#[tauri::command]
pub async fn set_remember_voices(app: AppHandle<Wry>, enabled: bool) -> Result<(), String> {
    let store = app.store(crate::calendar::ical::STORE_FILE).map_err(|e| e.to_string())?;
    store.set(REMEMBER_KEY, serde_json::json!(enabled));
    store.save().map_err(|e| e.to_string())?;
    REMEMBER_VOICES.store(enabled, Ordering::Relaxed);
    Ok(())
}

// ---------------------------------------------------------------------------
// Claude Code
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ClaudeStatus {
    pub installed: bool,
    pub path: Option<String>,
}

#[tauri::command]
pub async fn claude_code_status() -> ClaudeStatus {
    let path = crate::chat::find_claude();
    ClaudeStatus { installed: path.is_some(), path: path.map(|p| p.display().to_string()) }
}

// ---------------------------------------------------------------------------
// Updates
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub newer: bool,
    pub url: String,
    pub notes: String,
    pub command: String,
}

/// "0.9.10" -> [0, 9, 10]; ignores a leading "v".
fn parts(v: &str) -> Vec<u64> {
    v.trim().trim_start_matches('v').split('.').map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0)).collect()
}

pub fn is_newer(current: &str, latest: &str) -> bool {
    let (a, b) = (parts(current), parts(latest));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if y != x {
            return y > x;
        }
    }
    false
}

/// "https://github.com/o/r/releases/tag/v0.9.9" (or a relative path) -> "v0.9.9".
pub fn tag_from_location(location: &str) -> Option<String> {
    let tag = location.rsplit("/releases/tag/").next().filter(|_| location.contains("/releases/tag/"))?;
    let tag = tag.split(['?', '#']).next()?.trim();
    (!tag.is_empty()).then(|| tag.to_string())
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle<Wry>) -> Result<UpdateInfo, String> {
    let current = app.package_info().version.to_string();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Graphite-update-check")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(LATEST_RELEASE_URL).send().await.map_err(|_| "Couldn't reach GitHub.".to_string())?;
    let location = resp
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| "No release found (the project may have moved).".to_string())?;
    let tag = tag_from_location(&location).ok_or_else(|| "No release found (the project may have moved).".to_string())?;
    let url = if location.starts_with("http") { location } else { format!("https://github.com{location}") };
    Ok(UpdateInfo {
        newer: is_newer(&current, &tag),
        latest: tag.trim_start_matches('v').to_string(),
        url,
        notes: String::new(),
        command: INSTALL_COMMAND.to_string(),
        current,
    })
}

// ---------------------------------------------------------------------------
// Voice models
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct VoiceStatus {
    pub python_env: bool,
    pub model: bool,
}

fn model_path() -> PathBuf {
    home().join(".graphite/speaker-models").join(MODEL_FILE)
}

#[tauri::command]
pub async fn voice_models_status() -> VoiceStatus {
    VoiceStatus {
        python_env: home().join(".graphite/venv/bin/python").exists(),
        model: std::fs::metadata(model_path()).map(|m| m.len() > 10_000_000).unwrap_or(false),
    }
}

async fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    let out = tokio::process::Command::new(cmd)
        .args(args)
        .env_clear()
        .env("HOME", home())
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin:/usr/local/bin")
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("failed").to_string())
    }
}

/// Create the Python helper environment and fetch the voice model (about 110 MB). Takes a few minutes.
#[tauri::command]
pub async fn install_voice_models() -> Result<String, String> {
    let g = home().join(".graphite");
    std::fs::create_dir_all(g.join("speaker-models")).map_err(|e| e.to_string())?;
    if !g.join("venv/bin/python").exists() {
        run("/usr/bin/python3", &["-m", "venv", &g.join("venv").display().to_string()]).await.map_err(|e| {
            format!("Python 3 is needed. In Terminal run: xcode-select --install ({e})")
        })?;
    }
    run(&g.join("venv/bin/pip").display().to_string(), &["install", "-q", "--disable-pip-version-check", "sherpa-onnx", "numpy"])
        .await
        .map_err(|e| format!("Could not install the voice engine: {e}"))?;
    if !voice_models_status().await.model {
        let bytes = reqwest::Client::new()
            .get(MODEL_URL)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|_| "Could not download the voice model.".to_string())?
            .bytes()
            .await
            .map_err(|_| "The voice model download was cut off.".to_string())?;
        let tmp = model_path().with_extension("part");
        std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, model_path()).map_err(|e| e.to_string())?;
    }
    Ok("Voice models are ready. They start with your next meeting.".to_string())
}

// ---------------------------------------------------------------------------
// Diagnostics and folders
// ---------------------------------------------------------------------------

fn sw_vers() -> String {
    std::process::Command::new("/usr/bin/sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// A text report people can paste into a bug report. No meeting content, names or paths of meetings.
#[tauri::command]
pub async fn app_diagnostics(app: AppHandle<Wry>) -> String {
    let claude = crate::chat::find_claude().is_some();
    let v = voice_models_status().await;
    let transcribing = crate::audio::recording_commands::is_recording().await;
    format!(
        "Graphite {}\nmacOS {} ({})\nAccessibility: {}\nScreen Recording: {}\nClaude Code installed: {}\nVoice engine: {} / voice model: {}\nRemember voices: {}\nCurrently transcribing: {}\nCalendar: {}\n\nRecent capture log:\n{}\n",
        app.package_info().version,
        sw_vers(),
        std::env::consts::ARCH,
        crate::capture::perms::accessibility_granted(),
        crate::capture::perms::screen_granted(),
        claude,
        v.python_env,
        v.model,
        remember_voices(),
        transcribing,
        { let c = crate::calendar::calendar_status().await; format!("{} ({})", if c.enabled { "connected" } else { "not connected" }, c.source.unwrap_or_default()) },
        crate::capture::trace::tail(25),
    )
}

/// Open a Graphite folder in Finder. Only these two places.
#[tauri::command]
pub async fn open_graphite_folder(which: String) -> Result<(), String> {
    let path = match which.as_str() {
        "meetings" => dirs::video_dir().unwrap_or_else(|| home().join("Movies")).join("graphite-recordings"),
        "data" => dirs::data_dir().ok_or("no data folder")?.join("com.graphite.ai"),
        _ => return Err("unknown folder".into()),
    };
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    std::process::Command::new("open").arg(path).spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_latest_tag_is_read_from_the_redirect() {
        assert_eq!(tag_from_location("https://github.com/o/r/releases/tag/v0.9.9"), Some("v0.9.9".into()));
        assert_eq!(tag_from_location("/o/r/releases/tag/v1.0.0?x=1"), Some("v1.0.0".into()));
        assert_eq!(tag_from_location("https://github.com/o/r/releases"), None);
        assert_eq!(tag_from_location("https://github.com/o/r/releases/tag/"), None);
    }

    #[test]
    fn versions_compare_by_number_not_text() {
        assert!(is_newer("0.9.7", "0.9.8"));
        assert!(is_newer("0.9.9", "0.9.10"));
        assert!(is_newer("0.9.7", "v1.0.0"));
        assert!(!is_newer("0.9.7", "0.9.7"));
        assert!(!is_newer("0.9.8", "0.9.7"));
        assert!(!is_newer("0.9.7", ""), "no tag means no update");
    }
}
