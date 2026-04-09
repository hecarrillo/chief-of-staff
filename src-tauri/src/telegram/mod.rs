pub mod types;

use crate::state::{data_dir, AppState, Message, MessageSource};
use crate::todos::TodoStore;
use chrono::Utc;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::time::{interval, Duration};
use types::{SendMessageResponse, TelegramFileResponse, TelegramResponse};

const POLL_INTERVAL_SECS: u64 = 5;

pub async fn start_polling(app: AppHandle, state: Arc<AppState>, todos: Arc<TodoStore>) {
    let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));
    let mut last_update_id: Option<i64> = load_last_update_id().await;
    let client = reqwest::Client::new();

    loop {
        ticker.tick().await;
        let config = state.config.read().await;
        if config.bot_token.is_empty() {
            continue;
        }
        let token = config.bot_token.clone();
        let chat_id = config.chat_id.clone();
        drop(config);

        let url = match last_update_id {
            Some(id) => format!(
                "https://api.telegram.org/bot{}/getUpdates?offset={}&timeout=1",
                token, id + 1
            ),
            None => format!(
                "https://api.telegram.org/bot{}/getUpdates?timeout=1",
                token
            ),
        };

        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };

        let updates: TelegramResponse = match resp.json().await {
            Ok(u) => u,
            Err(_) => continue,
        };

        if !updates.ok {
            continue;
        }

        for update in updates.result {
            last_update_id = Some(update.update_id);
            save_last_update_id(update.update_id).await;

            let Some(tg_msg) = update.message else { continue };
            if tg_msg.chat.id.to_string() != chat_id {
                continue;
            }

            // Handle photo messages
            let (text, image) = if let Some(photos) = &tg_msg.photo {
                let caption = tg_msg.caption.clone().unwrap_or_default();
                if let Some(photo) = photos.last() {
                    let path = download_telegram_file(&client, &token, &photo.file_id).await;
                    (
                        if caption.is_empty() { "[Photo]".to_string() } else { caption },
                        path,
                    )
                } else {
                    continue;
                }
            } else if let Some(text) = tg_msg.text {
                (text, None)
            } else {
                continue;
            };

            // Handle /screen command — reads current tmux target
            if text.starts_with("/screen") {
                let target = state.target_window.read().await.clone();
                match crate::tmux::capture_pane(&target, 40) {
                    Ok(content) => {
                        let truncated = if content.len() > 3900 {
                            format!("{}...\n(truncated)", &content[..3900])
                        } else {
                            content
                        };
                        let _ = send_telegram(&state, &truncated).await;
                    }
                    Err(e) => {
                        let _ = send_telegram(&state, &format!("Screen read failed: {}", e)).await;
                    }
                }
                continue;
            }

            // Handle /todo commands
            let normalized = text
                .replace("/todo_add ", "/todo add ")
                .replace("/todo_done ", "/todo done ");
            if normalized.starts_with("/todo") {
                let reply = handle_todo_command(&normalized, &todos).await;
                let _ = send_telegram(&state, &reply).await;
                let _ = app.emit("todo-changed", &todos.list().await);
                continue;
            }

            // Handle numeric replies as answers to pending questions
            if let Ok(num) = text.trim().parse::<usize>() {
                let resp = client.post("http://localhost:7890/answer_number")
                    .json(&serde_json::json!({"number": num}))
                    .send()
                    .await;
                if let Ok(r) = resp {
                    if let Ok(body) = r.json::<serde_json::Value>().await {
                        if body.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                            let msg_text = body.get("message").and_then(|v| v.as_str()).unwrap_or("Answered");
                            let _ = send_telegram(&state, msg_text).await;
                            continue;
                        }
                    }
                }
            }

            // Extract reply context
            let (reply_preview, cos_text) = if let Some(ref reply_msg) = tg_msg.reply_to_message {
                let preview = reply_msg.text.clone()
                    .or(reply_msg.caption.clone())
                    .unwrap_or_default();
                let short = if preview.len() > 80 { format!("{}...", &preview[..80]) } else { preview.clone() };
                let prefixed = format!("[Replying to: \"{}\"]\n{}", short, text);
                (Some(short), prefixed)
            } else {
                (None, text.clone())
            };

            let msg = Message {
                id: format!("tg-{}", update.update_id),
                from: MessageSource::Telegram,
                text: text.clone(),
                timestamp: Utc::now(),
                forward_to_telegram: false,
                delivered: true,
                image: image.clone(),
                reply_to: None,
                reply_preview,
            };

            let is_new = state.add_message(msg.clone()).await;
            if is_new {
                *state.last_from_telegram.write().await = true;
                *state.mode.write().await = crate::state::Mode::Away;

                let _ = app.emit("new-message", &msg);
                let _ = app.emit("mode-changed", "away");

                // Forward to CoS via tmux
                let cos_text = if let Some(ref img_path) = image {
                    format!("{}\n[Image: {}]", cos_text, img_path)
                } else {
                    cos_text
                };
                let target = state.target_window.read().await.clone();
                if let Err(e) = crate::tmux::send_keys(&target, &cos_text) {
                    eprintln!("[telegram] tmux forward failed: {}", e);
                }
            }
        }
    }
}

