use tauri::{Manager, State};
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
fn delete_game(state: State<AppState>, game_id: String) -> Result<(), String> {
    state.db.delete_game(&game_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_snapshot(state: State<AppState>, snapshot_id: String) -> Result<(), String> {
    state.db.delete_snapshot(&snapshot_id).map_err(|e| e.to_string())
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
    state.db.delete_screenshot(&screenshot_id).map_err(|e| e.to_string())
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
            {
                let manager = GlobalHotKeyManager::new().map_err(|e| format!("Failed to create hotkey manager: {}", e))?;
                let hotkey = HotKey::new(None, Code::F11);
                manager.register(hotkey).map_err(|e| format!("Failed to register F11 hotkey: {}", e))?;
                
                let hotkey_id = hotkey.id();
                let screenshot_manager_for_hotkey = screenshot_manager.clone();
                
                std::thread::spawn(move || {
                    use global_hotkey::GlobalHotKeyEvent;
                    
                    loop {
                        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                            if event.id == hotkey_id {
                                println!("F11 pressed, capturing screenshot...");
                                match screenshot_manager_for_hotkey.lock() {
                                    Ok(sm) => {
                                        match sm.capture_screenshot_for_running_game() {
                                            Ok(_) => println!("Screenshot captured successfully"),
                                            Err(e) => eprintln!("Failed to capture screenshot: {}", e),
                                        }
                                    },
                                    Err(e) => eprintln!("Failed to lock screenshot_manager: {}", e),
                                }
                            }
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                });
            }

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
            capture_screenshot,
            get_screenshots,
            update_screenshot_note,
            delete_screenshot,
            load_screenshot_image_base64
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
