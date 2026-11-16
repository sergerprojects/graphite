//! Folders for organising meetings: create, rename, delete, and file a meeting in one.
//! A meeting is in at most one folder. Deleting a folder keeps its meetings.

use serde::Serialize;
use sqlx::Row;
use tauri::State;

use crate::state::AppState;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Folder {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct MeetingFolder {
    pub meeting_id: String,
    pub folder_id: String,
}

/// Trimmed name, or an error message fit to show the user.
pub fn clean_name(name: &str) -> Result<String, String> {
    let n = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if n.is_empty() {
        return Err("Give the folder a name.".to_string());
    }
    if n.chars().count() > 60 {
        return Err("Folder names can be up to 60 characters.".to_string());
    }
    Ok(n)
}

#[tauri::command]
pub async fn folders_list(state: State<'_, AppState>) -> Result<Vec<Folder>, String> {
    let rows = sqlx::query("SELECT id, name FROM folders ORDER BY lower(name)")
        .fetch_all(state.db_manager.pool())
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|r| Folder { id: r.get("id"), name: r.get("name") })
        .collect())
}

#[tauri::command]
pub async fn folder_create(state: State<'_, AppState>, name: String) -> Result<Folder, String> {
    let name = clean_name(&name)?;
    let pool = state.db_manager.pool();
    let existing = sqlx::query("SELECT id, name FROM folders WHERE lower(name) = lower(?)")
        .bind(&name)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(r) = existing {
        return Ok(Folder { id: r.get("id"), name: r.get("name") });
    }
    let id = format!("folder-{}", uuid::Uuid::new_v4());
    sqlx::query("INSERT INTO folders (id, name, created_at) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(&name)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Folder { id, name })
}

#[tauri::command]
pub async fn folder_rename(state: State<'_, AppState>, id: String, name: String) -> Result<(), String> {
    let name = clean_name(&name)?;
    sqlx::query("UPDATE folders SET name = ? WHERE id = ?")
        .bind(name)
        .bind(id)
        .execute(state.db_manager.pool())
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn folder_delete(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let pool = state.db_manager.pool();
    sqlx::query("UPDATE meetings SET folder_id = NULL WHERE folder_id = ?")
        .bind(&id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM folders WHERE id = ?")
        .bind(&id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Which folder each filed meeting is in (meetings in no folder are left out).
#[tauri::command]
pub async fn meeting_folders(state: State<'_, AppState>) -> Result<Vec<MeetingFolder>, String> {
    let rows = sqlx::query("SELECT id, folder_id FROM meetings WHERE folder_id IS NOT NULL")
        .fetch_all(state.db_manager.pool())
        .await
        .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|r| MeetingFolder { meeting_id: r.get("id"), folder_id: r.get("folder_id") })
        .collect())
}

/// File a meeting in a folder, or take it out of its folder with `None`.
#[tauri::command]
pub async fn meeting_set_folder(
    state: State<'_, AppState>,
    meeting_id: String,
    folder_id: Option<String>,
) -> Result<(), String> {
    sqlx::query("UPDATE meetings SET folder_id = ? WHERE id = ?")
        .bind(folder_id)
        .bind(meeting_id)
        .execute(state.db_manager.pool())
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_tidied_and_checked() {
        assert_eq!(clean_name("  Customer   calls ").unwrap(), "Customer calls");
        assert!(clean_name("   ").is_err());
        assert!(clean_name(&"x".repeat(61)).is_err());
        assert!(clean_name(&"x".repeat(60)).is_ok());
    }
}