async fn download_telegram_file(
    client: &reqwest::Client,
    token: &str,
    file_id: &str,
) -> Option<String> {
    let url = format!("https://api.telegram.org/bot{}/getFile?file_id={}", token, file_id);
    let resp = client.get(&url).send().await.ok()?;
    let file_resp: TelegramFileResponse = resp.json().await.ok()?;
    let file_path = file_resp.result?.file_path?;

    let download_url = format!("https://api.telegram.org/file/bot{}/{}", token, file_path);
    let bytes = client.get(&download_url).send().await.ok()?.bytes().await.ok()?;

    let ext = file_path.rsplit('.').next().unwrap_or("jpg");
    let images_dir = data_dir().join("images");
    let _ = std::fs::create_dir_all(&images_dir);
    let local_path = images_dir.join(format!("tg-{}.{}", chrono::Utc::now().timestamp_millis(), ext));
    tokio::fs::write(&local_path, &bytes).await.ok()?;
    Some(local_path.to_string_lossy().into_owned())
}

pub async fn send_telegram(state: &AppState, text: &str) -> Result<(), String> {
    let config = state.config.read().await;
    let url = format!("https://api.telegram.org/bot{}/sendMessage", config.bot_token);
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "chat_id": config.chat_id,
            "text": text
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let result: SendMessageResponse = resp.json().await.map_err(|e| e.to_string())?;
    if result.ok { Ok(()) } else { Err("Telegram sendMessage failed".into()) }
}

pub async fn send_telegram_photo(state: &AppState, path: &str, caption: &str) -> Result<(), String> {
    let config = state.config.read().await;
    let url = format!("https://api.telegram.org/bot{}/sendPhoto", config.bot_token);
    let file_bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
    let file_name = std::path::Path::new(path)
        .file_name().unwrap_or_default()
        .to_string_lossy().into_owned();

    let part = reqwest::multipart::Part::bytes(file_bytes).file_name(file_name);
    let form = reqwest::multipart::Form::new()
        .text("chat_id", config.chat_id.clone())
        .text("caption", caption.to_string())
        .part("photo", part);

    let client = reqwest::Client::new();
    let resp = client.post(&url).multipart(form).send().await.map_err(|e| e.to_string())?;
    let result: SendMessageResponse = resp.json().await.map_err(|e| e.to_string())?;
    if result.ok { Ok(()) } else { Err("Telegram sendPhoto failed".into()) }
}

async fn handle_todo_command(text: &str, todos: &TodoStore) -> String {
    let args = text.strip_prefix("/todo").unwrap_or("").trim();

    if args.is_empty() || args == "list" {
        return todos.format_for_telegram().await;
    }
    if let Some(task_text) = args.strip_prefix("add ") {
        let task_text = task_text.trim();
        if !task_text.is_empty() {
            todos.add(task_text, "telegram").await;
            return format!("Added: {}", task_text);
        }
    }
    if let Some(num) = args.strip_prefix("done ") {
        if let Ok(idx) = num.trim().parse::<usize>() {
            let items = todos.list().await;
            if idx > 0 && idx <= items.len() {
                if let Some(todo) = todos.toggle(&items[idx - 1].id).await {
                    let status = if todo.done { "done" } else { "undone" };
                    return format!("Marked {}: {}", status, todo.text);
                }
            }
        }
        return "Invalid task number".to_string();
    }
    if !args.is_empty() {
        todos.add(args, "telegram").await;
        return format!("Added: {}", args);
    }
    "Usage: /todo, /todo add <task>, /todo done <number>".to_string()
}

async fn load_last_update_id() -> Option<i64> {
    let path = data_dir().join("telegram_state.json");
    let data = tokio::fs::read_to_string(path).await.ok()?;
    let val: serde_json::Value = serde_json::from_str(&data).ok()?;
    val.get("last_update_id")?.as_i64()
}

async fn save_last_update_id(id: i64) {
    let path = data_dir().join("telegram_state.json");
    let json = serde_json::json!({ "last_update_id": id });
    let _ = tokio::fs::write(path, serde_json::to_string_pretty(&json).unwrap()).await;
}
