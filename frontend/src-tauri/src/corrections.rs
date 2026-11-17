//! Fixing a word the transcriber got wrong, remembering the person's own vocabulary
//! (company and product names), and keeping the notes from before an AI rewrite so
//! the rewrite can be undone.

use regex::{NoExpand, Regex};
use serde::Serialize;
use serde_json::Value;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

use crate::state::AppState;

const VOCAB_KEY: &str = "vocabulary";
const MAX_TERMS: usize = 200;
const MAX_TERM_LEN: usize = 60;
/// The only summary fields that hold the person's words. Everything else (block types, ids,
/// styles) is structure and is never touched.
const TEXT_KEYS: [&str; 6] = ["text", "markdown", "content", "title", "MeetingName", "summary"];

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Trim, collapse spaces, drop empty / over-long / odd entries and duplicates (ignoring case).
pub fn clean_terms(terms: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for t in terms {
        let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
        if t.is_empty() || t.chars().count() > MAX_TERM_LEN || t.chars().any(|c| c.is_control() || c == '"' || c == '\\') {
            continue;
        }
        if seen.insert(t.to_lowercase()) {
            out.push(t);
        }
        if out.len() >= MAX_TERMS {
            break;
        }
    }
    out
}

/// The sentence added to a summary request so saved names are spelled the way the person spells them.
pub fn vocabulary_prompt(terms: &[String]) -> String {
    if terms.is_empty() {
        return String::new();
    }
    format!(
        "The person uses these names and terms. The transcript may have them spelled wrong; always write them exactly like this: {}.",
        terms.join(", ")
    )
}

/// Replace every whole-word, case-insensitive match of `from` with `to`. Returns the new text and the count.
pub fn replace_term(text: &str, from: &str, to: &str) -> (String, usize) {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let (first, last) = (from.chars().next(), from.chars().last());
    let (Some(first), Some(last)) = (first, last) else { return (text.to_string(), 0) };
    let pattern = format!(
        "(?i){}{}{}",
        if is_word(first) { r"\b" } else { "" },
        regex::escape(from),
        if is_word(last) { r"\b" } else { "" }
    );
    let Ok(re) = Regex::new(&pattern) else { return (text.to_string(), 0) };
    let n = re.find_iter(text).count();
    if n == 0 {
        return (text.to_string(), 0);
    }
    (re.replace_all(text, NoExpand(to)).into_owned(), n)
}

fn replace_in_value(v: &mut Value, from: &str, to: &str) -> usize {
    match v {
        Value::Object(map) => {
            let mut n = 0;
            for (k, child) in map.iter_mut() {
                if let (true, Value::String(s)) = (TEXT_KEYS.contains(&k.as_str()), &*child) {
                    let (new, c) = replace_term(s, from, to);
                    if c > 0 {
                        *child = Value::String(new);
                        n += c;
                    }
                } else {
                    n += replace_in_value(child, from, to);
                }
            }
            n
        }
        Value::Array(items) => items.iter_mut().map(|i| replace_in_value(i, from, to)).sum(),
        _ => 0,
    }
}

fn check_pair(from: &str, to: &str) -> Result<(String, String), String> {
    let ok = |s: &str| !s.is_empty() && s.chars().count() <= MAX_TERM_LEN && !s.chars().any(|c| c.is_control() || c == '"' || c == '\\');
    let (from, to) = (from.trim(), to.trim());
    if !ok(from) || !ok(to) {
        return Err("Use short words without quotes or backslashes.".into());
    }
    if from == to {
        return Err("Those are the same.".into());
    }
    Ok((from.to_string(), to.to_string()))
}

// ---------------------------------------------------------------------------
// Vocabulary
// ---------------------------------------------------------------------------

pub fn vocabulary<R: Runtime>(app: &AppHandle<R>) -> Vec<String> {
    app.store(crate::calendar::ical::STORE_FILE)
        .ok()
        .and_then(|st| st.get(VOCAB_KEY))
        .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
        .map(clean_terms)
        .unwrap_or_default()
}

