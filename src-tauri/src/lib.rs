mod mc_title;
mod pty_registry;
mod resolve_mc;

use pty_registry::{PtySessionInfo, PtySessionRegistry};
use resolve_mc::resolve_mc;
use serde::Serialize;
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    AppHandle, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder,
};

#[derive(Serialize)]
struct LiveVerifySnapshot {
    session_count: usize,
    sessions: Vec<PtySessionInfo>,
    new_window_invoked: bool,
}

fn verify_live_multi_window_enabled() -> bool {
    std::env::args().any(|arg| arg == "--verify-live-multi-window")
}

fn write_live_verify_snapshot(
    app: &AppHandle,
    new_window_invoked: bool,
) -> Result<LiveVerifySnapshot, String> {
    let registry = app.state::<PtySessionRegistry>();
    let snapshot = LiveVerifySnapshot {
        session_count: registry.session_count(),
        sessions: registry.list_sessions(),
        new_window_invoked,
    };

    if let Ok(output_path) = std::env::var("MC_VERIFY_OUTPUT") {
        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|err| format!("failed to encode verify snapshot: {err}"))?;
        std::fs::write(&output_path, json)
            .map_err(|err| format!("failed to write verify snapshot: {err}"))?;
    }

    Ok(snapshot)
}

fn spawn_live_multi_window_verification(app: AppHandle) {
    std::thread::spawn(move || {
        let mut new_window_invoked = false;

        for _ in 0..40 {
            if app.state::<PtySessionRegistry>().session_count() >= 1 {
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }

        if new_mc_window(app.clone()).is_ok() {
            new_window_invoked = true;
        }

        for _ in 0..40 {
            let registry = app.state::<PtySessionRegistry>();
            if registry.session_count() >= 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }

        let _ = write_live_verify_snapshot(&app, new_window_invoked);
    });
}

#[tauri::command]
fn resolve_mc_path() -> Result<String, String> {
    resolve_mc()
        .map(|path| path.display().to_string())
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn dry_run_mc(
    registry: tauri::State<'_, PtySessionRegistry>,
    window_label: String,
) -> Result<String, String> {
    registry
        .dry_run_for_window(&window_label)
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn spawn_mc(
    app: AppHandle,
    registry: tauri::State<'_, PtySessionRegistry>,
    window_label: String,
    cols: u16,
    rows: u16,
) -> Result<PtySessionInfo, String> {
    registry.create_session(Some(app), window_label, cols, rows)
}

#[tauri::command]
fn write_pty(
    registry: tauri::State<'_, PtySessionRegistry>,
    window_label: String,
    data: String,
) -> Result<(), String> {
    registry.write_to_session(&window_label, &data)
}

#[tauri::command]
fn resize_pty(
    registry: tauri::State<'_, PtySessionRegistry>,
    window_label: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    registry.resize_session(&window_label, cols, rows)
}

#[tauri::command]
fn close_pty_session(
    registry: tauri::State<'_, PtySessionRegistry>,
    window_label: String,
) -> Result<bool, String> {
    Ok(registry.destroy_session(&window_label))
}

#[tauri::command]
fn list_pty_sessions(
    registry: tauri::State<'_, PtySessionRegistry>,
) -> Result<Vec<PtySessionInfo>, String> {
    Ok(registry.list_sessions())
}

#[tauri::command]
fn get_pty_session_count(registry: tauri::State<'_, PtySessionRegistry>) -> Result<usize, String> {
    Ok(registry.session_count())
}

#[tauri::command]
fn new_mc_window(app: AppHandle) -> Result<String, String> {
    let label = format!("mc-{}", uuid::Uuid::new_v4().simple());
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("index.html".into()))
        .title("Midnight Commander")
        .inner_size(1024.0, 768.0)
        .resizable(true)
        .build()
        .map_err(|err| err.to_string())?;
    Ok(label)
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let new_window =
        MenuItem::with_id(app, "new_window", "New Window", true, Some("CmdOrCtrl+N"))?;
    let close_window = PredefinedMenuItem::close_window(app, Some("Close Window"))?;
    let quit = PredefinedMenuItem::quit(app, Some("Quit"))?;
    let file_menu = Submenu::with_items(app, "File", true, &[&new_window, &close_window])?;
    let app_menu = Submenu::with_items(app, "Midnight Commander", true, &[&quit])?;
    Menu::with_items(app, &[&app_menu, &file_menu])
}

pub fn verify_sessions_cli() -> Result<(), String> {
    use std::path::PathBuf;

    let registry = PtySessionRegistry::new();
    let mc_path = resolve_mc().map_err(|err| err.to_string())?;
    println!("resolved mc: {}", mc_path.display());
    println!(
        "dry-run main: {}",
        registry
            .dry_run_for_window("mc-main")
            .map_err(|err| err.to_string())?
    );
    println!(
        "dry-run second: {}",
        registry
            .dry_run_for_window("mc-second")
            .map_err(|err| err.to_string())?
    );

    registry.create_session_with_command(
        None,
        "mc-main".to_string(),
        PathBuf::from("/bin/sleep"),
        vec!["30".to_string()],
        80,
        24,
    )?;
    registry.create_session_with_command(
        None,
        "mc-second".to_string(),
        PathBuf::from("/bin/sleep"),
        vec!["30".to_string()],
        80,
        24,
    )?;

    println!("session_count: {}", registry.session_count());
    for session in registry.list_sessions() {
        println!("session: {session:?}");
    }

    assert!(registry.destroy_session("mc-main"));
    println!("after_close_main: {}", registry.session_count());
    assert!(registry.destroy_session("mc-second"));
    println!("after_close_second: {}", registry.session_count());
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let registry = PtySessionRegistry::new();

    tauri::Builder::default()
        .manage(registry)
        .invoke_handler(tauri::generate_handler![
            resolve_mc_path,
            dry_run_mc,
            spawn_mc,
            write_pty,
            resize_pty,
            close_pty_session,
            list_pty_sessions,
            get_pty_session_count,
            new_mc_window,
        ])
        .setup(|app| {
            if let Ok(menu) = build_menu(app.handle()) {
                app.set_menu(menu)?;
            }

            if verify_live_multi_window_enabled() {
                spawn_live_multi_window_verification(app.handle().clone());
            }

            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id().0 == "new_window" {
                let _ = new_mc_window(app.clone());
            }
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let label = window.label().to_string();
                if let Some(registry) = window.app_handle().try_state::<PtySessionRegistry>() {
                    registry.destroy_session(&label);
                }
                // No frontend onCloseRequested listener — native close proceeds automatically.
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
                if let Some(registry) = app_handle.try_state::<PtySessionRegistry>() {
                    let labels: Vec<String> = registry
                        .list_sessions()
                        .into_iter()
                        .map(|session| session.window_label)
                        .collect();
                    for label in labels {
                        registry.destroy_session(&label);
                    }
                }
            }
        });
}