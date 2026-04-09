use crate::state::data_dir;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    pub text: String,
    pub done: bool,
    pub date: String,
    pub added_by: String,
}

#[derive(Debug)]
pub struct TodoStore {
    pub items: RwLock<Vec<Todo>>,
}

impl TodoStore {
    pub fn new() -> Arc<Self> {
        let items = load_today();
        Arc::new(Self {
            items: RwLock::new(items),
        })
    }

    pub async fn add(&self, text: &str, added_by: &str) -> Todo {
        let todo = Todo {
            id: uuid::Uuid::new_v4().to_string(),
            text: text.to_string(),
            done: false,
            date: today_str(),
            added_by: added_by.to_string(),
        };
        self.items.write().await.push(todo.clone());
        self.save().await;
        todo
    }

    pub async fn toggle(&self, id: &str) -> Option<Todo> {
        let mut items = self.items.write().await;
        let item = items.iter_mut().find(|t| t.id == id)?;
        item.done = !item.done;
        let result = item.clone();
        drop(items);
        self.save().await;
        Some(result)
    }

    pub async fn remove(&self, id: &str) -> bool {
        let mut items = self.items.write().await;
        let len_before = items.len();
        items.retain(|t| t.id != id);
        let removed = items.len() < len_before;
        drop(items);
        if removed { self.save().await; }
        removed
    }

    pub async fn list(&self) -> Vec<Todo> {
        self.items.read().await.clone()
    }

    pub async fn format_for_telegram(&self) -> String {
        let items = self.items.read().await;
        if items.is_empty() {
            return "No tasks for today.".to_string();
        }
        let mut lines = vec![format!("Tasks for {}:", today_str())];
        for (i, t) in items.iter().enumerate() {
            let check = if t.done { "x" } else { " " };
            lines.push(format!("{}. [{}] {}", i + 1, check, t.text));
        }
        let done = items.iter().filter(|t| t.done).count();
        lines.push(format!("\n{}/{} done", done, items.len()));
        lines.join("\n")
    }

    async fn save(&self) {
        let items = self.items.read().await;
        let dir = data_dir().join("todos");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("{}.json", today_str()));
        if let Ok(json) = serde_json::to_string_pretty(&*items) {
            let _ = std::fs::write(path, json);
        }
    }
}

fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn load_today() -> Vec<Todo> {
    let path = data_dir().join("todos").join(format!("{}.json", today_str()));
    std::fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}
