use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IncomingMessage {
    pub text: String,
    #[serde(default)]
    pub telegram: bool,
    #[serde(default)]
    pub image: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatusUpdate {
    pub project: Option<String>,
    pub status: String,
    pub details: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ModeResponse {
    pub mode: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub bot_token: String,
    pub chat_id: String,
    pub vault_path: String,
    pub cos_session: String,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct TodoAddRequest {
    pub text: String,
    pub added_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TodoToggleRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionRequest {
    pub question: String,
    pub options: Vec<String>,
    #[serde(default)]
    pub multi_select: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuestionPayload {
    pub id: String,
    pub question: String,
    pub options: Vec<String>,
    pub multi_select: bool,
}

#[derive(Debug, Deserialize)]
pub struct QuestionAnswer {
    pub id: String,
    pub selected: Vec<String>,
}
