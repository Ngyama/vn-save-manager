use tauri::{Manager, State, Emitter};
use std::sync::{Arc, Mutex};

mod db;
mod watcher;
mod snapshot;
mod screenshot;

use db::{Database, Game, Snapshot, Screenshot};
use snapshot::SnapshotManager;
use screenshot::ScreenshotManager;
use watcher::SaveWatcher;

#[cfg(target_os = "windows")]
use global_hotkey::{
    hotkey::{HotKey, Code},
    GlobalHotKeyManager,
};

#[cfg(target_os = "windows")]
struct AppState {
    db: Database,
    watcher: Arc<Mutex<SaveWatcher>>,
    snapshot_manager: Arc<Mutex<SnapshotManager>>,
    screenshot_manager: Arc<Mutex<ScreenshotManager>>,
    hotkey_manager: Arc<Mutex<Option<GlobalHotKeyManager>>>,
}

#[cfg(not(target_os = "windows"))]
struct AppState {
    db: Database,
    watcher: Arc<Mutex<SaveWatcher>>,
    snapshot_manager: Arc<Mutex<SnapshotManager>>,
    screenshot_manager: Arc<Mutex<ScreenshotManager>>,
}

#[tauri::command]
fn add_game(
    _app_handle: tauri::AppHandle,
    state: State<AppState>,
    name: String,
    save_folder_path: String,
    exe_path: String,
) -> Result<String, String> {
    use std::path::Path;

    let exe_path_obj = Path::new(&exe_path);
    let game_folder_path = exe_path_obj
        .parent()
        .ok_or_else(|| "无法获取游戏执行文件的父目录".to_string())?
        .to_string_lossy()
        .to_string();
    
    if !Path::new(&save_folder_path).exists() {
        return Err(format!("存档文件夹不存在: {}", save_folder_path));
    }
    if !Path::new(&exe_path).exists() {
        return Err(format!("游戏执行文件不存在: {}", exe_path));
    }
    
    let existing_games = state.db.get_games().map_err(|e| e.to_string())?;
    
    // Check for duplicate name
    if existing_games.iter().any(|g| g.name == name) {
        return Err(format!("游戏名称 \"{}\" 已存在", name));
    }
    
    // Check for duplicate exe_path
    let normalized_exe = Path::new(&exe_path).canonicalize()
        .map_err(|_| format!("无法规范化路径: {}", exe_path))?
        .to_string_lossy().to_string();
    
    if let Some(dup_game) = existing_games.iter().find(|g| {
        if let Some(ref existing_exe) = g.exe_path {
            if let Ok(existing_normalized) = Path::new(existing_exe).canonicalize() {
                existing_normalized.to_string_lossy() == normalized_exe
            } else {
                false
            }
        } else {
            false
        }
    }) {
        return Err(format!("游戏执行文件已被游戏 \"{}\" 使用", dup_game.name));
    }
    
    // Check for duplicate save_folder_path
    let normalized_save = Path::new(&save_folder_path).canonicalize()
        .map_err(|_| format!("无法规范化路径: {}", save_folder_path))?
        .to_string_lossy().to_string();
    
    if let Some(dup_game) = existing_games.iter().find(|g| {
        if let Some(ref existing_save) = g.save_folder_path {
            if let Ok(existing_normalized) = Path::new(existing_save).canonicalize() {
                existing_normalized.to_string_lossy() == normalized_save
            } else {
                false
            }
        } else {
            false
        }
    }) {
        return Err(format!("存档文件夹已被游戏 \"{}\" 使用", dup_game.name));
    }

    let visual_logger_path = Path::new(&game_folder_path).join("visual-logger");
    let screenshots_dir = visual_logger_path.join("screenshots");
    let snapshots_dir = visual_logger_path.join("snapshots");
    std::fs::create_dir_all(&screenshots_dir)
        .map_err(|e| format!("Failed to create screenshots folder: {}", e))?;
    std::fs::create_dir_all(&snapshots_dir)
        .map_err(|e| format!("Failed to create snapshots folder: {}", e))?;

    let id = state
        .db
        .add_game(&name, &game_folder_path, &save_folder_path, Some(exe_path.as_str()))
        .map_err(|e| e.to_string())?;
    
    state
        .watcher
        .lock()
        .map_err(|e| format!("Failed to lock watcher: {}", e))?
        .watch(&save_folder_path)
        .map_err(|e| e.to_string())?;

    Ok(id)
}

