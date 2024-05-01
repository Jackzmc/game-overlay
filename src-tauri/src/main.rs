#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod manager;

use log::{debug, info, warn};
use std::env;
use std::ops::Deref;
use sysinfo::{Pid, ProcessStatus, System};
use tauri::{AppHandle, Manager, PhysicalPosition, Position, Size, State, Url, Window};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use active_win_pos_rs::get_active_window;
use tungstenite::{connect, Message};
use log::{error, trace};
use crate::manager::{OverlayManagerInstance, start_manager_read_thread};


#[cfg(windows)]
// const TARGET_PROC_NAME: &str = "left4dead2.exe";
const TARGET_PROC_NAME: &str = "thunderbird.exe";
#[cfg(unix)]
// const TARGET_PROC_NAME: &str = "left4dead2";
const TARGET_PROC_NAME: &str = "thunderbird";
const PROCESS_CHECK_INTERVAL: u64 = 1000 * 1;

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
    manager: OverlayManager
}

impl AppData {
    pub fn new(manager_inst: OverlayManagerInstance) -> Self {
        let mut sys = System::new();
        sys.refresh_processes();
        Self {
            sys,
            view_state: ViewState::Hidden,
            manager: Arc::new(Mutex::new(manager_inst))
        }
    }
}

fn init_data() -> AppData {
    let url = Url::parse(&std::env::var("MANAGER_WS_URL").unwrap_or_else(|_| "ws://127.0.0.1:3011/socket".to_string())).expect("bad MANAGER_WS_URL");
    let manager = manager::OverlayManagerInstance::new(url);
    AppData::new(manager)
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", format!("warn,{}=info", env!("CARGO_PKG_NAME")));
    }
    pretty_env_logger::init();

    let context = tauri::generate_context!();
    let data = init_data();
    let manager = data.manager.clone();
    let data = Mutex::new(data);

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
                start_manager_read_thread(main_window.clone(), manager);
                start_process_check_thread(main_window.clone());
            }
            debug!("tauri setup done");
            Ok(())
        })
        .manage(data)
        .invoke_handler(tauri::generate_handler![init_login, overlay_key])
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
        debug!("process check thread started. target process: {}", TARGET_PROC_NAME);
        loop {
            let state = window.state::<Mutex<AppData>>();
            let active_window = match get_active_window() {
                Ok(window) => window,
                Err(_) => {
                    error!("get_active_window returned error");
                    continue;
                }
            };
            let our_pid = std::process::id() as u64;
            trace!("our_pid={our_pid} active_window pid={}", active_window.process_id);

            let mut data = state.lock().unwrap();
            data.sys.refresh_processes();
            let mut active = data.view_state == ViewState::Visible;
            // println!("active={} target={}", active_window.app_name, TARGET_PROC_NAME);

            if let Some(proc) = data.sys.processes_by_name(TARGET_PROC_NAME).next() {
                let pid = proc.pid().as_u32();
                // println!("proc found. pid={} active_pid={}", pid, active_window.process_id);
                if active_window.process_id == our_pid || active_window.process_id == pid as u64 || active_window.app_name == TARGET_PROC_NAME {
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
                active = active_window.app_name == TARGET_PROC_NAME;
            }

            if data.view_state != ViewState::Interactable {
                if data.view_state == ViewState::Visible && !active {
                    data.view_state = ViewState::Hidden;
                    // window.hide().unwrap();
                    window.close().unwrap();
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
    });
}

#[tauri::command]
async fn init_login(app: AppHandle) {
    let local_window = tauri::WindowBuilder::new(
        &app,
        "panel_login",
        tauri::WindowUrl::External("https://admin.jackz.me".parse().expect("bad url"))
    )
        .closable(false)
        .title("Admin Panel Login")
        .build().expect("window failed");
    local_window.open_devtools();
}

#[tauri::command]
fn overlay_key(window: Window, data: State<Mutex<AppData>>) -> bool {
    let mut data = data.lock().unwrap();
    if data.view_state == ViewState::Interactable {
        debug!("overlay_key: hiding");
        data.view_state = ViewState::Hidden;
        window.hide().unwrap();
        window.set_ignore_cursor_events(true).unwrap();
        false
    } else {
        debug!("overlay_key: showing");
        data.view_state = ViewState::Interactable;
        window.show().unwrap();
        window.set_ignore_cursor_events(false).unwrap();
        true
    }
}