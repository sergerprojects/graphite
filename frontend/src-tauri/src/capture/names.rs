//! While a recording runs: watch who the call app marks as speaking (Google Meet in
//! Chrome) and keep a timeline in `active_speakers.json` next to the recording. When
//! the meeting is saved, `apply` writes those names onto the transcript lines that
//! only say "Others" or "Speaker N".

use super::ax::{self, CallApp, Limits};
use super::speakers::{assign, collapse, is_placeholder, self_names, voice_names, Line, Sample, Span};
use super::{active, clock, meet, perms, voices};
use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Wry};

const FAST: Duration = Duration::from_millis(500);
const IDLE: Duration = Duration::from_secs(3);
const WRITE_EVERY: Duration = Duration::from_secs(10);

pub const FILE: &str = "active_speakers.json";

// ---------------------------------------------------------------------------
// Live names: while the meeting runs, a voice ("Speaker 2") that keeps lining up with
// one name in the call app gets that name straight away.
// ---------------------------------------------------------------------------

const FRESH: Duration = Duration::from_millis(2500);
const MIN_OBS: u32 = 3;
const MIN_SHARE: f64 = 0.7;

#[derive(Default)]
struct Live {
    at: Option<Instant>,
    speaking: Vec<String>,
    per_voice: HashMap<String, HashMap<String, u32>>,
    /// Times a name was lit while your own microphone was the one talking.
    mine: HashMap<String, u32>,
    /// Times a name was lit while a remote voice was talking.
    remote: HashMap<String, u32>,
}

static LIVE: Mutex<Option<Live>> = Mutex::new(None);

fn with_live<R>(f: impl FnOnce(&mut Live) -> R) -> R {
    let mut g = LIVE.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(Live::default))
}

fn live_reset() {
    *LIVE.lock().unwrap_or_else(|e| e.into_inner()) = Some(Live::default());
}

fn live_publish(speaking: &[String]) {
    with_live(|l| {
        l.at = Some(Instant::now());
        l.speaking = speaking.to_vec();
    });
}

fn fresh_speaking(l: &Live) -> Vec<String> {
    match l.at {
        Some(t) if t.elapsed() <= FRESH => l.speaking.clone(),
        _ => Vec::new(),
    }
}

fn is_me(l: &Live, name: &str) -> bool {
    let mine = l.mine.get(name).copied().unwrap_or(0);
    mine >= MIN_OBS && mine >= l.remote.get(name).copied().unwrap_or(0)
}

/// Your microphone just produced a line: whoever the call app shows speaking right now is you.
pub fn live_note_mine() {
    with_live(|l| {
        for n in fresh_speaking(l) {
            *l.mine.entry(n).or_default() += 1;
        }
    });
}

/// A remote voice ("Speaker 2") just produced a line. Returns the real name once that
/// voice has lined up with one name often enough.
pub fn live_name_for(voice: &str) -> Option<String> {
    with_live(|l| {
        let now = fresh_speaking(l);
        let others: Vec<String> = now.iter().filter(|n| !is_me(l, n)).cloned().collect();
        if others.len() == 1 {
            *l.remote.entry(others[0].clone()).or_default() += 1;
            *l.per_voice.entry(voice.to_string()).or_default().entry(others[0].clone()).or_default() += 1;
        }
        let counts = l.per_voice.get(voice)?;
        let total: u32 = counts.values().sum();
        let (name, n) = counts.iter().max_by_key(|(_, n)| **n)?;
        (total >= MIN_OBS && *n as f64 / total as f64 >= MIN_SHARE && !is_me(l, name)).then(|| name.clone())
    })
}

pub fn start(app: AppHandle<Wry>) {
    let _ = std::thread::Builder::new()
        .name("speaker-names".into())
        .spawn(move || tauri::async_runtime::block_on(run(app)));
}

