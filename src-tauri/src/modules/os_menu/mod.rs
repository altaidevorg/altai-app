//! OS-level taskbar / Dock / Start-menu integration: a right-click menu on the
//! app icon offering "New Window" and the recently-opened workspace folders.
//!
//! The recents are the same list the welcome screen shows — the frontend
//! mirrors them here via the `set_recent_folders` command whenever they change.
//! There is no high-level Tauri API for any of these surfaces, so each platform
//! drops to native code (see the platform submodules):
//!
//! - macOS: a Dock menu via `[NSApp setDockMenu:]`.
//! - Windows: a taskbar Jump List via the Shell COM APIs.
//! - Linux: a static `.desktop` "New Window" action baked at bundle time; a
//!   launcher menu cannot list recent folders dynamically.

use std::sync::Mutex;
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

use super::app_menu;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// The most-recently-opened workspace folders, newest first, mirrored from the
/// frontend store so the native menus can list them. Lives in Tauri state.
#[derive(Default)]
pub struct RecentFolders(pub Mutex<Vec<String>>);

/// Generate the label for an independently-created IDE window.
///
/// `studio` is intentionally reserved for the singleton Studio window, while
/// `main-*` is granted the same window capabilities as the primary IDE.
fn new_ide_window_label() -> String {
    format!("main-{}", uuid::Uuid::new_v4().simple())
}

/// Open a fresh ALTAI IDE window.
///
/// The primary `main` window is the singleton Agent Workspace / Studio
/// surface. Every explicit "New Window" action instead creates an independent
/// IDE page, matching desktop-editor conventions without duplicating Studio.
/// Shared by every entry point: the single-instance `--new-window` relaunch,
/// the macOS Dock item, and the frontend command.
pub fn spawn_new_window(app: &AppHandle) {
    // A unique label is mandatory (Tauri rejects duplicates). The `main-`
    // prefix matches the capability glob (`main-*`) so the new window inherits
    // the same plugin permissions as the primary one.
    let label = new_ide_window_label();

    let builder = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::App("index.html?mode=studio".into()),
    )
    .title("ALTAI IDE")
    .inner_size(1280.0, 800.0)
    .min_inner_size(900.0, 600.0)
    .focused(true);

    // Mirror the IDE chrome in `show_or_create_studio_window`: the overlay
    // titlebar on macOS, app-owned chrome on Linux, and native chrome on Windows.
    // The native traffic lights and the IDE's 40px header share a centerline.
    // with_webview_configuration opts out of Apple Intelligence Writing Tools.
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(tauri::LogicalPosition::new(16.0, 22.0))
        .with_webview_configuration(super::macos_webview::config_without_writing_tools());

    #[cfg(target_os = "linux")]
    let builder = builder.decorations(false).transparent(true);
    #[cfg(target_os = "windows")]
    let builder = builder
        .decorations(true)
        .transparent(false)
        .additional_browser_args(crate::WINDOWS_WEBVIEW_ARGS);

    match builder.build() {
        Ok(_window) => {
            // Some Linux window managers ignore the builder-time flag.
            #[cfg(target_os = "linux")]
            let _ = _window.set_decorations(false);
            #[cfg(target_os = "windows")]
            let _ = _window.set_decorations(true);
            let _ = crate::focus_webview_window(&_window);
        }
        Err(e) => log::error!("os_menu: failed to open new window: {e}"),
    }
}

/// Rebuild the native menu from the current recents. No-op on Linux, whose
/// launcher actions are static and baked into the `.desktop` file at bundle time.
fn rebuild(app: &AppHandle, recents: &[String]) {
    #[cfg(target_os = "macos")]
    macos::set_dock_menu(app, recents);
    #[cfg(target_os = "windows")]
    windows::set_jump_list(app, recents);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = (app, recents);
}

/// Build the menu once at startup. Recents are empty until the frontend pushes
/// them, but we still want "New Window" available immediately (macOS).
pub fn init(app: &AppHandle) {
    rebuild(app, &[]);
}

/// Mirror the frontend's recent-folders list and rebuild the native menu.
#[tauri::command]
pub fn set_recent_folders(
    app: AppHandle,
    folders: Vec<String>,
    state: tauri::State<'_, RecentFolders>,
) {
    if let Ok(mut guard) = state.0.lock() {
        *guard = folders.clone();
    }
    rebuild(&app, &folders);
    if let Err(error) = app_menu::install(&app, &folders) {
        log::error!("app_menu: failed to refresh recent folders: {error}");
    }
}

/// Open a fresh IDE window. Callable from the frontend too.
#[tauri::command]
pub fn open_new_window(app: AppHandle) {
    spawn_new_window(&app);
}

#[cfg(test)]
mod tests {
    use super::new_ide_window_label;

    #[test]
    fn new_ide_windows_never_reuse_the_singleton_studio_label() {
        let label = new_ide_window_label();

        assert!(label.starts_with("main-"));
        assert_ne!(label, "studio");
    }

    #[test]
    fn each_new_ide_window_gets_its_own_label() {
        assert_ne!(new_ide_window_label(), new_ide_window_label());
    }
}
