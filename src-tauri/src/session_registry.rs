use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub date: String,
    pub uuid: String,
    pub name: String,
    pub tmux_session: String,
    pub cwd: String,
    pub created_at: DateTime<Utc>,
}

pub fn sessions_dir() -> PathBuf {
    crate::state::data_dir().join("sessions")
}

pub fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

pub fn tmux_session_name(prefix: &str, date: &str) -> String {
    format!("{}-{}", prefix, date)
}

fn entry_path(date: &str) -> PathBuf {
    sessions_dir().join(format!("{}.json", date))
}

pub fn load(date: &str) -> Option<SessionEntry> {
    let data = std::fs::read_to_string(entry_path(date)).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save(entry: &SessionEntry) -> Result<(), String> {
    std::fs::create_dir_all(sessions_dir()).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(entry).map_err(|e| e.to_string())?;
    std::fs::write(entry_path(&entry.date), json).map_err(|e| e.to_string())
}

pub fn delete(date: &str) -> Result<(), String> {
    let path = entry_path(date);
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn list_all() -> Vec<SessionEntry> {
    let mut entries = Vec::new();
    if let Ok(read) = std::fs::read_dir(sessions_dir()) {
        for entry in read.flatten() {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(data) = std::fs::read_to_string(entry.path()) {
                    if let Ok(session) = serde_json::from_str::<SessionEntry>(&data) {
                        entries.push(session);
                    }
                }
            }
        }
    }
    entries.sort_by(|a, b| b.date.cmp(&a.date));
    entries
}

pub fn create_new(prefix: &str, date: &str, cwd: &str) -> SessionEntry {
    SessionEntry {
        date: date.to_string(),
        uuid: Uuid::new_v4().to_string(),
        name: date.to_string(),
        tmux_session: tmux_session_name(prefix, date),
        cwd: cwd.to_string(),
        created_at: Utc::now(),
    }
}
