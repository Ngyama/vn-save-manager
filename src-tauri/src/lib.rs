use tauri::{Manager, State};
use std::sync::{Arc, Mutex};

mod db;
mod watcher;
mod snapshot;

use db::{Database, Game, Snapshot};
use snapshot::SnapshotManager;
use watcher::SaveWatcher;

struct AppState {
    db: Database,
    watcher: Arc<Mutex<SaveWatcher>>,
    snapshot_manager: Arc<Mutex<SnapshotManager>>,
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

    let visual_log_path = Path::new(&game_folder_path).join("visual-log");
    std::fs::create_dir_all(&visual_log_path)
        .map_err(|e| format!("Failed to create visual-log folder: {}", e))?;

    let id = state
        .db
        .add_game(&name, &game_folder_path, &save_folder_path, Some(exe_path.as_str()))
        .map_err(|e| e.to_string())?;
    
    state
        .watcher
        .lock()
        .unwrap()
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
fn load_snapshot_image_base64(image_path: String) -> Result<String, String> {
    use std::fs;

    let bytes = fs::read(&image_path).map_err(|e| e.to_string())?;
    let b64 = base64::encode(bytes);
    Ok(format!("data:image/png;base64,{}", b64))
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
fn restore_snapshot(_state: State<AppState>, _snapshot_id: String, _target_slot_path: Option<String>) -> Result<(), String> {
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle();
            let db = Database::new(handle);
            
            let snapshot_manager = Arc::new(Mutex::new(SnapshotManager::new(handle.clone())));
            
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
                                        let last_time = last_snapshot_time_clone.lock().unwrap();
                                        last_time.elapsed() >= debounce_duration
                                    };
                                    
                                    if !should_process {
                                        println!("Skipping event, too soon after last snapshot: {:?}", event.paths);
                                        continue;
                                    }

                                    println!("Detected change in: {:?}", event.paths);
                                    
                                    if let Some(path) = event.paths.first() {
                                        let sm = sm_clone.lock().unwrap();
                                        match sm.process_save_event(path, last_snapshot_time_clone.clone()) {
                                            Ok(_) => {
                                            },
                                            Err(e) => {
                                            eprintln!("Failed to process save event: {}", e);
                                            }
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

            app.manage(AppState {
                db,
                watcher: watcher_arc,
                snapshot_manager,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            add_game,
            get_games,
            get_snapshots,
            restore_snapshot,
            delete_game,
            load_snapshot_image_base64,
            update_snapshot_note
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
