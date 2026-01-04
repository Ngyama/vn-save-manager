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
        .unwrap_or_else(|| Path::new(&save_folder_path))
        .to_string_lossy()
        .to_string();

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
fn restore_snapshot(_state: State<AppState>, _snapshot_id: String, _target_slot_path: Option<String>) -> Result<(), String> {
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
                    let _ = save_watcher.watch(watch_path);
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
                                            Err(e) => {
                                                eprintln!("Failed to lock last_snapshot_time: {}", e);
                                                continue;
                                            }
                                        }
                                    };
                                    
                                    if !should_process {
                                        println!("Skipping event, too soon after last snapshot: {:?}", event.paths);
                                        continue;
                                    }

                                    println!("Detected change in: {:?}", event.paths);
                                    
                                    if let Some(path) = event.paths.first() {
                                        match sm_clone.lock() {
                                            Ok(sm) => {
                                                match sm.process_save_event(path, last_snapshot_time_clone.clone()) {
                                                    Ok(_) => {},
                                                    Err(e) => eprintln!("Failed to process save event: {}", e),
                                                }
                                            },
                                            Err(e) => eprintln!("Failed to lock snapshot_manager: {}", e),
                                        }
                                    }
                                },
                                _ => {}
                            }
                        },
                        Err(e) => eprintln!("Watch error: {:?}", e),
                    }
                }
            });

            #[cfg(target_os = "windows")]
            let hotkey_manager = {
                let manager = GlobalHotKeyManager::new().map_err(|e| format!("Failed to create hotkey manager: {}", e))?;
                let hotkey = HotKey::new(None, Code::F11);
                manager.register(hotkey.clone()).map_err(|e| format!("Failed to register F11 hotkey: {}", e))?;
                
                println!("F11 hotkey registered successfully");
                
                let hotkey_id = hotkey.id();
                let screenshot_manager_for_hotkey = screenshot_manager.clone();
                let app_handle_for_hotkey = handle.clone();
                let last_screenshot_time = Arc::new(Mutex::new(std::time::Instant::now()));
                let is_capturing = Arc::new(Mutex::new(false));
                let debounce_duration = std::time::Duration::from_millis(2000);
                
                std::thread::spawn(move || {
                    use global_hotkey::GlobalHotKeyEvent;
                    
                    println!("Hotkey event listener thread started");
                    
                    let receiver = GlobalHotKeyEvent::receiver();
                    
                    loop {
                        // Use blocking recv to process one event at a time
                        match receiver.recv() {
                            Ok(event) => {
                                println!("Received hotkey event: id={:?}", event.id);
                                if event.id == hotkey_id {
                                    // CRITICAL: Check flag FIRST before processing
                                    // This prevents race conditions where multiple events are queued
                                    let can_process = {
                                        match (last_screenshot_time.lock(), is_capturing.lock()) {
                                            (Ok(last_time), Ok(capturing)) => {
                                                if *capturing {
                                                    println!("Screenshot already in progress, dropping this event");
                                                    false
                                                } else if last_time.elapsed() < debounce_duration {
                                                    println!("Screenshot debounce active (elapsed: {:?}, required: {:?}), dropping this event", last_time.elapsed(), debounce_duration);
                                                    false
                                                } else {
                                                    true
                                                }
                                            },
                                            _ => {
                                                eprintln!("Failed to lock mutexes");
                                                false
                                            },
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
                                                // Double-check in case another thread set it (shouldn't happen but be safe)
                                                if *capturing {
                                                    println!("Screenshot flag was set while we were checking, dropping event");
                                                    false
                                                } else {
                                                    *last_time = std::time::Instant::now();
                                                    *capturing = true;
                                                    println!("Starting screenshot capture... (is_capturing set to true)");
                                                    true
                                                }
                                            },
                                            _ => {
                                                eprintln!("Failed to lock mutexes for setting flags");
                                                false
                                            },
                                        }
                                    };
                                    
                                    if should_capture {
                                        // Drain any additional pending events for the same hotkey IMMEDIATELY
                                        let mut duplicate_count = 0;
                                        while let Ok(next_event) = receiver.try_recv() {
                                            if next_event.id == hotkey_id {
                                                duplicate_count += 1;
                                                println!("Dropping duplicate hotkey event #{}", duplicate_count);
                                            }
                                        }
                                        
                                        println!("F11 pressed, capturing screenshot...");
                                        let is_capturing_clone = is_capturing.clone();
                                        match screenshot_manager_for_hotkey.lock() {
                                            Ok(sm) => {
                                                match sm.capture_screenshot_for_running_game() {
                                                    Ok(screenshot) => {
                                                        println!("Screenshot captured successfully: {}", screenshot.image_path);
                                                        let _ = app_handle_for_hotkey.emit("screenshot-created", &screenshot);
                                                        // Reset flag AFTER screenshot is complete
                                                        if let Ok(mut capturing) = is_capturing_clone.lock() {
                                                            *capturing = false;
                                                            println!("Screenshot complete, is_capturing reset to false");
                                                        }
                                                    },
                                                    Err(e) => {
                                                        eprintln!("Failed to capture screenshot: {}", e);
                                                        eprintln!("Error details: {:?}", e);
                                                        if let Ok(mut capturing) = is_capturing_clone.lock() {
                                                            *capturing = false;
                                                            println!("Screenshot failed, is_capturing reset to false");
                                                        }
                                                    },
                                                }
                                            },
                                            Err(e) => {
                                                eprintln!("Failed to lock screenshot_manager: {}", e);
                                                if let Ok(mut capturing) = is_capturing_clone.lock() {
                                                    *capturing = false;
                                                    println!("Lock failed, is_capturing reset to false");
                                                }
                                            },
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                eprintln!("Hotkey event receiver error: {:?}", e);
                                break;
                            }
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
            delete_screenshot,
            load_screenshot_image_base64
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
