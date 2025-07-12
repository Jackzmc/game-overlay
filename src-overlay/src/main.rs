#![windows_subsystem = "windows"] // to turn off console.

mod ui;
mod manager;
mod templates;

mod defs;
mod registry;

use std::cell::OnceCell;
use std::collections::HashMap;
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::ptr::read;
use std::time::Instant;

use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::thread::JoinHandle;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tokio::io::Join;
use tokio::sync;
use tracing::{debug, error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use crate::manager::{start_manager_read_thread, OverlayManagerInstance};
use crate::ui::OverlayData;

const PROCESS_CHECK_INTERVAL_MS: u64 = 100;

static TARGET_PROC_NAME: OnceLock<String> = OnceLock::new();

#[derive(PartialEq, serde::Serialize, Clone, Debug)]
enum ViewState {
    Hidden,
    Visible,
    Interactable /* Should not change without user's control */
}


// struct AppData {
//     sys: System,
//     view_state: ViewState,
//     manager: OverlayManager,
//     config_file_path: PathBuf,
//     config: AppConfig,
//     http_url: Url,
//     element_cache: HashMap<String, UITemplate>
// }

#[derive(Clone)]
enum Signal {
    CloseUI,
    HotkeyPressed
}
struct UIContainer {
    tx: Sender<Signal>,
    handle: JoinHandle<()>
}

static ALWAYS_ACTIVE: OnceLock<bool> = OnceLock::new();
fn main() {
    dotenvy::dotenv().ok();
    setup_logging();

    ALWAYS_ACTIVE.set(env::var("ALWAYS_ACTIVE").is_ok()).unwrap();

    let target_proc_name = get_target_process();
    let mut ui_thread: Option<std::thread::JoinHandle<()>> = None;
    let mut sys = System::new();

    let kill_signal = Arc::new(AtomicBool::new(false)); // used to tell UI thread to
    let hotkeys = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::Home);
    if let Err(e) = hotkeys.register(hotkey) {
        error!("Registering global hotkey failed: {}", e);
    }

    let mut ui_container: Option<UIContainer> = None;

    // TODO: if user requests daemon do this, and understand what
    // let daemon = daemonize::Daemonize::new()
    //     .pid_file("/tmp/gameoverlay.pid");
    // daemon.start().unwrap();
    debug!("waiting for {}", target_proc_name);
    loop {
        // Detect if a hotkey was pressed
        if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == HotKeyState::Released && event.id == hotkey.id {
                debug!("Hotkey pressed, send to ui thread");
                if let Some(ui) = &ui_container {
                    ui.tx.send(Signal::HotkeyPressed).unwrap();
                }
                // try_start_ui_thread(&mut ui_thread, &kill_signal, &mut tx);
            }
        }

        if check_for_process(&mut sys, &target_proc_name) {
            // Process active, create ui thread if not already
            try_start_ui_thread(&mut ui_container, &kill_signal);
        } else if let Some(ui) = ui_container.take() {
            // Process inactive, kill the ui thread, if set
            debug!("waiting for UI thread to terminate");
            // kill_signal.store(true, Ordering::Relaxed);
            ui.tx.send(Signal::CloseUI).unwrap();
            ui.handle.join().unwrap();
            // kill_signal.store(false, Ordering::Relaxed);
            debug!("UI thread terminated");
        }
        std::thread::sleep(std::time::Duration::from_millis(PROCESS_CHECK_INTERVAL_MS));
    }
}

fn try_start_ui_thread(ui_container: &mut Option<UIContainer>, kill_signal: &Arc<AtomicBool>) {
    if ui_container.is_none() {
        debug!("spawning UI thread");
        let (tx, rx) = channel::<Signal>();
        *ui_container = Some(UIContainer {
            tx,
            handle: create_ui_thread(kill_signal.clone(), rx)
        });
    }
}

/// Checks if process name exists, returning true if so
fn check_for_process(sys: &mut System, target_name: &str) -> bool {
    if *ALWAYS_ACTIVE.get().unwrap() {
        return true;
    }
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always));
    sys.processes_by_name(target_name.as_ref()).any(|p| true)
}

fn create_ui_thread(kill_signal: Arc<AtomicBool>, rx: std::sync::mpsc::Receiver<Signal>) -> JoinHandle<()> {
    std::thread::spawn(|| {
        // Set up the manager, this sends and receives all requests
        let manager = OverlayManagerInstance::new();
        let manager = Arc::new(Mutex::new(manager));

        // Start background tasks
        let read_rx = start_manager_read_thread(manager.clone());

        // Start the UI
        let state = OverlayData::example(manager, read_rx, kill_signal, rx);
        info!("START ui loop");
        egui_overlay::start(state);
    })
}

fn setup_logging() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or(EnvFilter::new("overlay_ui=debug,warn")),
        )
        .init();
}

/// Returns the desired process to watch for overlay to activate
fn get_target_process() -> String {
    env::var("TARGET_PROC_NAME")
        .unwrap_or_else(|_| {
            if cfg!(target_os = "windows") {
                "left4dead2.exe".to_string()
            } else {
                "left4dead2".to_string()
            }
        })
}