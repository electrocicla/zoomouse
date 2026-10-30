//! User-session desktop shell. No listener, service, elevation or capture.
use std::sync::Mutex;

use serde::Serialize;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, State};
use zoomouse_host::{ConsentPhase, DisabledInput, HostSession};

type LocalHost = Mutex<HostSession<DisabledInput>>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HostStatus {
    phase: &'static str,
    input_enabled: bool,
    cleanup_failed: bool,
    discovery_enabled: bool,
    capture_enabled: bool,
    local_user_session_only: bool,
}

fn phase_label(phase: ConsentPhase) -> &'static str {
    match phase {
        ConsentPhase::Denied => "Denied",
        ConsentPhase::Pending => "Pending",
        ConsentPhase::Approved => "Approved",
        ConsentPhase::Revoked => "Revoked",
    }
}

#[tauri::command]
fn host_status(host: State<'_, LocalHost>) -> Result<HostStatus, String> {
    let current = host.lock().map_err(|_| "host lock unavailable".to_owned())?;
    Ok(HostStatus {
        phase: phase_label(current.phase()),
        input_enabled: false,
        cleanup_failed: current.cleanup_failed(),
        discovery_enabled: false,
        capture_enabled: false,
        local_user_session_only: true,
    })
}

/// Explicitly exposed UI command: deny/release, never grant.
#[tauri::command]
fn emergency_stop(host: State<'_, LocalHost>) -> Result<(), String> {
    let mut current = host.lock().map_err(|_| "host lock unavailable".to_owned())?;
    current.revoke().map_err(|error| error.to_string())
}

fn tray_icon() -> Image<'static> {
    let mut rgba = Vec::with_capacity(16 * 16 * 4);
    for y in 0..16 {
        for x in 0..16 {
            let active = (3..=12).contains(&x) && (3..=12).contains(&y);
            rgba.extend_from_slice(if active {
                &[105, 222, 200, 255]
            } else {
                &[16, 26, 41, 255]
            });
        }
    }
    Image::new_owned(rgba, 16, 16)
}

pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .manage(Mutex::new(HostSession::new(DisabledInput)))
        .invoke_handler(tauri::generate_handler![host_status, emergency_stop])
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "Open Zoomouse", true, None::<&str>)?;
            let revoke = MenuItem::with_id(app, "revoke", "Emergency stop / revoke", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Zoomouse", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &revoke, &quit])?;

            TrayIconBuilder::new()
                .icon(tray_icon())
                .tooltip("Zoomouse — remote access disabled")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id() == "show" {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    } else if event.id() == "revoke" {
                        let state = app.state::<LocalHost>();
                        if let Ok(mut host) = state.lock() {
                            let _ = host.revoke();
                        }
                    } else if event.id() == "quit" {
                        // A native key-release failure must not be hidden by exit.
                        let state = app.state::<LocalHost>();
                        let cleanup_ok = match state.lock() {
                            Ok(mut host) => host.revoke().is_ok(),
                            Err(_) => false,
                        };
                        if cleanup_ok {
                            app.exit(0);
                        } else if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
}
