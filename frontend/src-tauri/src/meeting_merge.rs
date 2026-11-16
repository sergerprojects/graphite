//! Join a second recording onto an earlier meeting: "continue recording" a meeting, or
//! stitching together a call that was cut off and picked up again. The transcript of the
//! second recording is appended to the first (its times shifted to follow on), notes are
//! joined, and the second meeting entry is removed. Files on disk are left alone.

use sqlx::Row;
use tauri::State;

use crate::state::AppState;

/// Seconds of gap put between the end of the first recording and the start of the next.
const GAP: f64 = 1.0;

/// How far to shift the second recording's times so they follow the first.
pub fn offset_after(last_end: Option<f64>) -> f64 {
    last_end.map(|e| e.max(0.0) + GAP).unwrap_or(0.0)
}

/// Notes of both recordings, first then second, skipping empty ones.
pub fn join_notes(first: &str, second: &str) -> String {
    match (first.trim(), second.trim()) {
        ("", s) => s.to_string(),
        (f, "") => f.to_string(),
        (f, s) => format!("{f}\n\n{s}"),
    }
}

#[tauri::command]
pub async fn meetings_merge(
    state: State<'_, AppState>,
    target_id: String,
    source_id: String,
) -> Result<usize, String> {
    if target_id == source_id {
        return Err("A meeting can't be joined to itself.".to_string());
    }
    let pool = state.db_manager.pool();
    let notes = |id: String| async move {
        sqlx::query("SELECT notes_markdown FROM meeting_notes WHERE meeting_id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .and_then(|r| r.try_get::<Option<String>, _>("notes_markdown").ok().flatten())
            .unwrap_or_default()
    };
    let (first, second) = (notes(target_id.clone()).await, notes(source_id.clone()).await);
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    for id in [&target_id, &source_id] {
        let exists = sqlx::query("SELECT 1 FROM meetings WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        if exists.is_none() {
            return Err("That meeting no longer exists.".to_string());
        }
    }

    let last_end: Option<f64> = sqlx::query("SELECT MAX(audio_end_time) AS m FROM transcripts WHERE meeting_id = ?")
        .bind(&target_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?
        .try_get("m")
        .ok();
    let offset = offset_after(last_end);

    let moved = sqlx::query(
        "UPDATE transcripts SET meeting_id = ?, \
         audio_start_time = audio_start_time + ?, audio_end_time = audio_end_time + ? \
         WHERE meeting_id = ?",
    )
    .bind(&target_id)
    .bind(offset)
    .bind(offset)
    .bind(&source_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?
    .rows_affected() as usize;

    let joined = join_notes(&first, &second);
    if !joined.is_empty() && joined != first.trim() {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO meeting_notes (meeting_id, notes_markdown, created_at, updated_at) VALUES (?, ?, ?, ?) \
             ON CONFLICT(meeting_id) DO UPDATE SET notes_markdown = excluded.notes_markdown, updated_at = excluded.updated_at",
        )
        .bind(&target_id)
        .bind(&joined)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    sqlx::query("DELETE FROM meetings WHERE id = ?")
        .bind(&source_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("UPDATE meetings SET updated_at = ? WHERE id = ?")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&target_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    log::info!("Merged meeting {source_id} into {target_id} ({moved} lines, +{offset:.1}s)");
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_recording_follows_the_first() {
        assert_eq!(offset_after(Some(167.5)), 168.5);
        assert_eq!(offset_after(None), 0.0);
        assert_eq!(offset_after(Some(-3.0)), 1.0);
    }

    #[test]
    fn notes_are_joined_without_blanks() {
        assert_eq!(join_notes("a", "b"), "a\n\nb");
        assert_eq!(join_notes("", "b"), "b");
        assert_eq!(join_notes("a", "  "), "a");
        assert_eq!(join_notes(" ", ""), "");
    }
}
