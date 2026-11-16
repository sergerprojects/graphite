//! Live "who is speaking" for the call side ("Others"): tells voices apart as people talk.
//!
//! A small Python helper (`tools/live_speakers.py`, embedded in this binary) runs next to
//! the app for the length of a recording. Each transcribed stretch of call audio is sent to
//! it in memory and it answers with a speaker number; the same voice keeps its number all
//! meeting. Nothing is written to disk and nothing leaves the Mac.
//!
//! Optional: it needs Python with `sherpa-onnx` and the voice model (see
//! scripts/setup-speakers.sh). If either is missing the feature stays off and call audio
//! is simply labelled "Others", as before.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

const SCRIPT: &str = include_str!("../../../../tools/live_speakers.py");
const READY_TIMEOUT: Duration = Duration::from_secs(40);
const REPLY_TIMEOUT: Duration = Duration::from_secs(4);
/// Fewer samples than this (at 16 kHz about 0.6 s) is not worth sending.
const MIN_SAMPLES: usize = 9_600;

static ENABLED: AtomicBool = AtomicBool::new(false);
static HELPER: Mutex<Option<Helper>> = Mutex::const_new(None);
/// Voice fingerprints of this meeting so far (no audio), written next to it when it stops.
static JOURNAL: std::sync::Mutex<Option<crate::capture::voices::Journal>> = std::sync::Mutex::new(None);

struct Helper {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

fn graphite_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".graphite"))
}

fn python() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("GRAPHITE_PYTHON").map(PathBuf::from).filter(|p| p.exists()) {
        return Some(p);
    }
    let p = crate::platform::venv_python(&graphite_dir()?.join("venv"));
    p.exists().then_some(p)
}

/// Start the helper for a new recording. Quietly leaves the feature off if it cannot run.
pub async fn start() {
    stop().await;
    // Calendar knows how many people are on the call. A 1:1 has one remote voice, so
    // there is nothing to tell apart (the UI shows the other person's name); a group
    // call can never have more remote voices than other attendees.
    let others = crate::calendar::current_other_count().await;
    if others == Some(1) {
        log::info!("Live speakers: off (1:1, one remote voice)");
        return;
    }
    let (Some(py), Some(dir)) = (python(), graphite_dir()) else {
        log::info!("Live speakers: off (no ~/.graphite/venv; run scripts/setup-speakers.sh to enable)");
        return;
    };
    let script = dir.join("live_speakers.py");
    if std::fs::write(&script, SCRIPT).is_err() {
        return;
    }
    let spawned = Command::new(&py)
        .arg(&script)
        .env_clear()
        .envs(crate::platform::base_env())
        // Saved voices are only used (and only saved) when the person switched that on.
        .env("GRAPHITE_VOICE_PROFILES", if crate::beta::remember_voices() { dirs::home_dir().unwrap_or_default().join(".graphite/voices.json") } else { std::path::PathBuf::from("/nonexistent") })
        .env("GRAPHITE_MAX_SPEAKERS", others.filter(|n| *n >= 2).map(|n| n.to_string()).unwrap_or_default())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn();
    let Ok(mut child) = spawned else { return };
    let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else { return };
    let mut stdout = BufReader::new(stdout);
    let mut line = String::new();
    let ready = tokio::time::timeout(READY_TIMEOUT, stdout.read_line(&mut line)).await;
    if !matches!(ready, Ok(Ok(n)) if n > 0) || !line.contains("\"ready\"") {
        log::info!("Live speakers: helper did not start ({})", line.trim());
        let _ = child.kill().await;
        return;
    }
    let model = serde_json::from_str::<serde_json::Value>(line.trim())
        .ok()
        .and_then(|v| v.get("model").and_then(|m| m.as_str().map(String::from)))
        .unwrap_or_default();
    *JOURNAL.lock().unwrap_or_else(|e| e.into_inner()) =
        Some(crate::capture::voices::Journal { model, entries: Vec::new() });
    *HELPER.lock().await = Some(Helper { child, stdin, stdout });
    ENABLED.store(true, Ordering::Relaxed);
    log::info!("Live speakers: on");
}

