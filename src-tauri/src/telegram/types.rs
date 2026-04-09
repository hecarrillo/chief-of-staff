use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TelegramResponse {
    pub ok: bool,
    pub result: Vec<TelegramUpdate>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub from: Option<TelegramUser>,
    pub chat: TelegramChat,
    pub text: Option<String>,
    pub caption: Option<String>,
    pub photo: Option<Vec<TelegramPhotoSize>>,
    pub reply_to_message: Option<Box<TelegramMessage>>,
    pub date: i64,
}

#[derive(Debug, Deserialize)]
pub struct TelegramPhotoSize {
    pub file_id: String,
    pub file_unique_id: String,
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Deserialize)]
pub struct TelegramFile {
    pub file_id: String,
    pub file_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramFileResponse {
    pub ok: bool,
    pub result: Option<TelegramFile>,
}

#[derive(Debug, Deserialize)]
pub struct TelegramUser {
    pub id: i64,
    pub first_name: String,
}

#[derive(Debug, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageResponse {
    pub ok: bool,
}
