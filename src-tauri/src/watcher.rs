use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;
use tauri::AppHandle;
use tauri::Emitter;

pub struct SaveWatcher {
    watcher: RecommendedWatcher,
}

impl SaveWatcher {
    pub fn new(_app_handle: AppHandle) -> (Self, Receiver<notify::Result<Event>>) {
        let (tx, rx) = channel();
        
        let watcher = RecommendedWatcher::new(tx, Config::default()).expect("Failed to create watcher");

        (Self { watcher }, rx)
    }

    pub fn watch(&mut self, path: &str) -> notify::Result<()> {
        let path = Path::new(path);
        self.watcher.watch(path, RecursiveMode::Recursive)?;
        Ok(())
    }

    pub fn unwatch(&mut self, path: &str) -> notify::Result<()> {
        let path = Path::new(path);
        self.watcher.unwatch(path)?;
        Ok(())
    }
}

pub fn start_watcher_loop(rx: Receiver<notify::Result<Event>>, app_handle: AppHandle) {
    std::thread::spawn(move || {
        let mut last_event_time = std::time::Instant::now();
        let debounce_duration = Duration::from_secs(2); 

        for res in rx {
            match res {
                Ok(event) => {
                    match event.kind {
                        notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                            if last_event_time.elapsed() < debounce_duration {
                                continue;
                            }
                            last_event_time = std::time::Instant::now();


                            if let Err(e) = app_handle.emit("save-detected", event.paths) {
                                // Failed to send event
                            }
                        },
                        _ => {}
                    }
                },
                Err(_) => {},
            }
        }
    });
}