async fn run(app: AppHandle<Wry>) {
    let mut samples: Vec<Sample> = Vec::new();
    let mut folder: Option<PathBuf> = None;
    let mut last_write = Instant::now();
    let mut dirty = false;
    let mut was_recording = false;
    let mut delay = IDLE;
    let mut tracker = active::MarkerTracker::default();
    let mut roster: Vec<String> = Vec::new();
    loop {
        tokio::time::sleep(delay).await;
        delay = IDLE;
        let recording = crate::audio::recording_commands::is_recording().await;
        if !recording {
            if dirty {
                if let Some(f) = &folder {
                    write(f, &samples, &roster);
                }
                dirty = false;
            }
            was_recording = false;
            continue;
        }
        if !was_recording {
            was_recording = true;
            samples.clear();
            tracker = active::MarkerTracker::default();
            roster.clear();
            live_reset();
            folder = None;
            dirty = false;
        }
        if crate::audio::recording_commands::is_recording_paused().await {
            continue;
        }
        if !super::names_enabled(&app) {
            super::trace::note("names", "off in Settings");
            continue;
        }
        if !perms::accessibility_granted() {
            super::trace::note("names", "Accessibility is not allowed for Graphite, so call names cannot be read");
            continue;
        }
        if folder.is_none() {
            folder = crate::audio::recording_commands::get_meeting_folder_path()
                .await
                .ok()
                .flatten()
                .map(PathBuf::from);
        }
        let Some(dir) = folder.clone() else { continue };
        let Some((speaking, listed)) = read_speaking(&mut tracker) else {
            super::trace::note("names", "no supported call found on screen");
            continue;
        };
        super::trace::note("names", &format!("watching a call: {} listed, {} speaking now", roster.len().max(listed.len()), speaking.len()));
        live_publish(&speaking);
        for n in listed {
            if !roster.contains(&n) {
                roster.push(n);
                dirty = true;
            }
        }
        delay = FAST;
        if let Some(t) = clock::stream_now() {
            samples.push(Sample { t, speaking });
            dirty = true;
        }
        if dirty && last_write.elapsed() >= WRITE_EVERY {
            write(&dir, &samples, &roster);
            last_write = Instant::now();
            dirty = false;
        }
    }
}

/// Who the running call app marks as speaking, or None when no supported call is on screen.
/// (names marked as speaking, everyone listed on the call when the app shows a roster)
pub fn read_speaking(tracker: &mut active::MarkerTracker) -> Option<(Vec<String>, Vec<String>)> {
    // Google Meet in Chrome
    if let Some(pid) = ax::find_pid(CallApp::Chrome) {
        let windows = ax::snapshot_windows(pid, true, &Limits::poll());
        if let Some((w, page)) = windows.iter().find_map(|w| meet::meet_page(w).map(|p| (w, p))) {
            let tiles = active::meet_tiles(page);
            let mut who = tracker.observe(&tiles);
            if who.is_empty() {
                who = meet::speaking(w); // label-style marker, if Meet ever adds one
            }
            return Some((who, tiles.into_iter().map(|t| t.name).collect()));
        }
    }
    // Zoom
    if let Some(pid) = ax::find_pid(CallApp::Zoom) {
        let windows = ax::snapshot_windows(pid, false, &Limits::poll());
        if !windows.is_empty() {
            let mut who: Vec<String> = Vec::new();
            for w in &windows {
                for n in active::zoom_speaking(w) {
                    if !who.contains(&n) {
                        who.push(n);
                    }
                }
            }
            return Some((who, Vec::new()));
        }
    }
    // Microsoft Teams (WebView2 needs the enhanced-UI switch)
    if let Some(pid) = ax::find_pid(CallApp::Teams) {
        let windows = ax::snapshot_windows(pid, true, &Limits::poll());
        if !windows.is_empty() {
            let mut who: Vec<String> = Vec::new();
            for w in &windows {
                for n in active::teams_speaking(w) {
                    if !who.contains(&n) {
                        who.push(n);
                    }
                }
            }
            return Some((who, Vec::new()));
        }
    }
    None
}

fn write(dir: &Path, samples: &[Sample], roster: &[String]) {
    if !roster.is_empty() {
        if let Ok(json) = serde_json::to_vec(&serde_json::json!({ "names": roster })) {
            let _ = std::fs::write(dir.join(super::voices::ROSTER_FILE), json);
        }
    }
    let spans = collapse(samples);
    if spans.is_empty() {
        return;
    }
    match serde_json::to_vec_pretty(&spans) {
        Ok(json) => {
            let tmp = dir.join(".active_speakers.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, dir.join(FILE));
            }
        }
        Err(e) => log::warn!("speaker names: {e}"),
    }
}

