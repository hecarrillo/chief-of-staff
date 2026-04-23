pub mod commands;
pub mod server;
pub mod session_registry;
pub mod state;
pub mod telegram;
pub mod tmux;
pub mod todos;
pub mod vault;
pub mod watcher;

use state::{AppState, BridgeConfig};
use std::sync::Arc;
use todos::TodoStore;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Resolve the Windows host IP as seen from inside WSL (the default gateway)
#[cfg(windows)]
fn resolve_wsl_host_ip() -> Option<String> {
    // The default gateway in WSL2 is the Windows host
    let output = std::process::Command::new("wsl")
        .args(["bash", "-c", "ip route show default | awk '{print $3}' | head -1"])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .ok()?;
    let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if ip.is_empty() { None } else { Some(ip) }
}

/// Convert a Windows path (C:\Users\...) to WSL path (/mnt/c/Users/...)
#[cfg(windows)]
fn win_to_wsl_path(path: &str) -> String {
    let p = path.replace('\\', "/");
    // Match drive letter pattern like C:/ or c:/
    if p.len() >= 3 && p.as_bytes()[1] == b':' && p.as_bytes()[2] == b'/' {
        let drive = (p.as_bytes()[0] as char).to_ascii_lowercase();
        format!("/mnt/{}/{}", drive, &p[3..])
    } else {
        p
    }
}

fn load_config() -> BridgeConfig {
    let config_path = state::data_dir().join("config.json");
    if let Ok(data) = std::fs::read_to_string(&config_path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        BridgeConfig::default()
    }
}

/// Resolve the framework file path, writing it out if needed, and return the
/// shell-friendly argument (WSL-converted on Windows).
fn prepare_framework_file(config: &state::BridgeConfig) -> String {
    let framework_path = state::data_dir().join("cos-framework.md");
    #[allow(unused_mut)]
    let mut framework_content = config.effective_framework();

    #[cfg(windows)]
    {
        let port = config.http_port;
        let host_ip = resolve_wsl_host_ip().unwrap_or_else(|| "host.internal".to_string());
        let from = format!("localhost:{}", port);
        let to = format!("{}:{}", host_ip, port);
        framework_content = framework_content.replace(&from, &to);
    }

    if let Err(e) = std::fs::write(&framework_path, &framework_content) {
        eprintln!("Failed to write framework: {}", e);
    }

    if cfg!(windows) {
        let win_path = framework_path.to_string_lossy().replace('\\', "/");
        if let Some(rest) = win_path.strip_prefix("C:/") {
            format!("/mnt/c/{}", rest)
        } else if let Some(rest) = win_path.strip_prefix("c:/") {
            format!("/mnt/c/{}", rest)
        } else {
            win_path
        }
    } else {
        framework_path.to_string_lossy().into_owned()
    }
}

/// Resolve cwd with WSL conversion on Windows
fn resolve_cwd(config: &state::BridgeConfig) -> String {
    let cwd = if config.cos_cwd.is_empty() {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .to_string_lossy()
            .into_owned()
    } else {
        config.cos_cwd.clone()
    };

    #[cfg(windows)]
    let cwd = win_to_wsl_path(&cwd);

    cwd
}

fn spawn_trust_accept(session: String) {
    std::thread::spawn(move || {
        for _ in 0..10 {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if let Ok(content) = tmux::capture_pane(&session, 15) {
                if content.contains("Yes, I trust this folder") {
                    let _ = tmux::send_keys(&session, "");
                    break;
                }
                if content.contains("Claude Code") && content.contains('>') {
                    break;
                }
            }
        }
    });
}

/// Ensure a tmux session for a specific dated SessionEntry.
/// Returns the tmux session name. Does nothing if the tmux session already exists.
/// - `is_new` indicates whether this entry was just created (fresh UUID) vs. loaded from registry.
pub fn ensure_dated_session(
    entry: &session_registry::SessionEntry,
    config: &state::BridgeConfig,
    is_new: bool,
) -> String {
    let tmux_name = entry.tmux_session.clone();

    // Already alive — just return. App will target it.
    if tmux::session_exists(&tmux_name) {
        return tmux_name;
    }

    if let Err(e) = tmux::create_session(&tmux_name, &entry.cwd) {
        eprintln!("Failed to create tmux session '{}': {}", tmux_name, e);
        return tmux_name;
    }

    let framework_arg = prepare_framework_file(config);

    // New session: create with deterministic UUID and display name.
    // Existing session: resume by UUID so history comes back.
    let claude_cmd = if is_new {
        format!(
            "claude --dangerously-skip-permissions --session-id {} --name {} --append-system-prompt-file {}",
            entry.uuid, entry.name, framework_arg
        )
    } else {
        format!(
            "claude --dangerously-skip-permissions --resume {} --append-system-prompt-file {}",
            entry.uuid, framework_arg
        )
    };
    let _ = tmux::send_keys(&tmux_name, &claude_cmd);

    spawn_trust_accept(tmux_name.clone());
    tmux_name
}

