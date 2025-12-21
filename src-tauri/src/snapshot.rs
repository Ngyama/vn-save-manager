use crate::db::{Database, Snapshot};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use chrono::Utc;
use uuid::Uuid;
use screenshots::Screen;
use arboard::Clipboard;
use image::GenericImageView;

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, RECT},
    System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    System::ProcessStatus::K32GetModuleFileNameExW,
    UI::WindowsAndMessaging::{EnumWindows, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible},
};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::CloseHandle;

struct CachedScreenshot {
    timestamp: Instant,
    path: PathBuf,
    origin_x: i32,
    origin_y: i32,
}

pub struct SnapshotManager {
    db: Database,
    app_handle: AppHandle,
    screenshot_cache: Arc<Mutex<VecDeque<CachedScreenshot>>>,
}

impl SnapshotManager {
    pub fn new(app_handle: AppHandle) -> Self {
        let db = Database::new(&app_handle);

        let screenshot_cache: Arc<Mutex<VecDeque<CachedScreenshot>>> =
            Arc::new(Mutex::new(VecDeque::new()));

        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir for screenshot cache");
        let cache_dir = app_data_dir.join("screenshot_cache");
        if let Err(e) = fs::create_dir_all(&cache_dir) {
            eprintln!("Failed to create screenshot cache dir: {}", e);
        }

        let cache_dir_clone = cache_dir.clone();
        let cache_for_thread = screenshot_cache.clone();
        std::thread::spawn(move || {
            let interval = Duration::from_millis(800);
            let max_items: usize = 20;

            loop {
                if let Err(e) = capture_and_store_screenshot(&cache_dir_clone, &cache_for_thread, max_items) {
                    eprintln!("Screenshot cache capture failed: {}", e);
                }

                std::thread::sleep(interval);
            }
        });

        Self {
            db,
            app_handle,
            screenshot_cache,
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
        let visual_log_dir = game_folder.join("visual-log");
        fs::create_dir_all(&visual_log_dir)?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let snapshot_folder_name = format!("{}_{}", game.name, timestamp);
        let snapshot_folder = visual_log_dir.join(&snapshot_folder_name);
        fs::create_dir_all(&snapshot_folder)?;

        let uuid = Uuid::new_v4().to_string();

        std::thread::sleep(std::time::Duration::from_millis(200));

        let dat_file_name = changed_file_path.file_name()
            .ok_or("Invalid file name")?
            .to_str()
            .ok_or("Invalid file name encoding")?;
        let dat_backup_path = snapshot_folder.join(dat_file_name);
        fs::copy(changed_file_path, &dat_backup_path)?;

        let screenshot_path = snapshot_folder.join("screenshot.png");

        let window_rect = game
            .exe_path
            .as_deref()
            .and_then(|exe| find_window_rect_for_exe(exe));

        let cached_screenshot: Option<(PathBuf, i32, i32)> = {
            let cache = self.screenshot_cache.lock().unwrap();
            let max_age = Duration::from_secs(10);
            let mut best: Option<(PathBuf, i32, i32, Duration)> = None;

            for item in cache.iter() {
                let age = item.timestamp.elapsed();
                if age <= max_age {
                    match &best {
                        Some((_, _, _, best_age)) => {
                            if age < *best_age {
                                best = Some((item.path.clone(), item.origin_x, item.origin_y, age));
                            }
                        }
                        None => {
                            best = Some((item.path.clone(), item.origin_x, item.origin_y, age));
                        }
                    }
                }
            }

            best.map(|(path, ox, oy, _)| (path, ox, oy))
        };

        if let Some((cached_path, origin_x, origin_y)) = cached_screenshot {
            if let Some(rect) = window_rect {
                if let Err(e) = crop_cached_to_window(&cached_path, &screenshot_path, rect, (origin_x, origin_y)) {
                    eprintln!("Failed to crop cached screenshot, fallback to full cached image: {}", e);
                    if let Err(e2) = fs::copy(&cached_path, &screenshot_path) {
                        eprintln!("Failed to copy cached screenshot, fallback to live capture: {}", e2);
                        capture_screen_to_path(&screenshot_path)?;
                    }
                }
            } else {
                if let Err(e) = fs::copy(&cached_path, &screenshot_path) {
                    eprintln!("Failed to copy cached screenshot, fallback to live capture: {}", e);
                    capture_screen_to_path(&screenshot_path)?;
                }
            }
        } else {
            capture_screen_to_path(&screenshot_path)?;
        }

        let app_data_dir = self.app_handle.path().app_data_dir()?;
        let ui_screenshots_dir = app_data_dir.join("screenshots").join(&game.id);
        fs::create_dir_all(&ui_screenshots_dir)?;
        let ui_screenshot_path = ui_screenshots_dir.join(format!("{}.png", uuid));
        if let Err(e) = fs::copy(&screenshot_path, &ui_screenshot_path) {
            eprintln!("Failed to copy screenshot for UI display: {}", e);
        }

        let context_path = snapshot_folder.join("context.txt");
        fs::write(&context_path, "").ok();

        let note_path = snapshot_folder.join("note.txt");
        fs::write(&note_path, "").ok();

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

        let mut clipboard = Clipboard::new()?;
        let text_content = clipboard.get_text().ok();

        let snapshot = Snapshot {
            id: uuid,
            game_id: game.id,
            original_save_path: changed_file_path.to_string_lossy().to_string(),
            backup_save_path: snapshot_folder.to_string_lossy().to_string(),
            image_path: Some(ui_screenshot_path.to_string_lossy().to_string()),
            text_content,
            note: None,
            created_at: Utc::now().to_rfc3339(),
        };

        self.db.add_snapshot(&snapshot)?;
        
        {
            let mut last_time = last_snapshot_time.lock().unwrap();
            *last_time = Instant::now();
        }
        
        self.app_handle.emit("snapshot-created", &snapshot)?;

        Ok(())
    }
}

fn capture_screen_to_path(target_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let screens = Screen::all()?;
    let primary_screen = screens.first().ok_or("No screen found")?;
    let image = primary_screen.capture()?;
    image.save(target_path)?;
    Ok(())
}

fn capture_and_store_screenshot(
    cache_dir: &PathBuf,
    cache: &Arc<Mutex<VecDeque<CachedScreenshot>>>,
    max_items: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let screens = Screen::all()?;
    let primary_screen = match screens.first() {
        Some(s) => s,
        None => return Ok(()),
    };

    let origin_x = primary_screen.display_info.x;
    let origin_y = primary_screen.display_info.y;

    let image = primary_screen.capture()?;

    let now_utc = Utc::now();
    let filename = format!("cache_{}.png", now_utc.format("%Y%m%d_%H%M%S%3f"));
    let path = cache_dir.join(filename);

    image.save(&path)?;

    let mut guard = cache.lock().unwrap();
    guard.push_back(CachedScreenshot {
        timestamp: Instant::now(),
        path: path.clone(),
        origin_x,
        origin_y,
    });

    while guard.len() > max_items {
        if let Some(old) = guard.pop_front() {
            let _ = fs::remove_file(old.path);
        }
    }

    Ok(())
}

fn crop_cached_to_window(
    cached_path: &PathBuf,
    target_path: &PathBuf,
    rect: (i32, i32, i32, i32),
    origin: (i32, i32),
) -> Result<(), Box<dyn std::error::Error>> {
    let img = image::open(cached_path)?;
    let (img_w, img_h) = img.dimensions();

    let (origin_x, origin_y) = origin;

    let (mut left, mut top, mut right, mut bottom) = (
        rect.0 - origin_x,
        rect.1 - origin_y,
        rect.2 - origin_x,
        rect.3 - origin_y,
    );

    if left < 0 {
        left = 0;
    }
    if top < 0 {
        top = 0;
    }
    if right > img_w as i32 {
        right = img_w as i32;
    }
    if bottom > img_h as i32 {
        bottom = img_h as i32;
    }

    let width = (right - left).max(1) as u32;
    let height = (bottom - top).max(1) as u32;

    let cropped = img.crop_imm(left as u32, top as u32, width, height);
    cropped.save(target_path)?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn find_window_rect_for_exe(exe_path: &str) -> Option<(i32, i32, i32, i32)> {
    fn normalize(p: &str) -> String {
        PathBuf::from(p)
            .to_string_lossy()
            .replace('\\', "/")
            .to_lowercase()
    }

    let target_norm = normalize(exe_path);

    struct SearchCtx {
        target_norm: String,
        found_rect: Option<RECT>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam.0 as *mut SearchCtx);

        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        let mut pid: u32 = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        if pid == 0 {
            return BOOL(1);
        }

        let hproc = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) };
        if let Err(_) = hproc {
            return BOOL(1);
        }
        let hproc = hproc.unwrap();

        let mut buf = [0u16; 260];
        let len = unsafe { K32GetModuleFileNameExW(hproc, None, &mut buf) };
        let _ = unsafe { CloseHandle(hproc) };
        if len == 0 {
            return BOOL(1);
        }

        let exe = String::from_utf16_lossy(&buf[..len as usize]);
        let exe_norm = exe.replace('\\', "/").to_lowercase();

        if exe_norm == ctx.target_norm {
            let mut rect = RECT::default();
            let ok = unsafe { GetWindowRect(hwnd, &mut rect) };
            if ok.is_ok() {
                ctx.found_rect = Some(rect);
                return BOOL(0);
            }
        }

        BOOL(1)
    }

    let mut ctx = SearchCtx {
        target_norm,
        found_rect: None,
    };

    let lparam = LPARAM(&mut ctx as *mut _ as isize);
    let _ = unsafe { EnumWindows(Some(enum_proc), lparam) };

    ctx.found_rect.map(|r| (r.left, r.top, r.right, r.bottom))
}

#[cfg(not(target_os = "windows"))]
fn find_window_rect_for_exe(_exe_path: &str) -> Option<(i32, i32, i32, i32)> {
    None
}

