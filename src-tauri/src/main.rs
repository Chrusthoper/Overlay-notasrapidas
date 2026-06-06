use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::UNIX_EPOCH;

#[derive(Deserialize)]
struct Config {
    notes_path: String,
}

#[derive(Serialize, Clone)]
struct NoteInfo {
    name: String,
    path: String,
    modified: u64,
    task_count: u32,
    tasks_done: u32,
}

fn resolve_notes_path() -> PathBuf {
    let config_content = include_str!("../config.toml");
    let config: Config = toml::from_str(config_content)
        .expect("Error al parsear config.toml");

    let expanded = shellexpand_path(&config.notes_path);
    PathBuf::from(expanded)
}

fn shellexpand_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}/{}", home.display(), rest);
        }
    }
    path.to_string()
}

fn find_front_matter_end(content: &str) -> usize {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return 0;
    }
    let after_first = &trimmed[3..];
    if let Some(end) = after_first.find("\n---") {
        let closing_line_end = end + 4;
        let prefix_len = content.len() - trimmed.len();
        let bytes = trimmed.as_bytes();
        let mut idx = closing_line_end;
        while idx < bytes.len() && (bytes[idx] == b'\n' || bytes[idx] == b'\r') {
            idx += 1;
        }
        return prefix_len + idx;
    }
    0
}

fn count_tasks(content: &str) -> (u32, u32) {
    let mut total: u32 = 0;
    let mut done: u32 = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
            total += 1;
            done += 1;
        } else if trimmed.starts_with("- [ ]") {
            total += 1;
        }
    }
    (done, total)
}

#[tauri::command]
fn get_recent_notes() -> Vec<NoteInfo> {
    let notes_dir = resolve_notes_path();
    let entries = match fs::read_dir(&notes_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    let mut notes: Vec<NoteInfo> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let content = fs::read_to_string(&path).unwrap_or_default();
        let (tasks_done, task_count) = count_tasks(&content);

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();

        notes.push(NoteInfo {
            name,
            path: path.to_string_lossy().to_string(),
            modified,
            task_count,
            tasks_done,
        });
    }

    notes.sort_by(|a, b| b.modified.cmp(&a.modified));
    notes.truncate(5);
    notes
}

#[tauri::command]
fn read_note(filename: &str) -> Result<String, String> {
    let notes_dir = resolve_notes_path();
    let file_path = notes_dir.join(filename);
    fs::read_to_string(&file_path)
        .map_err(|e| format!("Error al leer archivo: {}", e))
}

#[tauri::command]
fn open_tui() -> Result<String, String> {
    let tui_path = "/home/chs/notas/notas";
    let terminal = std::env::var("TERM")
        .unwrap_or_else(|_| "xterm".to_string());

    let result = Command::new("foot")
        .args(["-e", tui_path])
        .spawn()
        .or_else(|_| {
            Command::new(&terminal)
                .args(["-e", tui_path])
                .spawn()
        })
        .or_else(|_| {
            Command::new("xterm")
                .args(["-e", tui_path])
                .spawn()
        });

    match result {
        Ok(_) => Ok("TUI abierto".to_string()),
        Err(e) => Err(format!("Error al abrir TUI: {}", e)),
    }
}

#[tauri::command]
fn append_to_note(filename: &str, content: &str) -> Result<String, String> {
    let notes_dir = resolve_notes_path();
    let file_path = notes_dir.join(filename);

    if !notes_dir.exists() {
        fs::create_dir_all(&notes_dir)
            .map_err(|e| format!("Error al crear directorio: {}", e))?;
    }

    let new_text = format!("{}\n", content);

    if !file_path.exists() {
        fs::write(&file_path, &new_text)
            .map_err(|e| format!("Error al crear archivo: {}", e))?;
        return Ok("Creado".to_string());
    }

    let raw = fs::read_to_string(&file_path)
        .map_err(|e| format!("Error al leer archivo: {}", e))?;

    let fm_end = find_front_matter_end(&raw);

    if fm_end == 0 {
        let mut updated = raw;
        if !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(&new_text);
        fs::write(&file_path, updated)
            .map_err(|e| format!("Error al escribir archivo: {}", e))?;
    } else {
        let front_matter = &raw[..fm_end];
        let mut body = raw[fm_end..].to_string();
        if !body.ends_with('\n') {
            body.push('\n');
        }
        body.push_str(&new_text);
        let result = format!("{}{}", front_matter, body);
        fs::write(&file_path, result)
            .map_err(|e| format!("Error al escribir archivo: {}", e))?;
    }

    Ok("Agregado".to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            append_to_note,
            get_recent_notes,
            read_note,
            open_tui
        ])
        .setup(|_app| {
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Error al ejecutar la aplicación");
}
