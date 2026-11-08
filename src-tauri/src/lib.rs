// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::Mutex;
use std::thread;
use tauri::{AppHandle, Emitter};


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

#[derive(Default)]
struct PtyState {
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
}

#[tauri::command]
fn create_shell(
    cwd: Option<String>,
    app: AppHandle,
    state: tauri::State<PtyState>,
) -> Result<(), String> {
    if state.writer.lock().unwrap().is_some() {
        return Ok(());
    }

    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    let mut cmd = CommandBuilder::new("powershell.exe");
    #[cfg(not(target_os = "windows"))]
    let mut cmd = CommandBuilder::new(
        std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string())
    );

    cmd.env("TERM", "xterm-256color");
    if let Some(dir) = cwd {
        cmd.cwd(dir);
    }

    let mut child = pty_pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    let reader = pty_pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pty_pair.master.take_writer().map_err(|e| e.to_string())?;

    *state.writer.lock().unwrap() = Some(writer);
    *state.master.lock().unwrap() = Some(pty_pair.master);

    let app_handle = app.clone();
    thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app_handle.emit("pty-output", text);
                }
                Err(_) => break,
            }
        }
    });

    thread::spawn(move || { let _ = child.wait(); });
    Ok(())
}

#[tauri::command]
fn write_to_pty(data: String, state: tauri::State<PtyState>) -> Result<(), String> {
    let mut guard = state.writer.lock().unwrap();
    let writer = guard.as_mut().ok_or("shell not created")?;
    writer.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())
}

#[tauri::command]
fn resize_pty(rows: u16, cols: u16, state: tauri::State<PtyState>) -> Result<(), String> {
    let guard = state.master.lock().unwrap();
    let master = guard.as_ref().ok_or("shell not created")?;
    master
        .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .manage(PtyState::default())          // 新增
    .invoke_handler(tauri::generate_handler![
        list_dir,
        create_shell,
        write_to_pty,
        resize_pty
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
