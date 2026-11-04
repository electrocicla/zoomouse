//! User-session desktop shell. No listener, service, elevation or capture.
mod desktop_observer;

use std::sync::Mutex;
use std::time::Duration;

use desktop_observer::{observe, DesktopObservation};

use serde::Serialize;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, State};
use zoomouse_host::{ConsentPhase, DisabledInput, HostError, HostSession, InputBackend};

/// This shell has no native channel or approval constructor. Even a renderer
/// asking for consent cannot change this trusted Rust-side policy.
const APPROVAL_GATE: &str = "missingVerifiedChannelAndNativeApproval";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
enum ReleaseOutcome {
    NotAttempted,
    Confirmed,
    Failed,
}

/// The only mutable local state is host-owned; renderer IPC cannot mutate
/// consent, channel evidence, desktop authority, or the release outcome.
struct LocalHostState<B: InputBackend> {
    session: HostSession<B>,
    last_release: ReleaseOutcome,
}

impl<B: InputBackend> LocalHostState<B> {
    fn new(backend: B) -> Self {
        Self {
            session: HostSession::new(backend),
            last_release: ReleaseOutcome::NotAttempted,
        }
    }

    fn stop(&mut self) -> Result<(), HostError> {
        // Record failures even when the UI cannot receive an IPC reply.
        let result = self.session.revoke();
        self.last_release = if result.is_ok() {
            ReleaseOutcome::Confirmed
        } else {
            ReleaseOutcome::Failed
        };
        result
    }

    // Loss of the local input desktop or console session revokes and releases.
    // Do not overwrite a latched cleanup failure on repeat negative samples.
    fn observe_desktop(&mut self, desktop: DesktopObservation) {
        if desktop == DesktopObservation::Unavailable
            && self.session.phase() != ConsentPhase::Revoked
        {
            let _ = self.stop();
        }
    }

    fn status(&self, desktop: DesktopObservation) -> HostStatus {
        HostStatus {
            phase: phase_label(self.session.phase()),
            input_enabled: false,
            cleanup_failed: self.session.cleanup_failed()
                || self.last_release == ReleaseOutcome::Failed,
            discovery_enabled: false,
            capture_enabled: false,
            local_user_session_only: false,
            desktop_observation: desktop,
            approval_gate: APPROVAL_GATE,
            last_release_outcome: self.last_release,
        }
    }
}

type LocalHost = Mutex<LocalHostState<DisabledInput>>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HostStatus {
    phase: &'static str,
    input_enabled: bool,
    cleanup_failed: bool,
    discovery_enabled: bool,
    capture_enabled: bool,
    local_user_session_only: bool,
    desktop_observation: DesktopObservation,
    approval_gate: &'static str,
    last_release_outcome: ReleaseOutcome,
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
    // Native Win32 reads are independent of renderer/network claims.
    let desktop = observe().unwrap_or(DesktopObservation::Unavailable);
    let mut current = host
        .lock()
        .map_err(|_| "host lock unavailable".to_owned())?;
    current.observe_desktop(desktop);
    Ok(current.status(desktop))
}

/// Explicitly exposed UI command: deny/release, never grant.
#[tauri::command]
fn emergency_stop(host: State<'_, LocalHost>) -> Result<(), String> {
    let mut current = host
        .lock()
        .map_err(|_| "host lock unavailable".to_owned())?;
    current.stop().map_err(|error| error.to_string())
}

