pub mod markdown;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use tauri::AppHandle;
use tauri::Emitter;

#[derive(Debug, Clone, serde::Serialize)]
pub struct VaultChange {
    pub path: String,
    pub kind: String,
}

pub fn start_watching(app: AppHandle, vault_path: &str) {
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .expect("Failed to create vault watcher");

    let vault = Path::new(vault_path);
    if vault.exists() {
        watcher
            .watch(vault, RecursiveMode::Recursive)
            .expect("Failed to watch vault path");
    }

    std::thread::spawn(move || {
        let _watcher = watcher;
        for event in rx {
            let Ok(event) = event else { continue };
            for path in &event.paths {
                let ext = path.extension().and_then(|e| e.to_str());
                if ext != Some("md") {
                    continue;
                }
                let change = VaultChange {
                    path: path.to_string_lossy().into_owned(),
                    kind: format!("{:?}", event.kind),
                };
                let _ = app.emit("vault-change", &change);
            }
        }
    });
}

pub fn read_file(path: &str) -> Result<VaultFile, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("File not found: {}", path));
    }
    let content = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
    let (frontmatter, body) = markdown::parse_frontmatter(&content);
    Ok(VaultFile {
        path: path.to_string(),
        name: p.file_stem().unwrap_or_default().to_string_lossy().into_owned(),
        frontmatter,
        body,
    })
}

pub fn read_vault_files(vault_path: &str, category: &str) -> Result<Vec<VaultFile>, String> {
    let dir = Path::new(vault_path).join(category);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let (frontmatter, body) = markdown::parse_frontmatter(&content);
        files.push(VaultFile {
            path: path.to_string_lossy().into_owned(),
            name: path.file_stem().unwrap_or_default().to_string_lossy().into_owned(),
            frontmatter,
            body,
        });
    }

    files.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(files)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VaultFile {
    pub path: String,
    pub name: String,
    pub frontmatter: std::collections::HashMap<String, String>,
    pub body: String,
}
