use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub exe_path: Option<String>,
    pub game_folder_path: String,
    pub save_folder_path: Option<String>,
    pub cover_image: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub game_id: String,
    pub original_save_path: String,
    pub backup_save_path: String,
    pub text_content: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Screenshot {
    pub id: String,
    pub game_id: String,
    pub image_path: String,
    pub note: Option<String>,
    pub created_at: String,
}

pub struct Database {
    db_path: PathBuf,
}

impl Database {
    pub fn new(app_handle: &tauri::AppHandle) -> Self {
        let app_data_dir = app_handle.path().app_data_dir().expect("failed to get app data dir");
        std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
        let db_path = app_data_dir.join("galgame_saves.db");
        
        let db = Self { db_path };
        db.init().expect("failed to init db");
        db
    }

    fn connect(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
    }

    pub fn init(&self) -> Result<()> {
        let conn = self.connect()?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS games (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                exe_path TEXT,
                save_folder_path TEXT,
                cover_image TEXT
            )",
            [],
        )?;
        
        let _ = conn.execute(
            "ALTER TABLE games ADD COLUMN game_folder_path TEXT",
            [],
        );
        
        let _ = conn.execute(
            "UPDATE games SET game_folder_path = save_folder_path WHERE game_folder_path IS NULL AND save_folder_path IS NOT NULL",
            [],
        );

        conn.execute(
            "CREATE TABLE IF NOT EXISTS snapshots (
                id TEXT PRIMARY KEY,
                game_id TEXT NOT NULL,
                original_save_path TEXT NOT NULL,
                backup_save_path TEXT NOT NULL,
                text_content TEXT,
                note TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(game_id) REFERENCES games(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS screenshots (
                id TEXT PRIMARY KEY,
                game_id TEXT NOT NULL,
                image_path TEXT NOT NULL,
                note TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(game_id) REFERENCES games(id)
            )",
            [],
        )?;

        Ok(())
    }

    pub fn add_game(&self, name: &str, game_folder_path: &str, save_folder_path: &str, exe_path: Option<&str>) -> Result<String> {
        let conn = self.connect()?;
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO games (id, name, game_folder_path, save_folder_path, exe_path) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, game_folder_path, save_folder_path, exe_path],
        )?;
        Ok(id)
    }

    pub fn get_games(&self) -> Result<Vec<Game>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT id, name, exe_path, COALESCE(game_folder_path, save_folder_path, '') as game_folder_path, save_folder_path, cover_image FROM games")?;
        let game_iter = stmt.query_map([], |row| {
            Ok(Game {
                id: row.get(0)?,
                name: row.get(1)?,
                exe_path: row.get(2)?,
                game_folder_path: row.get(3)?,
                save_folder_path: row.get(4)?,
                cover_image: row.get(5)?,
            })
        })?;

        let mut games = Vec::new();
        for game in game_iter {
            games.push(game?);
        }
        Ok(games)
    }

    pub fn add_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO snapshots (id, game_id, original_save_path, backup_save_path, text_content, note, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                snapshot.id,
                snapshot.game_id,
                snapshot.original_save_path,
                snapshot.backup_save_path,
                snapshot.text_content,
                snapshot.note,
                snapshot.created_at
            ],
        )?;
        Ok(())
    }

    pub fn get_snapshots(&self, game_id: &str) -> Result<Vec<Snapshot>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT id, game_id, original_save_path, backup_save_path, text_content, note, created_at FROM snapshots WHERE game_id = ?1 ORDER BY created_at DESC")?;
        let snapshot_iter = stmt.query_map([game_id], |row| {
            Ok(Snapshot {
                id: row.get(0)?,
                game_id: row.get(1)?,
                original_save_path: row.get(2)?,
                backup_save_path: row.get(3)?,
                text_content: row.get(4)?,
                note: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        let mut snapshots = Vec::new();
        for s in snapshot_iter {
            snapshots.push(s?);
        }
        Ok(snapshots)
    }

    pub fn update_snapshot_note(&self, snapshot_id: &str, note: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "UPDATE snapshots SET note = ?1 WHERE id = ?2",
            params![note, snapshot_id],
        )?;
        Ok(())
    }

    pub fn delete_game(&self, game_id: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute("DELETE FROM snapshots WHERE game_id = ?1", [game_id])?;
        conn.execute("DELETE FROM screenshots WHERE game_id = ?1", [game_id])?;
        conn.execute("DELETE FROM games WHERE id = ?1", [game_id])?;
        Ok(())
    }

    pub fn delete_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute("DELETE FROM snapshots WHERE id = ?1", [snapshot_id])?;
        Ok(())
    }

    pub fn add_screenshot(&self, screenshot: &Screenshot) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT INTO screenshots (id, game_id, image_path, note, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                screenshot.id,
                screenshot.game_id,
                screenshot.image_path,
                screenshot.note,
                screenshot.created_at
            ],
        )?;
        Ok(())
    }

    pub fn get_screenshots(&self, game_id: &str) -> Result<Vec<Screenshot>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT id, game_id, image_path, note, created_at FROM screenshots WHERE game_id = ?1 ORDER BY created_at DESC")?;
        let screenshot_iter = stmt.query_map([game_id], |row| {
            Ok(Screenshot {
                id: row.get(0)?,
                game_id: row.get(1)?,
                image_path: row.get(2)?,
                note: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let mut screenshots = Vec::new();
        for s in screenshot_iter {
            screenshots.push(s?);
        }
        Ok(screenshots)
    }

    pub fn update_screenshot_note(&self, screenshot_id: &str, note: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "UPDATE screenshots SET note = ?1 WHERE id = ?2",
            params![note, screenshot_id],
        )?;
        Ok(())
    }

    pub fn delete_screenshot(&self, screenshot_id: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute("DELETE FROM screenshots WHERE id = ?1", [screenshot_id])?;
        Ok(())
    }
}


