use crate::db::{Database, Snapshot};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::AppHandle;
use tauri::Emitter;
use chrono::Utc;
use uuid::Uuid;

pub struct SnapshotManager {
    db: Database,
    app_handle: AppHandle,
}

impl SnapshotManager {
    pub fn new(app_handle: AppHandle) -> Self {
        let db = Database::new(&app_handle);
        Self {
            db,
            app_handle,
        }
    }

    pub fn process_save_event(&self, changed_file_path: &PathBuf, last_snapshot_time: Arc<Mutex<Instant>>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ext) = changed_file_path.extension() {
            if ext.to_string_lossy().to_lowercase() != "dat" {
                return Ok(());
            }
        } else {
            return Ok(());
        }

        let games = self.db.get_games()?;
        let mut target_game = None;

        for game in games {
            let mut matched = false;

            if let Some(ref save_folder) = game.save_folder_path {
                let base = Path::new(save_folder);
                if changed_file_path.starts_with(base) {
                    matched = true;
                }
            }

            if !matched {
                let base = Path::new(&game.game_folder_path);
                if changed_file_path.starts_with(base) {
                    matched = true;
                }
            }

            if matched {
                target_game = Some(game);
                break;
            }
        }

        let game = match target_game {
            Some(g) => g,
            None => {
                println!("File {:?} does not belong to any known game.", changed_file_path);
                return Ok(());
            }
        };

        let game_folder = PathBuf::from(&game.game_folder_path);
        let snapshots_dir = game_folder.join("visual-logger").join("snapshots");
        fs::create_dir_all(&snapshots_dir)?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let snapshot_folder_name = format!("{}_{}", game.name, timestamp);
        let snapshot_folder = snapshots_dir.join(&snapshot_folder_name);
        fs::create_dir_all(&snapshot_folder)?;

        let uuid = Uuid::new_v4().to_string();
        
        std::thread::sleep(std::time::Duration::from_millis(200));

        let dat_file_name = changed_file_path.file_name()
            .ok_or("Invalid file name")?
            .to_str()
            .ok_or("Invalid file name encoding")?;
        let dat_backup_path = snapshot_folder.join(dat_file_name);
        fs::copy(changed_file_path, &dat_backup_path)?;

        let default_name = format!("快照 {}", Utc::now().format("%Y-%m-%d %H:%M:%S"));

        let metadata = serde_json::json!({
            "id": uuid,
            "game_id": game.id,
            "game_name": game.name,
            "timestamp": Utc::now().to_rfc3339(),
            "dat_file": dat_file_name,
            "dat_path": changed_file_path.to_string_lossy().to_string(),
        });
        let metadata_path = snapshot_folder.join("metadata.json");
        fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)?;

        let context_path = snapshot_folder.join("context.txt");
        fs::write(&context_path, "").ok();

        let note_path = snapshot_folder.join("note.txt");
        fs::write(&note_path, "").ok();

        let snapshot = Snapshot {
            id: uuid,
            game_id: game.id,
            name: default_name,
            original_save_path: changed_file_path.to_string_lossy().to_string(),
            backup_save_path: snapshot_folder.to_string_lossy().to_string(),
            note: None,
            created_at: Utc::now().to_rfc3339(),
        };

        self.db.add_snapshot(&snapshot)?;
        
        {
            match last_snapshot_time.lock() {
                Ok(mut last_time) => *last_time = Instant::now(),
                Err(e) => eprintln!("Failed to update last_snapshot_time: {}", e),
            }
        }
        
        self.app_handle.emit("snapshot-created", &snapshot)?;

        Ok(())
    }
}
