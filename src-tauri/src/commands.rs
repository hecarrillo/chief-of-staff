use crate::session_registry::{self, SessionEntry};
use crate::state::{self, data_dir, AppState, Message, MessageSource, Mode};
use crate::tmux;
use crate::todos::TodoStore;
use crate::vault;
use chrono::Utc;
use std::process::Command;
use std::sync::Arc;
use tauri::State;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

type SharedState = Arc<AppState>;
type SharedTodos = Arc<TodoStore>;

#[tauri::command]
pub async fn send_message(
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
    text: String,
) -> Result<Message, String> {
    *state.last_from_telegram.write().await = false;
    *state.mode.write().await = Mode::AtDesk;
    let _ = tauri::Emitter::emit(&app, "mode-changed", "at_desk");

    let target = state.target_window.read().await.clone();
    let delivered = tmux::send_keys(&target, &text).is_ok();

    let msg = Message {
        id: uuid::Uuid::new_v4().to_string(),
        from: MessageSource::Hector,
        text,
        timestamp: Utc::now(),
        forward_to_telegram: false,
        delivered,
        image: None,
        reply_to: None,
        reply_preview: None,
    };

    state.add_message(msg.clone()).await;
    Ok(msg)
}

#[tauri::command]
pub async fn get_messages(state: State<'_, SharedState>) -> Result<Vec<Message>, String> {
    Ok(state.messages.read().await.clone())
}

#[tauri::command]
pub async fn get_mode(state: State<'_, SharedState>) -> Result<String, String> {
    let mode = state.mode.read().await;
    Ok(match *mode {
        Mode::AtDesk => "at_desk".to_string(),
        Mode::Away => "away".to_string(),
    })
}

#[tauri::command]
pub async fn set_mode_manual(
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
    mode: String,
) -> Result<(), String> {
    let new_mode = match mode.as_str() {
        "at_desk" => Mode::AtDesk,
        "away" => Mode::Away,
        _ => return Err("Invalid mode".into()),
    };
    *state.mode.write().await = new_mode;
    let _ = tauri::Emitter::emit(&app, "mode-changed", mode.as_str());
    Ok(())
}

#[tauri::command]
pub async fn send_message_with_image(
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
    text: String,
    image_path: String,
) -> Result<Message, String> {
    *state.last_from_telegram.write().await = false;
    *state.mode.write().await = Mode::AtDesk;
    let _ = tauri::Emitter::emit(&app, "mode-changed", "at_desk");

    let cos_text = format!("{}\n[Image: {}]", text, image_path);
    let target = state.target_window.read().await.clone();
    let delivered = tmux::send_keys(&target, &cos_text).is_ok();

    let msg = Message {
        id: uuid::Uuid::new_v4().to_string(),
        from: MessageSource::Hector,
        text,
        timestamp: Utc::now(),
        forward_to_telegram: false,
        delivered,
        image: Some(image_path),
        reply_to: None,
        reply_preview: None,
    };

    state.add_message(msg.clone()).await;
    Ok(msg)
}

