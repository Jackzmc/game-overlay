#![windows_subsystem = "windows"] // to turn off console.

mod ui;
mod manager;

use std::collections::HashMap;
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::ptr::read;
use std::time::Instant;

use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use crate::manager::{start_manager_read_thread, OverlayManagerInstance};
use crate::ui::OverlayData;

const PROCESS_CHECK_INTERVAL: u64 = 500;

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
    let target_process = get_target_process();
    let manager = OverlayManagerInstance::new();
    let manager = Arc::new(Mutex::new(manager));

    // Start background tasks
    let read_rx = start_manager_read_thread(manager.clone());

    // Start the UI
    let state = OverlayData::example(manager, read_rx);
    egui_overlay::start(state);
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