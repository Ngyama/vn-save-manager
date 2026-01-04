use crate::db::{Database, Screenshot};
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;
use chrono::Utc;
use uuid::Uuid;
use screenshots::Screen;

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, RECT},
    System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    System::ProcessStatus::K32GetModuleFileNameExW,
    UI::WindowsAndMessaging::{EnumWindows, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible},
};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::CloseHandle;

pub struct ScreenshotManager {
    db: Database,
    app_handle: AppHandle,
}

impl ScreenshotManager {
    pub fn new(app_handle: AppHandle) -> Self {
        let db = Database::new(&app_handle);
        Self {
            db,
            app_handle,
        }
    }

    pub fn capture_screenshot_for_running_game(&self) -> Result<Screenshot, Box<dyn std::error::Error>> {
        println!("capture_screenshot_for_running_game called");
        let games = self.db.get_games()?;
        println!("Found {} games", games.len());
        
        let running_game = games.iter()
            .find(|game| {
                if let Some(exe_path) = &game.exe_path {
                    println!("Checking game: {} with exe: {}", game.name, exe_path);
                    let found = find_window_rect_for_exe(exe_path).is_some();
                    if found {
                        println!("Found running window for game: {}", game.name);
                    }
                    found
                } else {
                    println!("Game {} has no exe_path", game.name);
                    false
                }
            })
            .ok_or_else(|| {
                let msg = format!("No running game found. Checked {} games.", games.len());
                println!("{}", msg);
                msg
            })?;

        println!("Capturing screenshot for game: {}", running_game.name);
        self.capture_screenshot(&running_game.id)
    }

    pub fn capture_screenshot(&self, game_id: &str) -> Result<Screenshot, Box<dyn std::error::Error>> {
        println!("capture_screenshot called for game_id: {}", game_id);
        let games = self.db.get_games()?;
        let game = games.iter()
            .find(|g| g.id == game_id)
            .ok_or("Game not found")?;

        let exe_path = game.exe_path.as_ref()
            .ok_or("Game exe_path not set")?;

        println!("Looking for window for exe: {}", exe_path);
        let window_rect = find_window_rect_for_exe(exe_path);
        if window_rect.is_none() {
            return Err(format!("Game window not found for: {}", exe_path).into());
        }
        let rect = window_rect.unwrap();
        println!("Found window rect: ({}, {}, {}, {})", rect.0, rect.1, rect.2, rect.3);

        let screens = Screen::all()?;
        let primary_screen = screens.first().ok_or("No screen found")?;
        
        #[cfg(target_os = "windows")]
        let (origin_x, origin_y) = {
            let info = &primary_screen.display_info;
            (info.x, info.y)
        };
        
        #[cfg(not(target_os = "windows"))]
        let (origin_x, origin_y) = (0, 0);
        
        let image_buffer = primary_screen.capture()?;

        let app_data_dir = self.app_handle.path().app_data_dir()?;
        let temp_dir = app_data_dir.join("temp");
        fs::create_dir_all(&temp_dir)?;
        let temp_path = temp_dir.join(format!("temp_{}.png", Uuid::new_v4()));
        image_buffer.save(&temp_path)?;

        let full_image = image::open(&temp_path)?;
        println!("Full image size: {}x{}", full_image.width(), full_image.height());
        println!("Origin: ({}, {})", origin_x, origin_y);
        
        let (mut left, mut top, mut right, mut bottom) = (
            rect.0 - origin_x,
            rect.1 - origin_y,
            rect.2 - origin_x,
            rect.3 - origin_y,
        );
        println!("Cropped rect before clamping: ({}, {}, {}, {})", left, top, right, bottom);
        left = left.max(0);
        top = top.max(0);
        right = right.min(full_image.width() as i32);
        bottom = bottom.min(full_image.height() as i32);

        let width = (right - left).max(1) as u32;
        let height = (bottom - top).max(1) as u32;
        let cropped_image = full_image.crop_imm(left as u32, top as u32, width, height);

        let game_folder = PathBuf::from(&game.game_folder_path);
        let screenshots_dir = game_folder.join("visual-logger").join("screenshots");
        fs::create_dir_all(&screenshots_dir)?;

        let now_utc = Utc::now();
        let timestamp = now_utc.format("%Y%m%d_%H%M%S").to_string();
        let millis = now_utc.timestamp_subsec_millis();
        let filename = format!("screenshot_{}_{:03}.png", timestamp, millis);
        let screenshot_path = screenshots_dir.join(&filename);
        cropped_image.save(&screenshot_path)?;

        let _ = fs::remove_file(&temp_path);

        let screenshot = Screenshot {
            id: Uuid::new_v4().to_string(),
            game_id: game_id.to_string(),
            image_path: screenshot_path.to_string_lossy().to_string(),
            note: None,
            created_at: now_utc.to_rfc3339(),
        };

        self.db.add_screenshot(&screenshot)?;
        
        Ok(screenshot)
    }
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

        if IsWindowVisible(hwnd).0 == 0 {
            return BOOL(1);
        }

        let mut pid: u32 = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        if pid == 0 {
            return BOOL(1);
        }

        let hproc = match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
            Ok(h) => h,
            Err(_) => return BOOL(1),
        };

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
            unsafe {
                let _ = GetWindowRect(hwnd, &mut rect);
            }
            ctx.found_rect = Some(rect);
            return BOOL(0);
        }

        BOOL(1)
    }

    let mut ctx = SearchCtx {
        target_norm,
        found_rect: None,
    };

    let _ = unsafe {
        EnumWindows(
            Some(enum_proc),
            LPARAM(&mut ctx as *mut SearchCtx as isize),
        )
    };

    ctx.found_rect.map(|r| (r.left, r.top, r.right, r.bottom))
}

#[cfg(not(target_os = "windows"))]
fn find_window_rect_for_exe(_exe_path: &str) -> Option<(i32, i32, i32, i32)> {
    None
}

