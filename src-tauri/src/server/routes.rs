use super::types::*;
use crate::state::{AppState, Message, MessageSource, Mode};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::{oneshot, RwLock};

pub struct ServerState {
    pub app_state: Arc<AppState>,
    pub app_handle: AppHandle,
    pub todo_store: Arc<crate::todos::TodoStore>,
    pub pending_questions: RwLock<HashMap<String, oneshot::Sender<Vec<String>>>>,
    pub latest_question_id: RwLock<Option<String>>,
    pub latest_question_options: RwLock<Vec<String>>,
}

pub async fn post_message(
    State(server): State<Arc<ServerState>>,
    Json(payload): Json<IncomingMessage>,
) -> (StatusCode, Json<ApiResponse>) {
    let msg = Message {
        id: uuid::Uuid::new_v4().to_string(),
        from: MessageSource::Cos,
        text: payload.text.clone(),
        timestamp: Utc::now(),
        forward_to_telegram: payload.telegram,
        delivered: true,
        image: payload.image.clone(),
        reply_to: None,
        reply_preview: None,
    };

    server.app_state.add_message(msg.clone()).await;
    let _ = server.app_handle.emit("new-message", &msg);

    let last_from_tg = *server.app_state.last_from_telegram.read().await;
    let is_away = *server.app_state.mode.read().await == Mode::Away;
    let should_forward = payload.telegram || last_from_tg || is_away;

    if should_forward {
        if let Some(ref image_path) = payload.image {
            let _ = crate::telegram::send_telegram_photo(
                &server.app_state, image_path, &payload.text
            ).await;
        } else {
            let _ = crate::telegram::send_telegram(&server.app_state, &payload.text).await;
        }
    }

    (StatusCode::OK, Json(ApiResponse { ok: true, message: "Message received".into() }))
}

pub async fn post_status(
    State(server): State<Arc<ServerState>>,
    Json(payload): Json<StatusUpdate>,
) -> (StatusCode, Json<ApiResponse>) {
    let text = format!(
        "[Status] {}: {}{}",
        payload.project.as_deref().unwrap_or("General"),
        payload.status,
        payload.details.as_ref().map(|d| format!("\n{}", d)).unwrap_or_default()
    );

    let msg = Message {
        id: uuid::Uuid::new_v4().to_string(),
        from: MessageSource::System,
        text,
        timestamp: Utc::now(),
        forward_to_telegram: false,
        delivered: true,
        image: None,
        reply_to: None,
        reply_preview: None,
    };

    server.app_state.add_message(msg.clone()).await;
    let _ = server.app_handle.emit("new-message", &msg);
    let _ = server.app_handle.emit("status-update", &payload);

    (StatusCode::OK, Json(ApiResponse { ok: true, message: "Status updated".into() }))
}

pub async fn get_mode(
    State(server): State<Arc<ServerState>>,
) -> Json<ModeResponse> {
    let mode = server.app_state.mode.read().await;
    Json(ModeResponse {
        mode: match *mode { Mode::AtDesk => "at_desk".into(), Mode::Away => "away".into() },
    })
}

pub async fn get_config(
    State(server): State<Arc<ServerState>>,
) -> Json<ConfigResponse> {
    let config = server.app_state.config.read().await;
    Json(ConfigResponse {
        bot_token: config.bot_token.clone(),
        chat_id: config.chat_id.clone(),
        vault_path: config.vault_path.clone(),
        cos_session: config.cos_session.clone(),
    })
}

pub async fn debug_tmux(
    State(server): State<Arc<ServerState>>,
) -> Json<ApiResponse> {
    let target = server.app_state.target_window.read().await.clone();
    match crate::tmux::send_keys(&target, "# ping") {
        Ok(()) => Json(ApiResponse { ok: true, message: "tmux send OK".into() }),
        Err(e) => Json(ApiResponse { ok: false, message: format!("tmux FAILED: {}", e) }),
    }
}

pub async fn todo_list(
    State(server): State<Arc<ServerState>>,
) -> Json<Vec<crate::todos::Todo>> {
    Json(server.todo_store.list().await)
}