#[tauri::command]
pub async fn get_vocabulary<R: Runtime>(app: AppHandle<R>) -> Vec<String> {
    vocabulary(&app)
}

#[tauri::command]
pub async fn set_vocabulary<R: Runtime>(app: AppHandle<R>, terms: Vec<String>) -> Result<Vec<String>, String> {
    let terms = clean_terms(terms);
    let store = app.store(crate::calendar::ical::STORE_FILE).map_err(|e| e.to_string())?;
    store.set(VOCAB_KEY, serde_json::json!(terms));
    store.save().map_err(|e| e.to_string())?;
    Ok(terms)
}

// ---------------------------------------------------------------------------
// Fix a word everywhere
// ---------------------------------------------------------------------------

#[derive(Serialize, Default)]
pub struct FixResult {
    pub transcript: usize,
    pub summary: usize,
    pub title: usize,
}

async fn fix_in_db(pool: &SqlitePool, meeting_id: &str, from: &str, to: &str) -> Result<FixResult, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut out = FixResult::default();

    let rows: Vec<(String, String)> = sqlx::query_as("SELECT id, transcript FROM transcripts WHERE meeting_id = ?")
        .bind(meeting_id)
        .fetch_all(&mut *tx)
        .await?;
    for (id, text) in rows {
        let (new, n) = replace_term(&text, from, to);
        if n > 0 {
            sqlx::query("UPDATE transcripts SET transcript = ? WHERE id = ?").bind(new).bind(id).execute(&mut *tx).await?;
            out.transcript += n;
        }
    }

    let result: Option<(Option<String>,)> = sqlx::query_as("SELECT result FROM summary_processes WHERE meeting_id = ?")
        .bind(meeting_id)
        .fetch_optional(&mut *tx)
        .await?;
    if let Some((Some(raw),)) = result {
        if let Ok(mut value) = serde_json::from_str::<Value>(&raw) {
            let n = replace_in_value(&mut value, from, to);
            if n > 0 {
                sqlx::query("UPDATE summary_processes SET result = ? WHERE meeting_id = ?")
                    .bind(value.to_string())
                    .bind(meeting_id)
                    .execute(&mut *tx)
                    .await?;
                out.summary = n;
            }
        }
    }

    let title: Option<(String,)> = sqlx::query_as("SELECT title FROM meetings WHERE id = ?").bind(meeting_id).fetch_optional(&mut *tx).await?;
    if let Some((title,)) = title {
        let (new, n) = replace_term(&title, from, to);
        if n > 0 {
            sqlx::query("UPDATE meetings SET title = ? WHERE id = ?").bind(new).bind(meeting_id).execute(&mut *tx).await?;
            out.title = n;
        }
    }

    tx.commit().await?;
    Ok(out)
}

/// Replace a misheard word in the transcript, the notes and the title of one meeting.
/// The notes as they were are kept first, so "Undo" can bring them back.
#[tauri::command]
pub async fn fix_word_everywhere(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    from: String,
    to: String,
) -> Result<FixResult, String> {
    let (from, to) = check_pair(&from, &to)?;
    let pool = state.db_manager.pool();
    backup_current(pool, &meeting_id).await?;
    fix_in_db(pool, &meeting_id, &from, &to).await.map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Undo for AI rewrites
// ---------------------------------------------------------------------------

fn backup_path(meeting_id: &str) -> Result<PathBuf, String> {
    let safe: String = meeting_id.chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
    if safe.is_empty() {
        return Err("bad meeting id".into());
    }
    let dir = dirs::data_dir().ok_or("no data folder")?.join("com.graphite.ai").join("summary-backups");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(format!("{safe}.json")))
}

async fn current_result(pool: &SqlitePool, meeting_id: &str) -> Result<Option<String>, String> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT result FROM summary_processes WHERE meeting_id = ?")
        .bind(meeting_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.and_then(|r| r.0).filter(|s| !s.trim().is_empty()))
}