pub fn read_spans(dir: &Path) -> Vec<Span> {
    std::fs::read_to_string(dir.join(FILE))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

const MERGE_SIM: f64 = 0.55;
const NAME_MIN_SECONDS: f64 = 6.0;
const NAME_RATIO: f64 = 1.8;
const LAG: f64 = 0.3;
const REMEMBER_MIN_SECONDS: f64 = 30.0;

/// Re-cluster this meeting's voices, relabel placeholder lines by voice, name the voices
/// the call app confirms, and remember them. Returns lines changed.
async fn voice_pass(
    pool: &SqlitePool,
    meeting_id: &str,
    folder: &Path,
    j: &voices::Journal,
    lines: &mut Vec<Line>,
    spans: &[Span],
    me: &[String],
) -> usize {
    let roster: Vec<String> = voices::read_roster(folder).into_iter().filter(|n| !me.iter().any(|m| m.eq_ignore_ascii_case(n))).collect();
    let cap = (!roster.is_empty()).then_some(roster.len());
    let embs: Vec<Vec<f32>> = j.entries.iter().map(|e| e.emb.clone()).collect();
    let ids = voices::recluster(&embs, cap, MERGE_SIM);
    let k = ids.iter().copied().max().map(|m| m + 1).unwrap_or(0);
    let placeholder: Vec<Line> = lines.iter().filter(|l| is_placeholder(&l.speaker)).cloned().collect();
    let owner = voices::lines_to_clusters(&placeholder, &j.entries, &ids, 4.0);
    let label = |c: usize| format!("Speaker {}", c + 1);

    let mut changed = 0;
    for l in lines.iter_mut() {
        if let Some(c) = owner.get(&l.id) {
            let new = label(*c);
            if l.speaker != new {
                let ok = sqlx::query("UPDATE transcripts SET speaker = ? WHERE id = ? AND meeting_id = ?")
                    .bind(&new)
                    .bind(&l.id)
                    .bind(meeting_id)
                    .execute(pool)
                    .await
                    .is_ok();
                if ok {
                    l.speaker = new;
                    changed += 1;
                }
            }
        }
    }

    // Name voices: who was lit while each voice talked (best one-to-one pairing).
    let mut cluster_times: Vec<Vec<(f64, f64)>> = vec![Vec::new(); k];
    for (e, c) in j.entries.iter().zip(&ids) {
        cluster_times[*c].push((e.t0, e.t1));
    }
    let mut names: Vec<String> = spans.iter().map(|s| s.name.clone()).collect();
    names.sort();
    names.dedup();
    names.retain(|n| !me.iter().any(|m| m.eq_ignore_ascii_case(n)));
    if names.is_empty() || k == 0 {
        return changed;
    }
    let scores = voices::cooccurrence(&cluster_times, spans, &names, LAG);
    let mut profiles = voices::Profiles::load(&profile_path());
    let mut learned = false;
    for nm in voices::name_voices(&scores, &names, NAME_MIN_SECONDS, NAME_RATIO) {
        let from = label(nm.cluster);
        let n = sqlx::query("UPDATE transcripts SET speaker = ? WHERE meeting_id = ? AND speaker = ?")
            .bind(&nm.name)
            .bind(meeting_id)
            .bind(&from)
            .execute(pool)
            .await
            .map(|r| r.rows_affected() as usize)
            .unwrap_or(0);
        changed += n;
        for l in lines.iter_mut().filter(|l| l.speaker == from) {
            l.speaker = nm.name.clone();
        }
        let talk: f64 = cluster_times[nm.cluster].iter().map(|(a, b)| b - a).sum();
        if talk >= REMEMBER_MIN_SECONDS && crate::beta::remember_voices() {
            let mine: Vec<&Vec<f32>> = j.entries.iter().zip(&ids).filter(|(_, c)| **c == nm.cluster).map(|(e, _)| &e.emb).collect();
            profiles.remember(&j.model, &nm.name, &voices::centroid(&mine));
            learned = true;
        }
    }
    if learned {
        profiles.save(&profile_path());
    }
    changed
}

fn profile_path() -> std::path::PathBuf {
    dirs::home_dir().unwrap_or_default().join(".graphite").join("voices.json")
}

/// Put the call app's names on the saved transcript. Never overwrites "You" or a name
/// a person already chose. Returns how many lines were renamed.
pub async fn apply(pool: &SqlitePool, meeting_id: &str, folder: &Path) -> usize {
    let spans = read_spans(folder);
    let journal = voices::read_journal(folder).filter(|j| !j.entries.is_empty());
    if spans.is_empty() && journal.is_none() {
        return 0;
    }
    let rows = match sqlx::query(
        "SELECT id, audio_start_time, audio_end_time, speaker FROM transcripts WHERE meeting_id = ? AND audio_start_time IS NOT NULL",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            log::warn!("speaker names: {e}");
            return 0;
        }
    };
    let lines: Vec<Line> = rows
        .iter()
        .map(|r| {
            let start: f64 = r.get("audio_start_time");
            let end: Option<f64> = r.get("audio_end_time");
            Line {
                id: r.get::<String, _>("id"),
                start,
                end: end.unwrap_or(start + 2.0),
                speaker: r.get::<Option<String>, _>("speaker").unwrap_or_default(),
            }
        })
        .collect();
    let me = self_names(&lines, &spans);
    let mut lines = lines;
    let mut changed = 0;

    // The whole-meeting voice pass: re-cluster the journal, pair voices with names.
    if let Some(j) = &journal {
        changed += voice_pass(pool, meeting_id, folder, j, &mut lines, &spans, &me).await;
    }
    let named = assign(&lines, &spans, &me);
    for (id, name) in &named {
        if sqlx::query("UPDATE transcripts SET speaker = ? WHERE id = ? AND meeting_id = ?")
            .bind(name)
            .bind(id)
            .bind(meeting_id)
            .execute(pool)
            .await
            .is_ok()
        {
            changed += 1;
        }
    }
    // A voice whose lines mostly agree on one name gets that name on its other lines too.
    for (label, name) in voice_names(&lines, &named, 0.7) {
        let _ = sqlx::query("UPDATE transcripts SET speaker = ? WHERE meeting_id = ? AND speaker = ?")
            .bind(&name)
            .bind(meeting_id)
            .bind(&label)
            .execute(pool)
            .await;
    }
    changed
}

