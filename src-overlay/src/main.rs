#![windows_subsystem = "windows"] // to turn off console.

mod ui;
mod manager;
mod templates;

mod defs;
mod registry;

use std::collections::HashMap;
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::ptr::read;
use std::time::Instant;

use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tokio::io::Join;
use tokio::task::JoinHandle;
use tracing::{debug, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use crate::manager::{start_manager_read_thread, OverlayManagerInstance};
use crate::ui::OverlayData;

const PROCESS_CHECK_INTERVAL_MS: u64 = 1000;

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

fn main() {
    dotenvy::dotenv().ok();
    setup_logging();

    // Set up the manager, this sends and receives all requests
    let target_proc_name = get_target_process();

    let mut ui_thread: Option<std::thread::JoinHandle<()>> = None;

    let mut sys = System::new();

    debug!("waiting for {}", target_proc_name);
    let kill_signal = Arc::new(AtomicBool::new(false)); // used to tell UI thread to end
    loop {
        if check_for_process(&mut sys, &target_proc_name) {
            // Process active, create ui thread if not already
            if ui_thread.is_none() {
                debug!("found process, spawning UI thread");
                ui_thread = Some(create_ui_thread(kill_signal.clone()));
            }
        } else if let Some(thread) = ui_thread.take() {
            // Process inactive
            // Kill the ui thread, if set
            debug!("waiting for UI thread to terminate");
            kill_signal.store(true, Ordering::Relaxed);
            thread.join().unwrap();
            kill_signal.store(false, Ordering::Relaxed);
            debug!("UI thread terminated");
        }
        std::thread::sleep(std::time::Duration::from_millis(PROCESS_CHECK_INTERVAL_MS));
    }
}

fn check_for_process(sys: &mut System, target_name: &str) -> bool {
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always));
    sys.processes_by_name(target_name.as_ref()).any(|p| true)
}

fn create_ui_thread(kill_signal: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(|| {
        let manager = OverlayManagerInstance::new();
        let manager = Arc::new(Mutex::new(manager));

        // Start background tasks
        let read_rx = start_manager_read_thread(manager.clone());

        // Start the UI
        let state = OverlayData::example(manager, read_rx, kill_signal);
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