#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod manager;
mod cache;

use log::{debug, info, warn};
use std::{env, fs};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::ops::Deref;
use std::path::PathBuf;
use sysinfo::{Pid, ProcessStatus, System};
use tauri::{AppHandle, Manager, PhysicalPosition, Position, Size, State, Url, Window};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use active_win_pos_rs::get_active_window;
use tungstenite::{connect, Message};
use log::{error, trace};
use once_cell::sync::Lazy;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::manager::{OverlayManagerInstance, start_manager_read_thread};


const PROCESS_CHECK_INTERVAL: u64 = 1000 * 1;

static TARGET_PROC_NAME: Lazy<String> = Lazy::new(|| env::var("TARGET_PROCESS_NAME").expect("Missing TARGET_PROCESS_NAME"));

#[derive(PartialEq, serde::Serialize, Clone, Debug)]
enum ViewState {
    Hidden,
    Visible,
    Interactable /* Should not change without user's control */
}

pub type OverlayManager = Arc<Mutex<OverlayManagerInstance>>;

struct AppData {
    sys: System,
    view_state: ViewState,
    manager: OverlayManager,
    config_file_path: PathBuf,
    config: AppConfig,
    http_url: Url,
    element_cache: HashMap<String, overlay_manager::UIElement>
}
#[derive(Serialize, Deserialize)]
pub struct AppConfig {
}
impl AppConfig {
    pub fn new() -> Self {
        Self {
            
        }
    }
}

impl AppData {
    pub fn new(manager_inst: OverlayManagerInstance, http_url: Url) -> Self {
        let mut sys = System::new();
        sys.refresh_processes();
        let config_path = tauri::api::path::config_dir().unwrap().join("config.json");
        let config = AppData::load(&config_path);
        Self {
            sys,
            view_state: ViewState::Hidden,
            manager: Arc::new(Mutex::new(manager_inst)),
            config_file_path: config_path,
            config,
            http_url,
            element_cache: HashMap::new()
        }
    }

    fn load(path: &PathBuf) -> AppConfig {
        match fs::read_to_string(path) {
            Ok(out) => {
                serde_json::from_str(&out).expect("bad JSON data")
            },
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    AppConfig::new()
                } else {
                    panic!("could not read config file at {:?}: {}", path, e)
                }
            }
        }
    }

    fn save(&mut self) {
        let json = serde_json::to_string(&self.config).expect("could not serialize JSON");
        fs::write(&self.config_file_path, json).expect("could not save JSON")
    }
}

fn init_data() -> AppData {
    let ws_url = Url::parse(&env::var("MANAGER_WS_URL").unwrap_or_else(|_| "ws://127.0.0.1:3011/socket".to_string())).expect("bad MANAGER_WS_URL");
    let http_url = Url::parse(&env::var("MANAGER_HTTP_URL").unwrap_or_else(|_| "http://127.0.0.1:3011".to_string())).expect("bad MANAGER_HTTP_URL");
    let manager = manager::OverlayManagerInstance::new(ws_url);
    AppData::new(manager, http_url)
}

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", format!("warn,{}=info", env!("CARGO_PKG_NAME")));
    }

    pretty_env_logger::init();

    let context = tauri::generate_context!();
    let data = init_data();
    let manager = data.manager.clone();
    let data = Mutex::new(data);

    // TODO: add token storage

    tauri::Builder::default()
        .setup(|app| {
            let main_window = app.get_window("main").unwrap();
            main_window.set_always_on_top(true).unwrap();
            main_window.set_decorations(false).unwrap();
            main_window.set_ignore_cursor_events(true).unwrap();
            main_window.open_devtools();
            main_window.hide().unwrap();
            let monitor = main_window.current_monitor().unwrap().unwrap();
            main_window.set_size(Size::from(monitor.size().to_owned())).unwrap();
            main_window.set_position(Position::Physical(PhysicalPosition { x: 0, y: 0 })).unwrap();
            {
                // TODO: process
                /*
                    1. manager connect
                    2. grab auth token from store, if available
                        -> if available, send client + auth
                        -> else, send client + open new window (WindowBuilder), do auth there
                        -> once manager reads auth, store data.
                
                 */
                start_manager_read_thread(main_window.clone(), manager);
                start_process_check_thread(main_window.clone());
            }
            debug!("tauri setup done");
            Ok(())
        })
        .manage(data)
        .invoke_handler(tauri::generate_handler![fetch_element, overlay_key, perform_action])
        .run(context)
        .expect("error while running tauri application");
}

