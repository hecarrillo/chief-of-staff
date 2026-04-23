use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    AtDesk,
    Away,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub from: MessageSource,
    pub text: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub forward_to_telegram: bool,
    #[serde(default)]
    pub delivered: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageSource {
    Hector,
    Cos,
    Telegram,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub bot_token: String,
    pub chat_id: String,
    pub http_port: u16,
    pub vault_path: String,
    /// tmux session name for the main CoS session
    pub cos_session: String,
    /// Working directory for the CoS session
    #[serde(default = "default_cwd")]
    pub cos_cwd: String,
    /// Path to an external framework file (e.g. Obsidian vault).
    /// If set and readable, this takes priority over cos_framework.
    #[serde(default)]
    pub cos_framework_path: String,
    /// Inline system framework prompt — used as fallback when cos_framework_path is empty or unreadable
    #[serde(default = "default_framework")]
    pub cos_framework: String,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            chat_id: String::new(),
            http_port: 7890,
            vault_path: String::new(),
            cos_session: "cos".into(),
            cos_cwd: default_cwd(),
            cos_framework_path: String::new(),
            cos_framework: default_framework(),
        }
    }
}

impl BridgeConfig {
    /// Resolve the effective framework: read from file path if set, else use inline
    pub fn effective_framework(&self) -> String {
        if !self.cos_framework_path.is_empty() {
            if let Ok(content) = std::fs::read_to_string(&self.cos_framework_path) {
                if !content.trim().is_empty() {
                    return content;
                }
            }
            eprintln!(
                "Warning: cos_framework_path '{}' unreadable, falling back to inline framework",
                self.cos_framework_path
            );
        }
        self.cos_framework.clone()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = data_dir().join("config.json");
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }
}

fn default_cwd() -> String {
    dirs::home_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

pub fn default_framework() -> String {
    include_str!("default_framework.md").to_string()
}

/// Returns the app data directory (~/.cos-desktop on all platforms)
pub fn data_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".cos-desktop")
}

#[derive(Debug)]
pub struct AppState {
    pub mode: RwLock<Mode>,
    pub messages: RwLock<Vec<Message>>,
    pub config: RwLock<BridgeConfig>,
    pub last_from_telegram: RwLock<bool>,
    /// Currently targeted tmux window (e.g. "cos:0" or "cos:agents")
    pub target_window: RwLock<String>,
    /// True if today's session was resumed (existing uuid) rather than freshly created.
    /// Used to warn that framework changes won't apply until Renew.
    pub session_was_resumed: RwLock<bool>,
}

impl AppState {
    pub fn new(config: BridgeConfig) -> Arc<Self> {
        let default_target = config.cos_session.clone();
        Self::new_with_target(config, default_target)
    }

    pub fn new_with_target(config: BridgeConfig, target: String) -> Arc<Self> {
        Self::new_full(config, target, false)
    }

    pub fn new_full(config: BridgeConfig, target: String, was_resumed: bool) -> Arc<Self> {
        let cached = Self::load_messages_from_disk();
        Arc::new(Self {
            mode: RwLock::new(Mode::AtDesk),
            messages: RwLock::new(cached),
            config: RwLock::new(config),
            last_from_telegram: RwLock::new(false),
            target_window: RwLock::new(target),
            session_was_resumed: RwLock::new(was_resumed),
        })
    }

    pub async fn add_message(&self, msg: Message) -> bool {
        let mut messages = self.messages.write().await;
        if messages.iter().any(|m| m.id == msg.id) {
            return false;
        }
        messages.push(msg);
        Self::save_messages_to_disk(&messages);
        true
    }

    pub fn load_messages_from_disk() -> Vec<Message> {
        let path = data_dir().join("history/messages.json");
        std::fs::read_to_string(path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    fn save_messages_to_disk(messages: &[Message]) {
        let dir = data_dir().join("history");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("messages.json");
        let to_save: Vec<&Message> = messages
            .iter().rev().take(500)
            .collect::<Vec<_>>().into_iter().rev().collect();
        if let Ok(json) = serde_json::to_string(&to_save) {
            let _ = std::fs::write(path, json);
        }
    }
}