#[cfg(test)]
mod live_tests {
    use super::*;

    // One test: the live state is shared, so keep the steps in order.
    #[test]
    fn a_voice_gets_a_name_after_lining_up_and_you_are_never_given_away() {
        live_reset();
        // Your mic runs while "Alex Morgan" is lit: that is you.
        for _ in 0..3 {
            live_publish(&["Alex Morgan".to_string()]);
            live_note_mine();
        }
        // A remote voice lines up with Noah three times.
        assert_eq!(live_name_for("Speaker 1"), None);
        live_publish(&["Ana Park".to_string()]);
        assert_eq!(live_name_for("Speaker 1"), None);
        assert_eq!(live_name_for("Speaker 1"), None);
        assert_eq!(live_name_for("Speaker 1"), Some("Ana Park".to_string()));
        // While you are lit, a remote line never learns your name.
        live_publish(&["Alex Morgan".to_string()]);
        for _ in 0..5 {
            assert_ne!(live_name_for("Speaker 2"), Some("Alex Morgan".to_string()));
        }
    }
}

#[cfg(test)]
mod voice_pass_tests {
    use super::*;
    use crate::capture::speakers::Span;

    fn unit8(i: usize, j: f32) -> Vec<f32> {
        let mut v = vec![0.0f32; 8];
        v[i] = 1.0;
        v[(i + 1) % 8] = j;
        let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.iter().map(|x| x / n).collect()
    }