pub async fn stop() {
    ENABLED.store(false, Ordering::Relaxed);
    // Keep the meeting's voice journal next to the transcript for the end-of-meeting pass.
    let journal = JOURNAL.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(j) = journal.filter(|j| !j.entries.is_empty()) {
        if let Ok(Some(folder)) = crate::audio::recording_commands::get_meeting_folder_path().await {
            crate::capture::voices::write_journal(std::path::Path::new(&folder), &j);
        }
    }
    if let Some(mut h) = HELPER.lock().await.take() {
        drop(h.stdin);
        let _ = h.child.kill().await;
    }
}

/// "Speaker N" for this stretch of call audio, or None when unsure / off.
pub async fn label(samples: &[f32], sample_rate: u32) -> Option<String> {
    if !enabled() || samples.len() < MIN_SAMPLES * sample_rate as usize / 16_000 {
        return None;
    }
    let mut guard = HELPER.lock().await;
    let h = guard.as_mut()?;
    let mut frame = Vec::with_capacity(8 + samples.len() * 4);
    frame.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    frame.extend_from_slice(&sample_rate.to_le_bytes());
    for s in samples {
        frame.extend_from_slice(&s.to_le_bytes());
    }
    let result = tokio::time::timeout(REPLY_TIMEOUT, async {
        h.stdin.write_all(&frame).await.ok()?;
        h.stdin.flush().await.ok()?;
        let mut line = String::new();
        h.stdout.read_line(&mut line).await.ok()?;
        parse_reply(&line)
    })
    .await;
    match result {
        Ok(Some(r)) => {
            let n = r.speaker?;
            let label = format!("Speaker {}", n + 1);
            if let Some(emb) = r.emb {
                if let Some(t1) = crate::capture::clock::stream_now() {
                    let dur = samples.len() as f64 / sample_rate as f64;
                    if let Some(j) = JOURNAL.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                        j.entries.push(crate::capture::voices::Entry { t0: (t1 - dur).max(0.0), t1, label: label.clone(), emb });
                    }
                }
            }
            // A voice the helper already knows from earlier meetings comes back by name.
            Some(r.name.unwrap_or(label))
        }
        _ => {
            // Helper died or stalled: stop using it for this recording.
            ENABLED.store(false, Ordering::Relaxed);
            None
        }
    }
}

/// What the helper says about one stretch of audio.
#[derive(Debug, PartialEq)]
struct Reply {
    speaker: Option<usize>,
    name: Option<String>,
    emb: Option<Vec<f32>>,
}

/// `{"speaker": 2, "name": "Maya", "emb": [...]}`; speaker -1 means "can't tell"; garbage -> None.
fn parse_reply(line: &str) -> Option<Reply> {
    let v: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    let n = v.get("speaker")?.as_i64()?;
    Some(Reply {
        speaker: (n >= 0).then_some(n as usize),
        name: v.get("name").and_then(|x| x.as_str()).map(String::from),
        emb: v
            .get("emb")
            .and_then(|e| e.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_helper_replies() {
        let r = parse_reply("{\"speaker\": 2, \"sim\": 0.9}\n").unwrap();
        assert_eq!((r.speaker, r.name, r.emb), (Some(2), None, None));
        assert_eq!(parse_reply("{\"speaker\": -1, \"sim\": 0.1}").unwrap().speaker, None);
        assert_eq!(parse_reply("not json"), None);
        let r = parse_reply("{\"speaker\": 0, \"name\": \"Maya\", \"emb\": [0.5, -0.25]}").unwrap();
        assert_eq!((r.name.as_deref(), r.emb), (Some("Maya"), Some(vec![0.5, -0.25])));
    }
}
