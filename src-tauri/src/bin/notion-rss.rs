#![cfg_attr(
    all(not(debug_assertions), target_os = "windows", not(feature = "cli")),
    windows_subsystem = "windows"
)]

use anyhow::Result;
use notion_rss::api::run_server;
use std::thread;
use std::time::Duration;

use notion_rss::cli::NotionConfig;
use notion_rss::read_file_to_feed;
use notion_rss::rss::{add_subscribe, deleted, update};
#[cfg(not(feature = "cli"))]
use notion_rss::tray::MyTray;
#[cfg(not(feature = "cli"))]
use notion_rss::ui::resolve_setup;
#[cfg(not(feature = "cli"))]
use tauri::Manager;

const BANNER: &str = r#"
███╗   ██╗ ██████╗ ████████╗██╗ ██████╗ ███╗   ██╗      ██████╗ ███████╗███████╗
████╗  ██║██╔═══██╗╚══██╔══╝██║██╔═══██╗████╗  ██║      ██╔══██╗██╔════╝██╔════╝
██╔██╗ ██║██║   ██║   ██║   ██║██║   ██║██╔██╗ ██║█████╗██████╔╝███████╗███████╗
██║╚██╗██║██║   ██║   ██║   ██║██║   ██║██║╚██╗██║╚════╝██╔══██╗╚════██║╚════██║
██║ ╚████║╚██████╔╝   ██║   ██║╚██████╔╝██║ ╚████║      ██║  ██║███████║███████║
╚═╝  ╚═══╝ ╚═════╝    ╚═╝   ╚═╝ ╚═════╝ ╚═╝  ╚═══╝      ╚═╝  ╚═╝╚══════╝╚══════╝
Build your own RSS Feeds in Notion.
________________________________________________
:  https://github.com/CodeGeass9527/notion-rss  :
 -----------------------------------------------
"#;

#[tokio::main]
async fn main() -> Result<()> {
    println!("{}", BANNER);
    std::env::set_var("RUST_LOG", "notion_sdk=debug");
    let config = NotionConfig::default();
    // add subscribe from file
    if let Some(p) = &config.file {
        for f in read_file_to_feed(p) {
            match add_subscribe(f).await {
                Ok(t) => {
                    println!("Submitted Successfully: {}.", t);
                }
                Err(e) => {
                    println!("Submitted Failed: {}.", e);
                }
            }
        }
        #[cfg(feature = "cli")]
        update().await;
        #[cfg(not(feature = "cli"))]
        update(None).await;
    }
    if config.deleted {
        deleted().await;
    }
    if config.api_server.is_some() {
        #[cfg(feature = "cli")]
        run_server();
        #[cfg(not(feature = "cli"))]
        run_server(None);
    }
    start(config).await;
    thread::sleep(Duration::from_secs(10));
    Ok(())
}

#[cfg(feature = "cli")]
async fn start(config: NotionConfig) {
    // Scheduled update
    if config.daemon && config.api_server.is_some() {
        loop {
            update().await;
            thread::sleep(Duration::from_secs(60 * 60 * config.hour));
        }
    } else {
        update().await;
    }
}

#[cfg(not(feature = "cli"))]
async fn start(config: NotionConfig) {
    if !config.cli {
        let builder = tauri::Builder::default()
            .system_tray(tauri::SystemTray::new().with_menu(MyTray::tray_menu()))
            .setup(|app| {
                resolve_setup(app);
                Ok(())
            })
            .on_system_tray_event(MyTray::on_system_tray_event)
            .invoke_handler(tauri::generate_handler![
                notion_rss::ui::save_config,
                notion_rss::ui::init_config,
                notion_rss::ui::init_user,
                notion_rss::ui::update_once,
                notion_rss::ui::run_api_server,
                notion_rss::ui::add_feed,
                notion_rss::ui::import_feed
            ]);
        let app = builder
            .build(tauri::generate_context!())
            .expect("error while running tauri application");
        let w = app.app_handle().get_window("main");
        tauri::async_runtime::spawn(async move {
            loop {
                update(w.clone()).await;
                thread::sleep(Duration::from_secs(60 * 60 * config.hour));
            }
        });
        app.run(|app_handle, e| match e {
            tauri::RunEvent::ExitRequested { api, .. } => {
                api.prevent_exit();
            }
            tauri::RunEvent::Exit => {
                app_handle.exit(0);
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::WindowEvent { label, event, .. } => {
                if label == "main" {
                    match event {
                        tauri::WindowEvent::CloseRequested { api, .. } => {
                            api.prevent_close();
                            app_handle.get_window("main").map(|win| {
                                let _ = win.hide();
                            });
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        });
        // Scheduled update
    } else {
        update(None).await;
    }
}