    #[tokio::test]
    async fn the_whole_pass_fixes_a_split_voice_and_names_it() {
        let dir = std::env::temp_dir().join(format!("graphite-pass-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE transcripts (id TEXT PRIMARY KEY, meeting_id TEXT, transcript TEXT, timestamp TEXT, audio_start_time REAL, audio_end_time REAL, duration REAL, speaker TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        // Noah talks 0-40 s (the live labeller split him into Speaker 1 and Speaker 2),
        // Lena talks 40-80 s (Speaker 3), you speak 80-90 s.
        let rows = [
            ("a", 0.0, 10.0, "Speaker 1"),
            ("b", 12.0, 20.0, "Speaker 2"),
            ("c", 22.0, 38.0, "Speaker 1"),
            ("d", 42.0, 60.0, "Speaker 3"),
            ("e", 62.0, 78.0, "Speaker 3"),
            ("f", 80.0, 90.0, "You"),
        ];
        for (id, a, b, who) in rows {
            sqlx::query("INSERT INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time, speaker) VALUES (?, 'm', 'x', 't', ?, ?, ?)")
                .bind(id).bind(a).bind(b).bind(who).execute(&pool).await.unwrap();
        }
        let e = |a: f64, b: f64, label: &str, v: Vec<f32>| voices::Entry { t0: a, t1: b, label: label.into(), emb: v };
        let journal = voices::Journal {
            model: "test-model".into(),
            entries: vec![
                e(0.0, 10.0, "Speaker 1", unit8(0, 0.05)),
                e(12.0, 20.0, "Speaker 2", unit8(0, 0.09)),
                e(22.0, 38.0, "Speaker 1", unit8(0, 0.07)),
                e(42.0, 60.0, "Speaker 3", unit8(4, 0.05)),
                e(62.0, 78.0, "Speaker 3", unit8(4, 0.08)),
            ],
        };
        voices::write_journal(&dir, &journal);
        std::fs::write(dir.join(voices::ROSTER_FILE), r#"{"names":["Ana Park","Cara Diaz","Alex Morgan"]}"#).unwrap();
        let spans = vec![
            Span { start: 0.0, end: 38.0, name: "Ana Park".into() },
            Span { start: 40.0, end: 78.0, name: "Cara Diaz".into() },
            Span { start: 80.0, end: 90.0, name: "Alex Morgan".into() },
        ];
        std::fs::write(dir.join(FILE), serde_json::to_vec(&spans).unwrap()).unwrap();

        // keep the test from touching the real saved voices
        std::env::set_var("HOME", &dir);
        let changed = apply(&pool, "m", &dir).await;
        assert!(changed > 0);

        let who: HashMap<String, String> = sqlx::query("SELECT id, speaker FROM transcripts")
            .fetch_all(&pool)
            .await
            .unwrap()
            .iter()
            .map(|r| (r.get::<String, _>("id"), r.get::<String, _>("speaker")))
            .collect();
        assert_eq!(who["a"], "Ana Park");
        assert_eq!(who["b"], "Ana Park", "the split voice is rejoined");
        assert_eq!(who["c"], "Ana Park");
        assert_eq!(who["d"], "Cara Diaz");
        assert_eq!(who["e"], "Cara Diaz");
        assert_eq!(who["f"], "You", "your own lines are left alone");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Manual check against a live call (needs Accessibility for the terminal):
/// `cargo test --lib live_meet_probe -- --ignored --nocapture`
#[cfg(test)]
mod live_probe {
    use super::*;

    #[test]
    #[ignore]
    fn live_meet_probe() {
        let pid = ax::find_pid(CallApp::Chrome);
        println!("chrome pid: {pid:?}");
        let Some(pid) = pid else { return };
        let t = Instant::now();
        let windows = ax::snapshot_windows(pid, true, &Limits::poll());
        println!("windows: {} in {:?}, nodes: {:?}", windows.len(), t.elapsed(), windows.iter().map(|w| w.count()).collect::<Vec<_>>());
        for w in &windows {
            println!("  window role={} title={:?} meet_page={}", w.role, w.title, meet::meet_page(w).is_some());
        }
        for w in &windows {
            if let Some(page) = meet::meet_page(w) {
                println!("  share: {:?}", meet::detect(w));
                let tiles = active::meet_tiles(page);
                println!("  tiles: {}", tiles.len());
                for t in &tiles {
                    println!("    {:30} classes={} v5h6Xc={} Oaajhc={}", t.name, t.classes.len(), t.classes.contains("v5h6Xc"), t.classes.contains("Oaajhc"));
                }
            }
        }
        let mut tracker = active::MarkerTracker::default();
        for round in 0..6 {
            if let Some((who, listed)) = read_speaking(&mut tracker) {
                println!("round {round}: speaking={who:?} roster={}", listed.len());
            } else {
                println!("round {round}: no call found");
            }
            std::thread::sleep(Duration::from_millis(700));
        }
    }
}