/// Launch mode controlled by the frontend launch prompt.
#[derive(Debug, Clone, Copy)]
pub enum LaunchMode {
    /// Create a fresh session (new UUID). Kills any prior tmux for today.
    Renew,
    /// Resume the existing registry entry for today. Creates registry if missing.
    Continue,
}

/// Phase 2 init: executed after the UI resolves the launch prompt.
/// Returns (tmux_session_name, was_resumed).
pub fn init_today_session(config: &state::BridgeConfig, mode: LaunchMode) -> (String, bool) {
    let today = session_registry::today_str();
    let cwd = resolve_cwd(config);

    let (entry, is_new) = match (session_registry::load(&today), mode) {
        (Some(e), LaunchMode::Continue) => (e, false),
        (_, LaunchMode::Renew) | (None, _) => {
            // Kill any stale tmux for today, clear registry, create fresh
            let tmux_name = session_registry::tmux_session_name(&config.cos_session, &today);
            if tmux::session_exists(&tmux_name) {
                let _ = tmux::kill_session(&tmux_name);
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            let _ = session_registry::delete(&today);
            let e = session_registry::create_new(&config.cos_session, &today, &cwd);
            let _ = session_registry::save(&e);
            (e, true)
        }
    };

    let tmux_name = ensure_dated_session(&entry, config, is_new);
    (tmux_name, !is_new)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = load_config();

    // Ensure data directories exist
    let data = state::data_dir();
    let _ = std::fs::create_dir_all(data.join("history"));
    let _ = std::fs::create_dir_all(data.join("images"));
    let _ = std::fs::create_dir_all(data.join("todos"));

    // Phase 1 only: build state with empty target. Frontend will call launch_session()
    // after the user resolves the launch prompt, which creates/resumes the tmux session.
    let app_state = AppState::new_full(config, String::new(), false);
    let todo_store = TodoStore::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .manage(Arc::clone(&app_state))
        .manage(Arc::clone(&todo_store))
        .invoke_handler(tauri::generate_handler![
            commands::send_message,
            commands::get_messages,
            commands::get_mode,
            commands::set_mode_manual,
            commands::send_message_with_image,
            commands::save_image,
            commands::get_sessions,
            commands::get_windows,
            commands::get_screen,
            commands::set_target_window,
            commands::get_target_window,
            commands::check_system,
            commands::send_to_session,
            commands::get_session_status,
            commands::get_config,
            commands::save_config,
            commands::get_vault_files,
            commands::read_vault_file,
            commands::read_daily_note,
            commands::write_daily_note,
            commands::todo_list,
            commands::todo_add,
            commands::todo_toggle,
            commands::todo_remove,
            commands::answer_question,
            commands::list_dated_sessions,
            commands::current_session,
            commands::resume_session,
            commands::renew_today,
            commands::get_launch_state,
            commands::launch_session,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let state = Arc::clone(&app_state);

            let server_handle = handle.clone();
            let server_state = Arc::clone(&state);
            let server_todos = Arc::clone(&todo_store);
            tauri::async_runtime::spawn(async move {
                server::start_server(server_handle, server_state, server_todos).await;
            });

            let tg_handle = handle.clone();
            let tg_state = Arc::clone(&state);
            let tg_todos = Arc::clone(&todo_store);
            tauri::async_runtime::spawn(async move {
                telegram::start_polling(tg_handle, tg_state, tg_todos).await;
            });

            // Response watcher: polls tmux for Claude output → forwards to app
            let watch_handle = handle.clone();
            let watch_state = Arc::clone(&state);
            tauri::async_runtime::spawn(async move {
                watcher::start_response_watcher(watch_handle, watch_state).await;
            });

            let vault_path = tauri::async_runtime::block_on(state.config.read())
                .vault_path
                .clone();
            vault::start_watching(handle, &vault_path);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