#[tauri::command]
pub async fn save_image(bytes: Vec<u8>, ext: String) -> Result<String, String> {
    let dir = data_dir().join("images");
    let _ = tokio::fs::create_dir_all(&dir).await;
    let path = dir.join(format!("ui-{}.{}", chrono::Utc::now().timestamp_millis(), ext));
    tokio::fs::write(&path, &bytes).await.map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

// --- tmux session management ---

#[tauri::command]
pub fn get_sessions() -> Result<Vec<tmux::SessionInfo>, String> {
    tmux::list_sessions()
}

#[tauri::command]
pub fn get_windows(session: String) -> Result<Vec<tmux::WindowInfo>, String> {
    tmux::list_windows(&session)
}

#[tauri::command]
pub fn get_screen(target: String, lines: u32) -> Result<String, String> {
    tmux::capture_pane(&target, lines)
}

#[tauri::command]
pub async fn set_target_window(
    state: State<'_, SharedState>,
    target: String,
) -> Result<(), String> {
    *state.target_window.write().await = target;
    Ok(())
}

#[tauri::command]
pub async fn get_target_window(
    state: State<'_, SharedState>,
) -> Result<String, String> {
    Ok(state.target_window.read().await.clone())
}

#[tauri::command]
pub fn send_to_session(target: String, text: String) -> Result<(), String> {
    tmux::send_keys(&target, &text)
}

#[tauri::command]
pub async fn get_session_status(
    state: State<'_, SharedState>,
) -> Result<SessionStatus, String> {
    let target = state.target_window.read().await.clone();
    let session_name = target.split(':').next().unwrap_or(&target).to_string();
    let exists = tmux::session_exists(&session_name);
    let ready = if exists { tmux::is_claude_ready(&target) } else { false };
    Ok(SessionStatus { exists, ready, target })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionStatus {
    pub exists: bool,
    pub ready: bool,
    pub target: String,
}

// --- setup / onboarding ---

#[tauri::command]
pub fn check_system() -> SystemCheck {
    let os = std::env::consts::OS.to_string();

    // Check tmux
    let tmux_path = crate::tmux::tmux_bin_pub().to_string();
    let tmux_found = std::path::Path::new(&tmux_path).exists()
        || crate::tmux::is_running()
        || which_exists("tmux")
        || wsl_which_exists("tmux");

    // Check claude CLI
    let claude_found = which_exists("claude") || wsl_which_exists("claude");

    // Check if config exists (first run?)
    let config_exists = state::data_dir().join("config.json").exists();

    SystemCheck {
        os,
        tmux_found,
        tmux_path: if tmux_found { tmux_path } else { String::new() },
        claude_found,
        config_exists,
        home_dir: dirs::home_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    }
}

fn which_exists(cmd: &str) -> bool {
    // Try common shell to resolve
    let shell = if cfg!(windows) { "cmd" } else { "/bin/sh" };
    let args: &[&str] = if cfg!(windows) {
        &["/C", &format!("where {}", cmd)]
    } else {
        &["-c", &format!("which {}", cmd)]
    };
    let mut c = Command::new(shell);
    c.args(args);
    #[cfg(windows)]
    c.creation_flags(CREATE_NO_WINDOW);
    c.output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if a command exists inside WSL (Windows only)
fn wsl_which_exists(cmd: &str) -> bool {
    if !cfg!(windows) {
        return false;
    }
    let mut c = Command::new("wsl");
    c.args(["--", "bash", "-c", &format!("which {}", cmd)]);
    #[cfg(windows)]
    c.creation_flags(CREATE_NO_WINDOW);
    c.output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemCheck {
    pub os: String,
    pub tmux_found: bool,
    pub tmux_path: String,
    pub claude_found: bool,
    pub config_exists: bool,
    pub home_dir: String,
}

// --- settings ---

#[tauri::command]
pub async fn get_config(
    state: State<'_, SharedState>,
) -> Result<crate::state::BridgeConfig, String> {
    Ok(state.config.read().await.clone())
}

#[tauri::command]
pub async fn save_config(
    state: State<'_, SharedState>,
    config: crate::state::BridgeConfig,
) -> Result<(), String> {
    config.save()?;
    *state.config.write().await = config;
    Ok(())
}

// --- vault ---

#[tauri::command]
pub fn get_vault_files(
    state: State<'_, SharedState>,
    category: String,
) -> Result<Vec<vault::VaultFile>, String> {
    let config = tauri::async_runtime::block_on(state.config.read());
    vault::read_vault_files(&config.vault_path, &category)
}

#[tauri::command]
pub fn read_vault_file(path: String) -> Result<vault::VaultFile, String> {
    vault::read_file(&path)
}

// --- daily note (today's RAM) ---

fn daily_path_for(vault_path: &str, date: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(vault_path).join("daily").join(format!("{}.md", date))
}

const DAILY_TEMPLATE: &str = "# {date}\n\n## Todos\n\n## Log\n\n## Open\n";

#[tauri::command]
pub async fn read_daily_note(
    state: State<'_, SharedState>,
    date: Option<String>,
) -> Result<DailyNote, String> {
    let vault = state.config.read().await.vault_path.clone();
    if vault.is_empty() {
        return Err("vault_path not configured".into());
    }
    let d = date.unwrap_or_else(|| session_registry::today_str());
    let path = daily_path_for(&vault, &d);
    let content = if path.exists() {
        std::fs::read_to_string(&path).map_err(|e| e.to_string())?
    } else {
        let tpl = DAILY_TEMPLATE.replace("{date}", &d);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&path, &tpl).map_err(|e| e.to_string())?;
        tpl
    };
    Ok(DailyNote {
        date: d,
        path: path.to_string_lossy().into_owned(),
        content,
    })
}

#[tauri::command]
pub async fn write_daily_note(
    state: State<'_, SharedState>,
    date: Option<String>,
    content: String,
) -> Result<(), String> {
    let vault = state.config.read().await.vault_path.clone();
    if vault.is_empty() {
        return Err("vault_path not configured".into());
    }
    let d = date.unwrap_or_else(|| session_registry::today_str());
    let path = daily_path_for(&vault, &d);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DailyNote {
    pub date: String,
    pub path: String,
    pub content: String,
}

// --- todos ---

#[tauri::command]
pub async fn todo_list(todos: State<'_, SharedTodos>) -> Result<Vec<crate::todos::Todo>, String> {
    Ok(todos.list().await)
}

#[tauri::command]
pub async fn todo_add(
    app: tauri::AppHandle,
    todos: State<'_, SharedTodos>,
    text: String,
) -> Result<crate::todos::Todo, String> {
    let todo = todos.add(&text, "hector").await;
    let _ = tauri::Emitter::emit(&app, "todo-changed", &todos.list().await);
    Ok(todo)
}

#[tauri::command]
pub async fn todo_toggle(
    app: tauri::AppHandle,
    todos: State<'_, SharedTodos>,
    id: String,
) -> Result<Option<crate::todos::Todo>, String> {
    let result = todos.toggle(&id).await;
    let _ = tauri::Emitter::emit(&app, "todo-changed", &todos.list().await);
    Ok(result)
}

#[tauri::command]
pub async fn todo_remove(
    app: tauri::AppHandle,
    todos: State<'_, SharedTodos>,
    id: String,
) -> Result<bool, String> {
    let removed = todos.remove(&id).await;
    let _ = tauri::Emitter::emit(&app, "todo-changed", &todos.list().await);
    Ok(removed)
}

// --- dated session registry ---

#[tauri::command]
pub fn list_dated_sessions() -> Vec<SessionEntry> {
    session_registry::list_all()
}

#[tauri::command]
pub async fn current_session(state: State<'_, SharedState>) -> Result<CurrentSession, String> {
    let today = session_registry::today_str();
    let entry = session_registry::load(&today);
    let target = state.target_window.read().await.clone();
    let was_resumed = *state.session_was_resumed.read().await;

    Ok(CurrentSession { today, entry, target, was_resumed })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CurrentSession {
    pub today: String,
    pub entry: Option<SessionEntry>,
    pub target: String,
    pub was_resumed: bool,
}

/// Launch state queried by the UI on app mount. Decides whether to show the launch prompt.
#[tauri::command]
pub async fn get_launch_state(state: State<'_, SharedState>) -> Result<LaunchState, String> {
    let today = session_registry::today_str();
    let entry = session_registry::load(&today);
    let target = state.target_window.read().await.clone();
    // needs_prompt: we have an existing entry AND the user hasn't launched yet (target empty).
    let needs_prompt = entry.is_some() && target.is_empty();

    // Framework staleness: framework mtime vs entry created_at.
    let framework_stale = entry
        .as_ref()
        .map(|e| {
            let fw = state::data_dir().join("cos-framework.md");
            if let Ok(meta) = std::fs::metadata(&fw) {
                if let Ok(modified) = meta.modified() {
                    let fw_ts: chrono::DateTime<chrono::Utc> = modified.into();
                    return fw_ts > e.created_at;
                }
            }
            false
        })
        .unwrap_or(false);

    Ok(LaunchState {
        today,
        entry,
        needs_prompt,
        framework_stale,
        launched: !target.is_empty(),
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LaunchState {
    pub today: String,
    pub entry: Option<SessionEntry>,
    pub needs_prompt: bool,
    pub framework_stale: bool,
    pub launched: bool,
}

/// Phase-2 init: create/resume tmux session for today based on chosen mode.
#[tauri::command]
pub async fn launch_session(
    state: State<'_, SharedState>,
    mode: String,
) -> Result<String, String> {
    let launch_mode = match mode.as_str() {
        "renew" => crate::LaunchMode::Renew,
        "continue" => crate::LaunchMode::Continue,
        other => return Err(format!("Unknown launch mode: {}", other)),
    };
    let config = state.config.read().await.clone();
    let (tmux_name, was_resumed) = crate::init_today_session(&config, launch_mode);
    *state.target_window.write().await = tmux_name.clone();
    *state.session_was_resumed.write().await = was_resumed;
    Ok(tmux_name)
}

/// Bring up the tmux session for a given date (creating it + resuming Claude if needed)
/// and switch the app's target_window to it.
#[tauri::command]
pub async fn resume_session(
    state: State<'_, SharedState>,
    date: String,
) -> Result<String, String> {
    let entry = session_registry::load(&date)
        .ok_or_else(|| format!("No session found for {}", date))?;
    let config = state.config.read().await.clone();

    // If today's date and the entry doesn't exist yet, create_new logic handled elsewhere.
    // Here we always treat as resume (is_new=false).
    let tmux_name = crate::ensure_dated_session(&entry, &config, false);
    *state.target_window.write().await = tmux_name.clone();
    Ok(tmux_name)
}

/// Kill today's tmux session, delete the registry entry, and init a fresh session for today.
#[tauri::command]
pub async fn renew_today(state: State<'_, SharedState>) -> Result<SessionEntry, String> {
    let config = state.config.read().await.clone();
    let (tmux_name, _) = crate::init_today_session(&config, crate::LaunchMode::Renew);
    *state.target_window.write().await = tmux_name;
    *state.session_was_resumed.write().await = false;
    let today = session_registry::today_str();
    session_registry::load(&today).ok_or_else(|| "Failed to load fresh entry".to_string())
}

#[tauri::command]
pub async fn answer_question(id: String, selected: Vec<String>) -> Result<String, String> {
    let client = reqwest::Client::new();
    let resp = client.post("http://localhost:7890/answer")
        .json(&serde_json::json!({"id": id, "selected": selected}))
        .send().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(body.get("message").and_then(|v| v.as_str()).unwrap_or("Answered").to_string())
}
