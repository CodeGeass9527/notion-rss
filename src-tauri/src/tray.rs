// src-tauri/src/tray.rs
use crate::rss::update;
use crate::ui::create_window;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager
};

pub struct MyTray {}

impl MyTray {
    pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        let update_item = MenuItemBuilder::with_id("update", "Update").build(app)?;
        let open_window_item = MenuItemBuilder::with_id("open_window", "Dashboard").build(app)?;
        let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

        let menu = MenuBuilder::new(app)
            .items(&[&update_item, &open_window_item, &quit_item])
            .build()?;

        let tray = TrayIconBuilder::new()
            .menu(&menu)
            .on_menu_event(move |app, event| match event.id().as_ref() {
                "quit" => {
                    app.exit(0);
                    std::process::exit(0);
                }
                "update" => {
                    let h = app.clone();
                    tauri::async_runtime::spawn(async move {
                        update(h.get_webview_window("main")).await; // 使用 get_webview_window
                    });
                }
                "open_window" => create_window(app),
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    let app = tray.app_handle();
                    if let Some(webview_window) = app.get_webview_window("main") { // 使用 get_webview_window
                        let _ = webview_window.show();
                        let _ = webview_window.set_focus();
                    }
                }
            })
            .build(app)?;

        Ok(())
    }
}