#[derive(serde::Serialize, Clone)]
struct ProcessDataResult {
    pid: u32,
    cpu_usage: f32,
    start_time: u64,
    mem_usage: u64,
    status: String
}
fn start_process_check_thread(window: tauri::Window) {
    std::thread::spawn(move || {
        debug!("process check thread started. target process: {}", TARGET_PROC_NAME.deref());
        loop {
            let state = window.state::<Mutex<AppData>>();
            let active_window = match get_active_window() {
                Ok(window) => window,
                Err(_) => {
                    error!("get_active_window returned error");
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                }
            };
            let our_pid = std::process::id() as u64;
            if active_window.process_id == our_pid {
                // Ignore our window
                continue;
            }
            trace!("our_pid={our_pid} active_window pid={}", active_window.process_id);

            let mut data = state.lock().unwrap();
            data.sys.refresh_processes();
            let mut active = data.view_state == ViewState::Visible;
            // println!("active={} target={}", active_window.app_name, TARGET_PROC_NAME);

            if let Some(proc) = data.sys.processes_by_name(TARGET_PROC_NAME.deref()).next() {
                let pid = proc.pid().as_u32();
                // println!("proc found. pid={} active_pid={}", pid, active_window.process_id);
                /* active_window.process_id == our_pid ||  */
                if active_window.process_id == pid as u64 || active_window.app_name == *TARGET_PROC_NAME {
                    trace!("found proc. pid={pid}");
                    window.emit("process", ProcessDataResult {
                        pid,
                        cpu_usage: proc.cpu_usage(),
                        start_time: proc.start_time(),
                        mem_usage: proc.memory(),
                        status: proc.status().to_string(),
                    }).unwrap();
                    active = true;
                } else {
                    active = false;
                }
            } else {
                // Fallback incase we can detect active window but can't find it's name (unix differences). Won't have payload data.
                active = active_window.app_name == *TARGET_PROC_NAME;
            }

            if data.view_state != ViewState::Interactable {
                if data.view_state == ViewState::Visible && !active {
                    data.view_state = ViewState::Hidden;
                    window.hide().unwrap();
                    debug!("app is now inactive, hiding overlay");
                } else if data.view_state == ViewState::Hidden && active {
                    data.view_state = ViewState::Visible;
                    window.show().unwrap();
                    debug!("app is now active, showing overlay");
                }
            }
            drop(data);
            std::thread::sleep(Duration::from_millis(PROCESS_CHECK_INTERVAL));
        }
        error!("exiting process check loop");
    });
}

#[tauri::command]
async fn fetch_element(data: State<'_, Mutex<AppData>>, namespace: String, id: String) -> Result<Option<overlay_manager::UIElement>, String> {
    let url = {
        let mut data = data.lock().unwrap();
        let mut url = data.http_url.clone();
        url.set_path(&format!("/elements/{namespace}/{id}"));
        url
    };
    let response = reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?;
    if response.status() == StatusCode::NOT_FOUND {
        Ok(None)
    } else {
        let elem = response.error_for_status()
            .map_err(|e| e.to_string())?
            .json().await
            .map_err(|e| e.to_string())?;
        // TODO: cache
        Ok(Some(elem))
    }
}

#[tauri::command]
fn overlay_key(window: Window, data: State<Mutex<AppData>>) -> bool {
    let mut data = data.lock().unwrap();
    if data.view_state == ViewState::Interactable {
        debug!("overlay_key: hiding");
        // TODO: check process to determine state
        data.view_state = ViewState::Hidden;
        window.hide().unwrap();
        window.set_ignore_cursor_events(true).unwrap();
        false
    } else {
        debug!("overlay_key: showing");
        data.view_state = ViewState::Interactable;
        window.show().unwrap();
        window.set_ignore_cursor_events(false).unwrap();
        window.set_focus().unwrap();
        true
    }
}

#[tauri::command]
fn perform_action(action: String) -> Result<String, String> {
    Err("not implemented".to_string())
}
