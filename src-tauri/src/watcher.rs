use crate::state::{AppState, Message, MessageSource};
use crate::tmux;
use chrono::Utc;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::time::{interval, Duration};

/// Polls the tmux pane for Claude responses and forwards them to the app.
pub async fn start_response_watcher(app: AppHandle, state: Arc<AppState>) {
    let mut ticker = interval(Duration::from_secs(2));
    let mut last_snapshot = String::new();

    loop {
        ticker.tick().await;

        let target = state.target_window.read().await.clone();
        let content = match tmux::capture_pane(&target, 50) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if content == last_snapshot {
            continue;
        }

        // First poll — just set baseline, don't emit
        if last_snapshot.is_empty() {
            last_snapshot = content;
            continue;
        }

        // Find new lines by diffing against previous snapshot
        let old_lines: Vec<&str> = last_snapshot.lines().collect();
        let new_lines: Vec<&str> = content.lines().collect();

        let mut new_start = 0;
        if let Some(last_old) = old_lines.last() {
            for (i, line) in new_lines.iter().enumerate().rev() {
                if line == last_old {
                    new_start = i + 1;
                    break;
                }
            }
        }

        // Extract Claude response lines (prefixed with ⏺)
        let response_text: Vec<String> = new_lines[new_start..]
            .iter()
            .filter(|line| line.trim().starts_with('\u{23FA}'))
            .map(|line| line.trim().trim_start_matches('\u{23FA}').trim().to_string())
            .collect();

        // Update snapshot AFTER we've extracted what we need
        last_snapshot = content;

        if response_text.is_empty() {
            continue;
        }

        let text = response_text.join("\n");
        if text.is_empty() {
            continue;
        }

        let msg = Message {
            id: uuid::Uuid::new_v4().to_string(),
            from: MessageSource::Cos,
            text,
            timestamp: Utc::now(),
            forward_to_telegram: false,
            delivered: true,
            image: None,
            reply_to: None,
            reply_preview: None,
        };

        let is_new = state.add_message(msg.clone()).await;
        if is_new {
            let _ = app.emit("new-message", &msg);

            let is_away = *state.mode.read().await == crate::state::Mode::Away;
            let last_from_tg = *state.last_from_telegram.read().await;
            if is_away || last_from_tg {
                let _ = crate::telegram::send_telegram(&state, &msg.text).await;
            }
        }
    }
}