fn try_revoke(state: &LocalHost) -> bool {
    match state.lock() {
        Ok(mut host) => host.stop().is_ok(),
        Err(_) => false,
    }
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
        .manage(Mutex::new(LocalHostState::new(DisabledInput)))
        .invoke_handler(tauri::generate_handler![host_status, emergency_stop])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Closing settings must not silently terminate a running tray host.
                // Only the explicit Quit tray action attempts revocation and exit.
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "Open Zoomouse", true, None::<&str>)?;
            let revoke =
                MenuItem::with_id(app, "revoke", "Emergency stop / revoke", true, None::<&str>)?;
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
                        // Never discard a native release failure while hidden in the tray.
                        let state = app.state::<LocalHost>();
                        let cleanup_ok = try_revoke(state.inner());
                        if !cleanup_ok {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    } else if event.id() == "quit" {
                        // A native key-release failure must not be hidden by exit.
                        let state = app.state::<LocalHost>();
                        let cleanup_ok = try_revoke(state.inner());
                        if cleanup_ok {
                            app.exit(0);
                        } else if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // The UI may be hidden to tray: native desktop-loss monitoring
            // cannot depend on an IPC status request or renderer focus.
            let observer_app = app.handle().clone();
            std::thread::Builder::new()
                .name("zoomouse-desktop-guard".into())
                .spawn(move || loop {
                    std::thread::sleep(Duration::from_millis(500));
                    let desktop = observe().unwrap_or(DesktopObservation::Unavailable);
                    let state = observer_app.state::<LocalHost>();
                    if let Ok(mut host) = state.lock() {
                        host.observe_desktop(desktop);
                    };
                })?;
            Ok(())
        })
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zoomouse_host::InputAction;

    struct FailingRelease;

    impl InputBackend for FailingRelease {
        fn dispatch(&mut self, _action: &InputAction) -> Result<(), HostError> {
            Err(HostError::Unauthorized)
        }

        fn release_all(&mut self) -> Result<(), HostError> {
            Err(HostError::CleanupFailed)
        }
    }

    #[test]
    fn local_shell_never_advertises_approval_or_a_verified_user_session() {
        let mut host = LocalHostState::new(DisabledInput);
        let status = host.status(DesktopObservation::Unavailable);
        assert_eq!(status.phase, "Denied");
        assert_eq!(status.approval_gate, APPROVAL_GATE);
        assert!(!status.input_enabled);
        assert!(!status.local_user_session_only);
        assert!(!status.discovery_enabled);
        assert!(!status.capture_enabled);
        assert_eq!(status.last_release_outcome, ReleaseOutcome::NotAttempted);
        host.stop().expect("disabled backend safely releases");
        assert_eq!(
            host.status(DesktopObservation::Unavailable).phase,
            "Revoked"
        );
        assert_eq!(
            host.status(DesktopObservation::Unavailable)
                .last_release_outcome,
            ReleaseOutcome::Confirmed
        );
    }

    #[test]
    fn observed_interactive_desktop_cannot_grant_access() {
        let host = LocalHostState::new(DisabledInput);
        let status = host.status(DesktopObservation::InteractiveDefault);
        assert_eq!(
            status.desktop_observation,
            DesktopObservation::InteractiveDefault
        );
        assert_eq!(status.phase, "Denied");
        assert!(!status.local_user_session_only);
        assert!(!status.input_enabled);
        assert_eq!(status.approval_gate, APPROVAL_GATE);
    }

    #[test]
    fn desktop_loss_revokes_without_erasing_cleanup_failure() {
        let mut host = LocalHostState::new(FailingRelease);
        host.observe_desktop(DesktopObservation::InteractiveDefault);
        assert_eq!(host.session.phase(), ConsentPhase::Denied);
        host.observe_desktop(DesktopObservation::Unavailable);
        assert_eq!(host.session.phase(), ConsentPhase::Revoked);
        assert_eq!(host.last_release, ReleaseOutcome::Failed);
        host.observe_desktop(DesktopObservation::Unavailable);
        assert_eq!(host.last_release, ReleaseOutcome::Failed);
        assert!(host.status(DesktopObservation::Unavailable).cleanup_failed);
    }

    #[test]
    fn native_release_failure_latches_diagnostics_and_never_restores_access() {
        let mut host = LocalHostState::new(FailingRelease);
        assert!(host.stop().is_err());
        let status = host.status(DesktopObservation::Unavailable);
        assert_eq!(status.phase, "Revoked");
        assert_eq!(status.last_release_outcome, ReleaseOutcome::Failed);
        assert!(status.cleanup_failed);
        assert!(!status.input_enabled);
    }
}