#[tauri::command]
fn get_games(state: State<AppState>) -> Result<Vec<Game>, String> {
    state.db.get_games().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_game_stats(state: State<AppState>, game_id: String) -> Result<(usize, usize), String> {
    use rusqlite::params;
    let conn = state.db.connect().map_err(|e| e.to_string())?;
    
    let snapshot_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshots WHERE game_id = ?1",
            params![game_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    
    let screenshot_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM screenshots WHERE game_id = ?1",
            params![game_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    
    Ok((snapshot_count, screenshot_count))
}

#[tauri::command]
fn get_snapshots(state: State<AppState>, game_id: String) -> Result<Vec<Snapshot>, String> {
    state.db.get_snapshots(&game_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_screenshot_image_base64(image_path: String) -> Result<String, String> {
    use std::fs;
    use base64::{Engine as _, engine::general_purpose};

    let bytes = fs::read(&image_path).map_err(|e| e.to_string())?;
    let b64 = general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/png;base64,{}", b64))
}

#[tauri::command]
fn load_snapshot_image_base64(image_path: String) -> Result<String, String> {
    load_screenshot_image_base64(image_path)
}

#[tauri::command]
fn update_snapshot_note(state: State<AppState>, snapshot_id: String, note: String) -> Result<(), String> {
    state.db.update_snapshot_note(&snapshot_id, &note).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_snapshot_name(state: State<AppState>, snapshot_id: String, name: String) -> Result<(), String> {
    state.db.update_snapshot_name(&snapshot_id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_game(state: State<AppState>, game_id: String, delete_visual_logger: bool) -> Result<(), String> {
    state.db.delete_game(&game_id, delete_visual_logger).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_snapshot(state: State<AppState>, snapshot_id: String) -> Result<(), String> {
    use std::fs;
    
    // Get snapshot before deleting
    let snapshot = state.db.get_snapshot(&snapshot_id).map_err(|e| e.to_string())?;
    
    // Delete from database
    state.db.delete_snapshot(&snapshot_id).map_err(|e| e.to_string())?;
    
    // Delete backup directory
    let backup_path = std::path::Path::new(&snapshot.backup_save_path);
    if backup_path.exists() {
        if backup_path.is_dir() {
            fs::remove_dir_all(backup_path).map_err(|e| format!("Failed to delete snapshot directory: {}", e))?;
        } else {
            fs::remove_file(backup_path).map_err(|e| format!("Failed to delete snapshot file: {}", e))?;
        }
    }
    
    Ok(())
}

#[tauri::command]
fn restore_snapshot(state: State<AppState>, snapshot_id: String) -> Result<(), String> {
    use std::fs;
    
    let snapshot = state.db.get_snapshot(&snapshot_id).map_err(|e| e.to_string())?;
    
    let backup_path = std::path::Path::new(&snapshot.backup_save_path);
    let original_path = std::path::Path::new(&snapshot.original_save_path);
    
    if !backup_path.exists() {
        return Err(format!("备份文件不存在: {}", snapshot.backup_save_path));
    }
    
    if backup_path.is_dir() {
        return Err("备份路径是目录，无法恢复。请确保备份路径是文件。".to_string());
    }
    
    if let Some(parent) = original_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("无法创建目标目录: {}", e))?;
    }
    
    fs::copy(backup_path, original_path)
        .map_err(|e| format!("无法复制备份文件到原始路径: {}", e))?;
    
    Ok(())
}

#[tauri::command]
fn capture_screenshot(state: State<AppState>, game_id: String) -> Result<Screenshot, String> {
    state
        .screenshot_manager
        .lock()
        .map_err(|e| format!("Failed to lock screenshot_manager: {}", e))?
        .capture_screenshot(&game_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_screenshots(state: State<AppState>, game_id: String) -> Result<Vec<Screenshot>, String> {
    state.db.get_screenshots(&game_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_screenshot_note(state: State<AppState>, screenshot_id: String, note: String) -> Result<(), String> {
    state.db.update_screenshot_note(&screenshot_id, &note).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_screenshot_name(state: State<AppState>, screenshot_id: String, name: String) -> Result<(), String> {
    state.db.update_screenshot_name(&screenshot_id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_screenshot(state: State<AppState>, screenshot_id: String) -> Result<(), String> {
    use std::fs;
    
    // Get screenshot before deleting
    let screenshot = state.db.get_screenshot(&screenshot_id).map_err(|e| e.to_string())?;
    
    // Delete from database
    state.db.delete_screenshot(&screenshot_id).map_err(|e| e.to_string())?;
    
    // Delete image file
    let image_path = std::path::Path::new(&screenshot.image_path);
    if image_path.exists() {
        fs::remove_file(image_path).map_err(|e| format!("Failed to delete screenshot file: {}", e))?;
    }
    
    Ok(())
}

#[tauri::command]
fn batch_delete_snapshots(state: State<AppState>, snapshot_ids: Vec<String>) -> Result<(), String> {
    use std::fs;
    
    let mut errors = Vec::new();
    
    for snapshot_id in snapshot_ids {
        match state.db.get_snapshot(&snapshot_id) {
            Ok(snapshot) => {
                // Delete from database
                if let Err(e) = state.db.delete_snapshot(&snapshot_id) {
                    errors.push(format!("删除快照 {} 失败: {}", snapshot.name, e));
                    continue;
                }
                
                // Delete backup directory
                let backup_path = std::path::Path::new(&snapshot.backup_save_path);
                if backup_path.exists() {
                    let result = if backup_path.is_dir() {
                        fs::remove_dir_all(backup_path)
                    } else {
                        fs::remove_file(backup_path).map(|_| ())
                    };
                    if let Err(e) = result {
                        errors.push(format!("删除快照 {} 的备份文件失败: {}", snapshot.name, e));
                    }
                }
            },
            Err(e) => {
                errors.push(format!("获取快照失败: {}", e));
            }
        }
    }
    
    if !errors.is_empty() {
        return Err(format!("批量删除完成，但有一些错误:\n{}", errors.join("\n")));
    }
    
    Ok(())
}

#[tauri::command]
fn batch_delete_screenshots(state: State<AppState>, screenshot_ids: Vec<String>) -> Result<(), String> {
    use std::fs;
    
    let mut errors = Vec::new();
    
    for screenshot_id in screenshot_ids {
        match state.db.get_screenshot(&screenshot_id) {
            Ok(screenshot) => {
                // Delete from database
                if let Err(e) = state.db.delete_screenshot(&screenshot_id) {
                    errors.push(format!("删除截图失败: {}", e));
                    continue;
                }
                
                // Delete image file
                let image_path = std::path::Path::new(&screenshot.image_path);
                if image_path.exists() {
                    if let Err(e) = fs::remove_file(image_path) {
                        errors.push(format!("删除截图文件失败: {}", e));
                    }
                }
            },
            Err(e) => {
                errors.push(format!("获取截图失败: {}", e));
            }
        }
    }
    
    if !errors.is_empty() {
        return Err(format!("批量删除完成，但有一些错误:\n{}", errors.join("\n")));
    }
    
    Ok(())
}

#[tauri::command]
fn batch_export_screenshots(state: State<AppState>, screenshot_ids: Vec<String>, export_dir: String) -> Result<usize, String> {
    use std::fs;
    use std::path::Path;
    
    let export_path = Path::new(&export_dir);
    if !export_path.exists() {
        return Err(format!("导出目录不存在: {}", export_dir));
    }
    if !export_path.is_dir() {
        return Err(format!("导出路径不是目录: {}", export_dir));
    }
    
    let mut exported_count = 0;
    let mut errors = Vec::new();
    
    for screenshot_id in screenshot_ids {
        match state.db.get_screenshot(&screenshot_id) {
            Ok(screenshot) => {
                let source_path = Path::new(&screenshot.image_path);
                if !source_path.exists() {
                    errors.push(format!("截图文件不存在: {}", screenshot.name));
                    continue;
                }
                
                // Generate export filename from screenshot name or use original filename
                let source_filename = source_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("screenshot.png");
                
                // Sanitize filename: remove invalid characters
                let safe_name = screenshot.name
                    .chars()
                    .map(|c| if ":<>\"|?*\\/".contains(c) { '_' } else { c })
                    .collect::<String>();
                
                // Try to preserve original extension
                let extension = source_path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("png");
                
                let export_filename = if safe_name.trim().is_empty() {
                    source_filename.to_string()
                } else {
                    format!("{}.{}", safe_name.trim(), extension)
                };
                
                let dest_path = export_path.join(&export_filename);
                
                // Handle duplicate filenames
                let mut final_dest_path = dest_path.clone();
                let mut counter = 1;
                while final_dest_path.exists() {
                    let stem = Path::new(&export_filename)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("screenshot");
                    let ext = Path::new(&export_filename)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("png");
                    final_dest_path = export_path.join(format!("{} ({}).{}", stem, counter, ext));
                    counter += 1;
                }
                
                match fs::copy(source_path, &final_dest_path) {
                    Ok(_) => {
                        exported_count += 1;
                    },
                    Err(e) => {
                        errors.push(format!("导出截图 {} 失败: {}", screenshot.name, e));
                    }
                }
            },
            Err(e) => {
                errors.push(format!("获取截图失败: {}", e));
            }
        }
    }
    
    if exported_count == 0 && !errors.is_empty() {
        return Err(format!("导出失败:\n{}", errors.join("\n")));
    }
    
    if !errors.is_empty() {
        return Err(format!("成功导出 {} 张截图，但有 {} 个错误:\n{}", 
            exported_count, 
            errors.len(),
            errors.join("\n")
        ));
    }
    
    Ok(exported_count)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle();
            let db = Database::new(&handle);
            
            let snapshot_manager = Arc::new(Mutex::new(SnapshotManager::new(handle.clone())));
            let screenshot_manager = Arc::new(Mutex::new(ScreenshotManager::new(handle.clone())));
            
            let (mut save_watcher, rx) = SaveWatcher::new(handle.clone());
            
            if let Ok(games) = db.get_games() {
                for game in games {
                    let watch_path = game
                        .save_folder_path
                        .as_deref()
                        .unwrap_or(&game.game_folder_path);
                    if let Err(e) = save_watcher.watch(watch_path) {
                                        // Failed to watch game folder
                    }
                }
            }

            let watcher_arc = Arc::new(Mutex::new(save_watcher));

            let sm_clone = snapshot_manager.clone();
            let last_snapshot_time = Arc::new(Mutex::new(std::time::Instant::now()));
            let last_snapshot_time_clone = last_snapshot_time.clone();
            
            std::thread::spawn(move || {
                let debounce_duration = std::time::Duration::from_secs(2);

                for res in rx {
                    match res {
                        Ok(event) => {
                             match event.kind {
                                notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                                    let should_process = {
                                        match last_snapshot_time_clone.lock() {
                                            Ok(last_time) => last_time.elapsed() >= debounce_duration,
                                            Err(_) => continue,
                                        }
                                    };
                                    
                                    if !should_process {
                                        continue;
                                    }
                                    
                                    if let Some(path) = event.paths.first() {
                                        match sm_clone.lock() {
                                            Ok(sm) => {
                                                match sm.process_save_event(path, last_snapshot_time_clone.clone()) {
                                                    Ok(_) => {},
                                                    Err(_) => {},
                                                }
                                            },
                                            Err(_) => {},
                                        }
                                    }
                                },
                                _ => {}
                            }
                        },
                        Err(_) => {},
                    }
                }
            });

            #[cfg(target_os = "windows")]
            let hotkey_manager = {
                let manager = GlobalHotKeyManager::new().map_err(|e| format!("Failed to create hotkey manager: {}", e))?;
                let hotkey = HotKey::new(None, Code::F11);
                manager.register(hotkey.clone()).map_err(|e| format!("Failed to register F11 hotkey: {}", e))?;
                
                let hotkey_id = hotkey.id();
                let screenshot_manager_for_hotkey = screenshot_manager.clone();
                let app_handle_for_hotkey = handle.clone();
                let last_screenshot_time = Arc::new(Mutex::new(std::time::Instant::now()));
                let is_capturing = Arc::new(Mutex::new(false));
                let debounce_duration = std::time::Duration::from_millis(2000);
                
                std::thread::spawn(move || {
                    use global_hotkey::GlobalHotKeyEvent;
                    
                    let receiver = GlobalHotKeyEvent::receiver();
                    
                    loop {
                        // Use blocking recv to process one event at a time
                        match receiver.recv() {
                            Ok(event) => {
                                if event.id == hotkey_id {
                                    // CRITICAL: Check flag FIRST before processing
                                    // This prevents race conditions where multiple events are queued
                                    let can_process = {
                                        match (last_screenshot_time.lock(), is_capturing.lock()) {
                                            (Ok(last_time), Ok(capturing)) => {
                                                if *capturing {
                                                    false
                                                } else if last_time.elapsed() < debounce_duration {
                                                    false
                                                } else {
                                                    true
                                                }
                                            },
                                            _ => false,
                                        }
                                    };
                                    
                                    if !can_process {
                                        // Skip this event and continue
                                        continue;
                                    }
                                    
                                    // Now set the flags atomically
                                    let should_capture = {
                                        match (last_screenshot_time.lock(), is_capturing.lock()) {
                                            (Ok(mut last_time), Ok(mut capturing)) => {
                                                if *capturing {
                                                    false
                                                } else {
                                                    *last_time = std::time::Instant::now();
                                                    *capturing = true;
                                                    true
                                                }
                                            },
                                            _ => false,
                                        }
                                    };
                                    
                                    if should_capture {
                                        while let Ok(next_event) = receiver.try_recv() {
                                            if next_event.id == hotkey_id {
                                                // Drop duplicate events
                                            }
                                        }
                                        let is_capturing_clone = is_capturing.clone();
                                        match screenshot_manager_for_hotkey.lock() {
                                            Ok(sm) => {
                                                match sm.capture_screenshot_for_running_game() {
                                                    Ok(screenshot) => {
                                                        let _ = app_handle_for_hotkey.emit("screenshot-created", &screenshot);
                                                        if let Ok(mut capturing) = is_capturing_clone.lock() {
                                                            *capturing = false;
                                                        }
                                                    },
                                                    Err(_) => {
                                                        if let Ok(mut capturing) = is_capturing_clone.lock() {
                                                            *capturing = false;
                                                        }
                                                    },
                                                }
                                            },
                                            Err(_) => {
                                                if let Ok(mut capturing) = is_capturing_clone.lock() {
                                                    *capturing = false;
                                                }
                                            },
                                        }
                                    }
                                }
                            },
                            Err(_) => break,
                        }
                    }
                });
                
                Arc::new(Mutex::new(Some(manager)))
            };

            #[cfg(target_os = "windows")]
            app.manage(AppState {
                db,
                watcher: watcher_arc,
                snapshot_manager,
                screenshot_manager,
                hotkey_manager,
            });

            #[cfg(not(target_os = "windows"))]
            app.manage(AppState {
                db,
                watcher: watcher_arc,
                snapshot_manager,
                screenshot_manager,
            });

            Ok(())
        })
            .invoke_handler(tauri::generate_handler![
            add_game,
            get_games,
            get_game_stats,
            get_snapshots,
            restore_snapshot,
            delete_game,
            delete_snapshot,
            load_snapshot_image_base64,
            update_snapshot_note,
            update_snapshot_name,
            capture_screenshot,
            get_screenshots,
            update_screenshot_note,
            update_screenshot_name,
            delete_screenshot,
            load_screenshot_image_base64,
            batch_delete_snapshots,
            batch_delete_screenshots,
            batch_export_screenshots
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("应用运行错误: {}", e);
            std::process::exit(1);
        });
}
