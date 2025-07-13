#![windows_subsystem = "windows"] // to turn off console.

mod ui;
mod manager;
mod templates;

mod defs;
mod registry;

use std::cell::OnceCell;
use std::collections::HashMap;
use std::env;
use std::io::{pipe, PipeReader, PipeWriter, Read};
use std::net::{IpAddr, SocketAddr};
use std::os::unix::process::parent_id;
use std::os::unix::raw::pid_t;
use std::path::PathBuf;
use std::ptr::read;
use std::time::{Duration, Instant};

use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::thread::JoinHandle;
use fork::{fork, waitpid, Fork};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use libc::SIGTERM;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tokio::io::Join;
use tokio::sync;
use tracing::{debug, error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use crate::manager::{start_manager_read_thread, OverlayManagerInstance};
use crate::ui::OverlayData;

const MAIN_INTERVAL_SLEEP_MS: u64 = 500;

static TARGET_PROC_NAME: OnceLock<String> = OnceLock::new();

struct UIContainer {
    pid: pid_t,
    wait_thread: JoinHandle<()>,
}

static ALWAYS_ACTIVE: OnceLock<bool> = OnceLock::new();
fn main() {
    dotenvy::dotenv().ok();
    setup_logging();

    ALWAYS_ACTIVE.set(env::var("ALWAYS_ACTIVE").is_ok()).unwrap();

    let target_proc_name = get_target_process();
    let mut sys = System::new();

    let mut ui_cont: Option<UIContainer> = None;

    // TODO: need to have ui native window follow target proc window?
    let (reader, writer) = pipe().unwrap();

    info!("PID: {} - Waiting for {}", std::process::id(), target_proc_name);
    'main: loop {
       if let Some(target_pid) = check_for_process(&mut sys, &target_proc_name) {
           // Process active, create ui process if not already, and check if process still running
           // TODO: test on windows
           match &ui_cont {
               Some(ui) => {
                   // egui sometimes crashes from seg fault seemingly randomly
                   // so just keep relaunching until it works shrug ¯\_(ツ)_/¯
                   if ui.wait_thread.is_finished() {
                       error!("child process died. recreating?");
                       ui_cont = None; // will cause a new fork()
                   }
               },
               None => {
                   // Fork the process, spawning a child process that becomes the UI
                   // The main process here continues, and continues to check if target process & ui process are active
                   match fork() {
                       Ok(Fork::Parent(child)) => {
                           // Running on parent:
                           info!("ui child created: {}", child);
                           ui_cont = Some(UIContainer {
                               pid: child.clone(),
                               // Used to detect if child died, thread just waits until it ends
                               wait_thread: std::thread::spawn(move || {
                                   waitpid(child).expect("waitpid failed");
                               }),
                           });
                       }

                       Ok(Fork::Child) => {
                           // Running on child:
                           info!("child alive, ending main loop");
                           start_child_process(reader, target_pid);
                           break 'main;
                       }
                       Err(e) => { panic!("fork failed: {}", e) }
                   }
               }

           }
       } else if let Some(ui) = ui_cont.take() {
           // Process inactive, kill the ui child, if set
           debug!("Waiting for child process to end");
           unsafe {
               libc::kill(ui.pid, SIGTERM);
           }
           ui.wait_thread.join().unwrap();
           debug!("Child terminated");
       }
        std::thread::sleep(Duration::from_millis(MAIN_INTERVAL_SLEEP_MS));
    }
    // Any code below here runs both on main and ui process
}

/// Runs on child process, starting up and entering the egui event loop
fn start_child_process(mut reader: PipeReader, target_pid: u32) {
    let manager = OverlayManagerInstance::new();
    let manager = Arc::new(Mutex::new(manager));

    // Start background tasks
    let read_rx = start_manager_read_thread(manager.clone());

    // idk if this is needed but it doesnt complain
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let mut buf = [0u8; 1024];
        while let Ok(n) = reader.read(&mut buf) {
            if n > 0 {
                tx.send(buf[..n].to_vec()).unwrap();
            }
        }
    });

    // Start the UI
    let state = OverlayData::example(manager, read_rx, rx, target_pid);
    info!("START ui loop");
    egui_overlay::start(state);
}

/// Checks if process name exists, returning the pid of the process
fn check_for_process(sys: &mut System, target_name: &str) -> Option<u32> {
    if *ALWAYS_ACTIVE.get().unwrap() {
        return Some(0);
    }
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always));
    sys.processes_by_name(target_name.as_ref()).find(|p| true).map(|p| p.pid().as_u32())
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