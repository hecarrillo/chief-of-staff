pub mod routes;
pub mod types;

use crate::state::AppState;
use crate::todos::TodoStore;
use axum::routing::{get, post};
use axum::Router;
use routes::ServerState;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::net::TcpListener;

pub async fn start_server(app: AppHandle, state: Arc<AppState>, todos: Arc<TodoStore>) {
    let port = state.config.read().await.http_port;

    let server_state = Arc::new(ServerState {
        app_state: state,
        app_handle: app,
        todo_store: todos,
        pending_questions: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        latest_question_id: tokio::sync::RwLock::new(None),
        latest_question_options: tokio::sync::RwLock::new(Vec::new()),
    });

    let router = Router::new()
        .route("/message", post(routes::post_message))
        .route("/status", post(routes::post_status))
        .route("/mode", get(routes::get_mode))
        .route("/config", get(routes::get_config))
        .route("/todos", get(routes::todo_list))
        .route("/todos/add", post(routes::todo_add))
        .route("/todos/toggle", post(routes::todo_toggle))
        .route("/question", post(routes::post_question))
        .route("/answer", post(routes::post_answer))
        .route("/answer_number", post(routes::answer_by_number))
        .route("/debug/tmux", get(routes::debug_tmux))
        .with_state(server_state);

    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to {}", addr));

    println!("CoS bridge server listening on {}", addr);

    axum::serve(listener, router).await.expect("Server failed");
}
