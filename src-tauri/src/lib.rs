// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};


#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}


#[derive(Serialize)]
struct EntryInfo {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
}

#[derive(Serialize)]
struct Listing {
    current: String,
    parent: Option<String>,
    entries: Vec<EntryInfo>,
}

// Windows 的 canonicalize 会返回 \\?\D:\... 前缀，显示时去掉更友好
fn clean_path(p: &Path) -> String {
    let s = p.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    {
        s.strip_prefix(r"\\?\").map(|x| x.to_string()).unwrap_or(s)
    }
    #[cfg(not(target_os = "windows"))]
    { s }
}

#[tauri::command]
fn list_dir(path: Option<String>) -> Result<Listing, String> {
    let target: PathBuf = match path {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => std::env::current_dir().map_err(|e| e.to_string())?,
    };

    let target = fs::canonicalize(&target)
        .map_err(|e| format!("无法访问 {}: {}", target.display(), e))?;

    let mut entries = Vec::new();
    for entry in fs::read_dir(&target).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        entries.push(EntryInfo {
            name: entry.file_name().to_string_lossy().to_string(),
            path: clean_path(&entry.path()),
            is_dir: meta.is_dir(),
            size: meta.len(),
        });
    }

    // 排序：目录在前，然后按名称不区分大小写
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let parent = target.parent().map(clean_path);

    Ok(Listing {
        current: clean_path(&target),
        parent,
        entries,
    })
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![list_dir])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