pub async fn todo_add(
    State(server): State<Arc<ServerState>>,
    Json(payload): Json<TodoAddRequest>,
) -> (StatusCode, Json<ApiResponse>) {
    let todo = server.todo_store.add(&payload.text, &payload.added_by.unwrap_or("cos".into())).await;
    let _ = server.app_handle.emit("todo-changed", &server.todo_store.list().await);
    (StatusCode::OK, Json(ApiResponse { ok: true, message: format!("Added: {}", todo.text) }))
}

pub async fn todo_toggle(
    State(server): State<Arc<ServerState>>,
    Json(payload): Json<TodoToggleRequest>,
) -> (StatusCode, Json<ApiResponse>) {
    if let Some(todo) = server.todo_store.toggle(&payload.id).await {
        let _ = server.app_handle.emit("todo-changed", &server.todo_store.list().await);
        let status = if todo.done { "done" } else { "undone" };
        (StatusCode::OK, Json(ApiResponse { ok: true, message: format!("Marked {}: {}", status, todo.text) }))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse { ok: false, message: "Todo not found".into() }))
    }
}

pub async fn post_question(
    State(server): State<Arc<ServerState>>,
    Json(payload): Json<QuestionRequest>,
) -> Json<ApiResponse> {
    let id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = oneshot::channel::<Vec<String>>();

    server.pending_questions.write().await.insert(id.clone(), tx);
    *server.latest_question_id.write().await = Some(id.clone());
    *server.latest_question_options.write().await = payload.options.clone();

    let question_payload = QuestionPayload {
        id: id.clone(),
        question: payload.question.clone(),
        options: payload.options.clone(),
        multi_select: payload.multi_select,
    };
    let _ = server.app_handle.emit("cos-question", &question_payload);

    let is_away = *server.app_state.mode.read().await == Mode::Away;
    if is_away {
        let mut tg_text = format!("CoS asks:\n{}\n", payload.question);
        for (i, opt) in payload.options.iter().enumerate() {
            tg_text.push_str(&format!("{}. {}\n", i + 1, opt));
        }
        tg_text.push_str("\nReply with the number(s).");
        let _ = crate::telegram::send_telegram(&server.app_state, &tg_text).await;
    }

    match tokio::time::timeout(std::time::Duration::from_secs(300), rx).await {
        Ok(Ok(selected)) => Json(ApiResponse { ok: true, message: selected.join(", ") }),
        _ => {
            server.pending_questions.write().await.remove(&id);
            Json(ApiResponse { ok: false, message: "Question timed out".into() })
        }
    }
}

pub async fn post_answer(
    State(server): State<Arc<ServerState>>,
    Json(payload): Json<QuestionAnswer>,
) -> (StatusCode, Json<ApiResponse>) {
    let mut pending = server.pending_questions.write().await;
    if let Some(tx) = pending.remove(&payload.id) {
        let _ = tx.send(payload.selected);
        (StatusCode::OK, Json(ApiResponse { ok: true, message: "Answer delivered".into() }))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse { ok: false, message: "No pending question".into() }))
    }
}

pub async fn answer_by_number(
    State(server): State<Arc<ServerState>>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse>) {
    let num = payload.get("number").and_then(|n| n.as_u64()).unwrap_or(0) as usize;
    let options = server.latest_question_options.read().await;
    let qid = server.latest_question_id.read().await.clone();

    if num == 0 || num > options.len() {
        return (StatusCode::BAD_REQUEST, Json(ApiResponse { ok: false, message: "Invalid option number".into() }));
    }

    let selected = options[num - 1].clone();
    drop(options);

    if let Some(id) = qid {
        let mut pending = server.pending_questions.write().await;
        if let Some(tx) = pending.remove(&id) {
            let _ = tx.send(vec![selected.clone()]);
            return (StatusCode::OK, Json(ApiResponse { ok: true, message: format!("Selected: {}", selected) }));
        }
    }

    (StatusCode::NOT_FOUND, Json(ApiResponse { ok: false, message: "No pending question".into() }))
}
