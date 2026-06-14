use chrono::{DateTime, Utc};
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: String,
    pub content: String,
    pub copied_at: DateTime<Utc>,
    pub pinned: bool,
}

pub fn start_watcher(tx: Sender<ClipboardItem>) {
    thread::spawn(move || {
        let mut clipboard = match arboard::Clipboard::new() {
            Ok(cb) => cb,
            Err(_) => return,
        };
        let mut last_seen = String::new();

        loop {
            if let Ok(text) = clipboard.get_text() {
                if !text.is_empty() && text != last_seen {
                    let item = ClipboardItem {
                        id: uuid::Uuid::new_v4().to_string(),
                        content: text.clone(),
                        copied_at: Utc::now(),
                        pinned: false,
                    };
                    last_seen = text;
                    let _ = tx.send(item);
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}