async fn backup_current(pool: &SqlitePool, meeting_id: &str) -> Result<bool, String> {
    let Some(result) = current_result(pool, meeting_id).await? else { return Ok(false) };
    std::fs::write(backup_path(meeting_id)?, result).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Keep the notes as they are now. Call before an AI rewrite.
#[tauri::command]
pub async fn backup_summary(state: tauri::State<'_, AppState>, meeting_id: String) -> Result<bool, String> {
    backup_current(state.db_manager.pool(), &meeting_id).await
}

#[tauri::command]
pub async fn has_summary_backup(meeting_id: String) -> bool {
    backup_path(&meeting_id).map(|p| p.exists()).unwrap_or(false)
}

/// Swap the notes with the kept copy. Running it twice goes back and forth, so Undo can be undone.
#[tauri::command]
pub async fn restore_summary_backup(state: tauri::State<'_, AppState>, meeting_id: String) -> Result<bool, String> {
    let pool = state.db_manager.pool();
    let path = backup_path(&meeting_id)?;
    let Ok(kept) = std::fs::read_to_string(&path) else { return Ok(false) };
    let Ok(value) = serde_json::from_str::<Value>(&kept) else { return Ok(false) };
    let now = current_result(pool, &meeting_id).await?;
    let saved = crate::database::repositories::summary::SummaryProcessesRepository::update_meeting_summary(pool, &meeting_id, &value)
        .await
        .map_err(|e| e.to_string())?;
    if saved {
        match now {
            Some(now) => std::fs::write(&path, now).map_err(|e| e.to_string())?,
            None => {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn whole_words_only_and_ignoring_case() {
        assert_eq!(replace_term("JetMine and jetmine, not Jetmines.", "Jetmine", "Jumpmind"), ("Jumpmind and Jumpmind, not Jetmines.".into(), 2));
        assert_eq!(replace_term("nothing here", "Jetmine", "Jumpmind").1, 0);
    }

    #[test]
    fn terms_with_symbols_and_dollars_are_literal() {
        assert_eq!(replace_term("use C++ today", "C++", "Rust").0, "use Rust today");
        assert_eq!(replace_term("a Jet", "jet", "$1 x").0, "a $1 x");
    }

    #[test]
    fn only_the_words_in_the_notes_change_never_the_structure() {
        let mut v = json!({
            "markdown": "## Summary\nJetmine left.",
            "summary_json": [{"type": "paragraph", "content": [{"type": "text", "text": "Jetmine left."}]}],
        });
        assert_eq!(replace_in_value(&mut v, "Jetmine", "Jumpmind"), 2);
        assert_eq!(v["summary_json"][0]["content"][0]["text"], "Jumpmind left.");
        assert_eq!(v["summary_json"][0]["type"], "paragraph");

        let mut v = json!({"summary_json": [{"type": "paragraph"}]});
        assert_eq!(replace_in_value(&mut v, "paragraph", "x"), 0, "block types are structure");
    }

    #[test]
    fn vocabulary_is_cleaned() {
        let got = clean_terms(vec!["  Jumpmind ".into(), "jumpmind".into(), "".into(), "a\"b".into(), "Sales  force".into()]);
        assert_eq!(got, vec!["Jumpmind".to_string(), "Sales force".to_string()]);
        assert!(vocabulary_prompt(&[]).is_empty());
        assert!(vocabulary_prompt(&["Jumpmind".into()]).contains("Jumpmind"));
    }

    #[test]
    fn a_fix_needs_two_different_short_words() {
        assert!(check_pair("", "x").is_err());
        assert!(check_pair("a", "a").is_err());
        assert!(check_pair("a\"", "b").is_err());
        assert_eq!(check_pair(" Jetmine ", "Jumpmind").unwrap(), ("Jetmine".into(), "Jumpmind".into()));
    }
}
