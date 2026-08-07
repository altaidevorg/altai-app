use serde::Serialize;
use tauri::{
    menu::{Menu, MenuEvent, MenuItem, MenuItemBuilder, Submenu, SubmenuBuilder},
    AppHandle, Emitter, Manager, Wry,
};

use super::{os_menu, os_menu::RecentFolders};

pub const MENU_COMMAND_EVENT: &str = "altai:menu-command";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MenuCommand {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

fn item(
    app: &AppHandle,
    id: &str,
    text: &str,
    accelerator: Option<&str>,
) -> tauri::Result<MenuItem<Wry>> {
    let mut builder = MenuItemBuilder::with_id(id, text);
    if let Some(accelerator) = accelerator {
        builder = builder.accelerator(accelerator);
    }
    builder.build(app)
}

fn recent_menu(app: &AppHandle, recents: &[String]) -> tauri::Result<Submenu<Wry>> {
    let mut menu = SubmenuBuilder::new(app, "Open Recent");
    if recents.is_empty() {
        let empty = MenuItemBuilder::with_id("file.openRecent.empty", "No Recent Projects")
            .enabled(false)
            .build(app)?;
        menu = menu.item(&empty);
    } else {
        for (index, path) in recents.iter().take(10).enumerate() {
            let label = std::path::Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .unwrap_or(path);
            let recent = item(app, &format!("file.openRecent.{index}"), label, None)?;
            menu = menu.item(&recent);
        }
    }
    menu.build()
}

pub fn build(app: &AppHandle, recents: &[String]) -> tauri::Result<Menu<Wry>> {
    let settings = item(app, "app.settings", "Settings…", Some("CmdOrCtrl+,"))?;
    let shortcuts = item(
        app,
        "app.shortcuts",
        "Keyboard Shortcuts…",
        Some("CmdOrCtrl+K"),
    )?;
    let app_menu = SubmenuBuilder::new(app, "ALTAI")
        .about_with_text("About ALTAI", None)
        .separator()
        .item(&settings)
        .item(&shortcuts)
        .separator()
        .services()
        .separator()
        .hide_with_text("Hide ALTAI")
        .hide_others()
        .show_all()
        .separator()
        .quit_with_text("Quit ALTAI")
        .build()?;

    let new_file = item(app, "file.newFile", "New File…", Some("CmdOrCtrl+N"))?;
    let new_window = item(app, "window.new", "New Window", Some("CmdOrCtrl+Shift+N"))?;
    let open_folder = item(app, "file.openFolder", "Open Folder…", Some("CmdOrCtrl+O"))?;
    let close_workspace = item(app, "file.closeWorkspace", "Close Workspace", None)?;
    let save = item(app, "file.save", "Save", Some("CmdOrCtrl+S"))?;
    let close_editor = item(
        app,
        "file.closeEditor",
        "Close Editor or Terminal",
        Some("CmdOrCtrl+W"),
    )?;
    let recent = recent_menu(app, recents)?;
    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&new_file)
        .item(&new_window)
        .separator()
        .item(&open_folder)
        .item(&recent)
        .separator()
        .item(&save)
        .separator()
        .item(&close_editor)
        .item(&close_workspace)
        .build()?;

    let find = item(app, "edit.find", "Find", Some("CmdOrCtrl+F"))?;
    let find_in_files = item(
        app,
        "edit.findInFiles",
        "Find in Files",
        Some("CmdOrCtrl+Shift+F"),
    )?;
    let toggle_comment = item(
        app,
        "edit.toggleComment",
        "Toggle Line Comment",
        Some("CmdOrCtrl+/"),
    )?;
    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .item(&find)
        .item(&find_in_files)
        .separator()
        .item(&toggle_comment)
        .build()?;

    let explorer = item(app, "view.explorer", "Explorer", Some("CmdOrCtrl+Shift+E"))?;
    let source_control = item(
        app,
        "view.sourceControl",
        "Source Control",
        Some("CmdOrCtrl+Shift+G"),
    )?;
    let agent = item(app, "view.agent", "AI Agent", Some("CmdOrCtrl+I"))?;
    let terminal = item(app, "view.terminal", "Terminal", Some("CmdOrCtrl+J"))?;
    let sidebar = item(
        app,
        "view.sidebar",
        "Toggle Primary Side Bar",
        Some("CmdOrCtrl+B"),
    )?;
    let zoom_in = item(app, "view.zoomIn", "Zoom In", Some("CmdOrCtrl+="))?;
    let zoom_out = item(app, "view.zoomOut", "Zoom Out", Some("CmdOrCtrl+-"))?;
    let zoom_reset = item(app, "view.zoomReset", "Reset Zoom", Some("CmdOrCtrl+0"))?;
    let appearance = SubmenuBuilder::new(app, "Appearance")
        .item(&sidebar)
        .item(&agent)
        .item(&terminal)
        .separator()
        .item(&zoom_in)
        .item(&zoom_out)
        .item(&zoom_reset)
        .build()?;
    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&explorer)
        .item(&source_control)
        .item(&appearance)
        .build()?;

    let next_tab = item(app, "go.nextTab", "Next Tab", Some("Ctrl+Tab"))?;
    let previous_tab = item(
        app,
        "go.previousTab",
        "Previous Tab",
        Some("Ctrl+Shift+Tab"),
    )?;
    let next_pane = item(app, "go.nextPane", "Focus Next Pane", Some("CmdOrCtrl+]"))?;
    let previous_pane = item(
        app,
        "go.previousPane",
        "Focus Previous Pane",
        Some("CmdOrCtrl+["),
    )?;
    let go_menu = SubmenuBuilder::new(app, "Go")
        .item(&next_tab)
        .item(&previous_tab)
        .separator()
        .item(&next_pane)
        .item(&previous_pane)
        .build()?;

    let terminal_new = item(app, "terminal.new", "New Terminal", Some("CmdOrCtrl+T"))?;
    let terminal_new_private = item(
        app,
        "terminal.newPrivate",
        "New Private Terminal",
        Some("CmdOrCtrl+R"),
    )?;
    let split_terminal = item(
        app,
        "terminal.split",
        "Split Terminal",
        Some("CmdOrCtrl+\\"),
    )?;
    let toggle_terminal = item(app, "terminal.toggle", "Toggle Terminal", None)?;
    let terminal_menu = SubmenuBuilder::new(app, "Terminal")
        .item(&terminal_new)
        .item(&terminal_new_private)
        .item(&split_terminal)
        .separator()
        .item(&toggle_terminal)
        .build()?;

    let open_ide = item(app, "window.openIde", "Open IDE", None)?;
    let window_menu = SubmenuBuilder::new(app, "Window")
        .item(&open_ide)
        .separator()
        .minimize()
        .maximize()
        .fullscreen()
        .separator()
        .bring_all_to_front()
        .build()?;

    let help_shortcuts = item(app, "help.shortcuts", "Keyboard Shortcuts…", None)?;
    let github = item(app, "help.github", "ALTAI on GitHub", None)?;
    let help_menu = SubmenuBuilder::new(app, "Help")
        .item(&help_shortcuts)
        .separator()
        .item(&github)
        .build()?;

    tauri::menu::MenuBuilder::new(app)
        .item(&app_menu)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .item(&go_menu)
        .item(&terminal_menu)
        .item(&window_menu)
        .item(&help_menu)
        .build()
}

