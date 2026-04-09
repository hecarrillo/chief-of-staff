use crate::state::{self, data_dir, AppState, Message, MessageSource, Mode};
use crate::tmux;
use crate::todos::TodoStore;
use crate::vault;
use chrono::Utc;
use std::process::Command;
use std::sync::Arc;
use tauri::State;

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
        || which_exists("tmux");

    // Check claude CLI
    let claude_found = which_exists("claude");

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
    Command::new(shell)
        .args(args)
        .output()
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

#[tauri::command]
pub async fn answer_question(id: String, selected: Vec<String>) -> Result<String, String> {
    let client = reqwest::Client::new();
    let resp = client.post("http://localhost:7890/answer")
        .json(&serde_json::json!({"id": id, "selected": selected}))
        .send().await.map_err(|e| e.to_string())?;
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(body.get("message").and_then(|v| v.as_str()).unwrap_or("Answered").to_string())
}
