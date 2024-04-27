#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod manager;

use sysinfo::{Pid, ProcessStatus, System};
use tauri::{AppHandle, Manager, PhysicalPosition, Position, Size, State, Url, Window};
use std::sync::Mutex;
use windows::{
    Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowModuleFileNameW, GetWindowThreadProcessId,
    },
    Win32::System::ProcessStatus::{
        GetModuleFileNameExW
    },
    Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION
    }
};
use windows::Win32::Foundation::{HANDLE, HMODULE};
use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
use tungstenite::{connect, Message};
use crate::manager::ManagerResponse;


const TARGET_PROC_NAME: &str = "left4dead2.exe";
// const TARGET_PROC_NAME: &str = "WinSCP.exe";
const MANAGER_WS_URL: &str = "ws://localhost:3012/socket";

#[derive(PartialEq, serde::Serialize, Clone, Debug)]
enum ViewState {
    Hidden,
    Visible,
    Interactable /* Should not change without user's control */
}

struct AppData {
    sys: System,
    view_state: ViewState
}

impl AppData {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_processes();
        Self {
            sys,
            view_state: ViewState::Hidden
        }
    }
}

fn main() {
    let context = tauri::generate_context!();
    let data = Mutex::new(AppData::new());
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
            main_window
                .set_position(Position::Physical(PhysicalPosition { x: 0, y: 0 }))
                .unwrap();

            Ok(())
        })
        .manage(data)
        .invoke_handler(tauri::generate_handler![check_process, init_manager, init_login, overlay_key])
        .run(context)
        .expect("error while running tauri application");
}

#[derive(serde::Serialize)]
struct ProcessDataResult {
    pid: u32,
    cpu_usage: f32,
    start_time: u64,
    mem_usage: u64,
    status: String
}
enum ProcessData {
    Result(ProcessDataResult),
}
#[tauri::command]
fn check_process(app: tauri::AppHandle, data: State<Mutex<AppData>>) -> Option<ProcessDataResult> {
    let mut data = data.lock().unwrap();
    data.sys.refresh_processes();
    let window = app.get_window("main").unwrap();
    let active_pid = get_active_window_pid();
    let our_pid = std::process::id();
    let mut active = data.view_state == ViewState::Visible;
    let mut result: Option<ProcessDataResult> = None;
    if let Some(proc) = data.sys.processes_by_exact_name(TARGET_PROC_NAME).next() {
        let pid = proc.pid().as_u32();
        if active_pid == our_pid || active_pid == pid {
            result = Some(ProcessDataResult {
                pid: proc.pid().as_u32(),
                cpu_usage: proc.cpu_usage(),
                start_time: proc.start_time(),
                mem_usage: proc.memory(),
                status: proc.status().to_string(),
            });
            active = true;
        } else {
            active = false;
        }
    } else {
        active = false;
    }

    if data.view_state != ViewState::Interactable {
        if data.view_state == ViewState::Visible && !active {
            data.view_state = ViewState::Hidden;
            window.hide().unwrap();
        } else if data.view_state == ViewState::Hidden && active {
            data.view_state = ViewState::Visible;
            window.show().unwrap();
        }
    }
    println!("view_state={:?} active={}", data.view_state, active);
    result
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
        data.view_state = ViewState::Hidden;
        window.hide().unwrap();
        window.set_ignore_cursor_events(true).unwrap();
        false
    } else {
        data.view_state = ViewState::Interactable;
        window.show().unwrap();
        window.set_ignore_cursor_events(false).unwrap();
        true
    }
}

// init a background process on the command, and emit periodic events only to the window that used the command
#[tauri::command]
fn init_manager(window: Window) {
    std::thread::spawn(move || {
        let mut manager = manager::Manager::new(Url::parse(MANAGER_WS_URL).expect("bad manager url"));
        if let Err(err) = manager.reconnect() {
            window.emit("manager", ManagerResponse::ManagerDisconnected { message: Some(err.to_string()) }).unwrap();
        } else {
            loop {
                if let Ok(Some(response)) = manager.read() {
                    window.emit("manager", response).unwrap();
                }
            }
        }
    });
}

fn get_active_window_pid() -> u32 {
    unsafe {
        let hwnd = GetForegroundWindow();

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid
    }
}
fn get_active_window() -> (u32, String) {
    unsafe {
       let pid = get_active_window_pid();

        let proc = OpenProcess(
            PROCESS_QUERY_INFORMATION,
            false,
            pid
        ).expect("could not open");


        let mut bytes: [u16; 500] = [0; 500];
        let len = GetModuleBaseNameW(proc, HMODULE(0), &mut bytes);
        // let len = windows::Win32::System::ProcessStatus::GetProcessImageFileNameW(proc, &mut bytes);
        let exe = String::from_utf16_lossy(&bytes[..len as usize]);

        (pid, exe)
    }
}