pub fn install(app: &AppHandle, recents: &[String]) -> tauri::Result<()> {
    app.set_menu(build(app, recents)?)?;
    Ok(())
}

fn emit_command(app: &AppHandle, id: &str, path: Option<String>) {
    let command = MenuCommand {
        id: id.to_string(),
        path,
    };

    let windows = app.webview_windows();
    let target = windows
        .values()
        .find(|window| window.is_focused().unwrap_or(false))
        .or_else(|| windows.get("main"));

    if let Some(window) = target {
        let _ = window.emit(MENU_COMMAND_EVENT, command);
    }
}

fn is_ide_window_label(label: &str) -> bool {
    label == "studio" || label.starts_with("main-")
}

pub fn handle_event(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref();
    match id {
        "window.new" => os_menu::spawn_new_window(app),
        "window.openIde" => {
            if let Err(error) = crate::show_or_create_studio_window(app, None) {
                log::error!("Could not open ALTAI IDE window: {error}");
            }
        }
        "app.settings" => {
            // Studio settings are a native app-level window. IDE settings
            // remain the existing in-IDE tab and still flow through React.
            let ide_is_focused = app.webview_windows().iter().any(|(label, window)| {
                is_ide_window_label(label) && window.is_focused().unwrap_or(false)
            });
            if ide_is_focused {
                emit_command(app, id, None);
            } else if let Err(error) = crate::show_or_create_settings_window(
                app,
                "settings",
                "ALTAI Studio Settings",
                "app",
                None,
            ) {
                log::error!("Could not open ALTAI Studio settings: {error}");
            }
        }
        "help.github" => {
            let _ = tauri_plugin_opener::open_url(
                "https://github.com/altaidevorg/altai-app",
                None::<&str>,
            );
        }
        _ if id.starts_with("file.openRecent.") => {
            let Some(index) = id
                .strip_prefix("file.openRecent.")
                .and_then(|value| value.parse::<usize>().ok())
            else {
                return;
            };
            let path = app
                .state::<RecentFolders>()
                .0
                .lock()
                .ok()
                .and_then(|recents| recents.get(index).cloned());
            if path.is_some() {
                emit_command(app, "file.openRecent", path);
            }
        }
        _ => emit_command(app, id, None),
    }
}

#[cfg(test)]
mod tests {
    use super::is_ide_window_label;

    #[test]
    fn classifies_singleton_and_fresh_ide_windows() {
        assert!(is_ide_window_label("studio"));
        assert!(is_ide_window_label("main-0198f3f7"));
        assert!(!is_ide_window_label("main"));
        assert!(!is_ide_window_label("settings"));
    }
}
