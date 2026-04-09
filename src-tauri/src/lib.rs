pub mod commands;
pub mod server;
pub mod state;
pub mod telegram;
pub mod tmux;
pub mod todos;
pub mod vault;
pub mod watcher;

use state::{AppState, BridgeConfig};
use std::sync::Arc;
use todos::TodoStore;

fn load_config() -> BridgeConfig {
    let config_path = state::data_dir().join("config.json");
    if let Ok(data) = std::fs::read_to_string(&config_path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        BridgeConfig::default()
    }
}

/// Kill any existing CoS session and start fresh
fn ensure_cos_session(config: &state::BridgeConfig) {
    let session_name = &config.cos_session;

    // Always kill the old session — no reuse across app instances
    if tmux::session_exists(session_name) {
        let _ = tmux::kill_session(session_name);
        // Brief pause so tmux fully cleans up
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    let cwd = if config.cos_cwd.is_empty() {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .to_string_lossy()
            .into_owned()
    } else {
        config.cos_cwd.clone()
    };

    if let Err(e) = tmux::create_session(session_name, &cwd) {
        eprintln!("Failed to create tmux session '{}': {}", session_name, e);
        return;
    }

    // Write framework to file so Claude reads it on startup
    let framework_path = state::data_dir().join("cos-framework.md");
    if let Err(e) = std::fs::write(&framework_path, &config.cos_framework) {
        eprintln!("Failed to write framework: {}", e);
    }

    // Launch Claude Code with the framework appended to system prompt
    // --append-system-prompt-file keeps CLAUDE.md discovery + adds our framework
    let cmd = format!(
        "claude --dangerously-skip-permissions --append-system-prompt-file {}",
        framework_path.to_string_lossy()
    );
    let _ = tmux::send_keys(session_name, &cmd);

    // Auto-accept the workspace trust dialog after a short delay
    // Claude shows a trust prompt for new directories — send Enter to accept
    std::thread::spawn({
        let name = session_name.to_string();
        move || {
            for _ in 0..10 {
                std::thread::sleep(std::time::Duration::from_secs(2));
                if let Ok(content) = tmux::capture_pane(&name, 15) {
                    if content.contains("Yes, I trust this folder") {
                        let _ = tmux::send_keys_raw(&name, "");
                        // send Enter key literally
                        let _ = std::process::Command::new(tmux::tmux_bin_pub())
                            .args(["send-keys", "-t", &name, "Enter"])
                            .output();
                        break;
                    }
                    // Already past trust dialog
                    if content.contains("Claude Code") && content.contains('>') {
                        break;
                    }
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = load_config();

    // Ensure data directories exist
    let data = state::data_dir();
    let _ = std::fs::create_dir_all(data.join("history"));
    let _ = std::fs::create_dir_all(data.join("images"));
    let _ = std::fs::create_dir_all(data.join("todos"));

    // Auto-init: create tmux CoS session if not running
    ensure_cos_session(&config);

    let app_state = AppState::new(config);
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
            commands::todo_list,
            commands::todo_add,
            commands::todo_toggle,
            commands::todo_remove,
            commands::answer_question,
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
