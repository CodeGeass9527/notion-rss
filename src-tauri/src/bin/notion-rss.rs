//! src-tauri/src/bin/notion-rss.rs
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
use notion_rss::tray::MyTray;
use notion_rss::ui::resolve_setup;
use tauri::{Manager};
use actix_web::rt::time::sleep;
const BANNER: &str = r#"
███╗   ██╗ ██████╗ ████████╗██╗ ██████╗ ███╗   ██╗      ██████╗ ███████╗███████╗
████╗  ██║██╔═══██╗╚══██╔══╝██║██╔═══██╗████╗  ██║      ██╔══██╗██╔════╝██╔════╝
██╔██╗ ██║██║   ██║   ██║   ██║██║   ██║██╔██╗ ██║█████╗██████╔╝███████╗███████╗
██║╚██╗██║██║   ██║   ██║   ██║██║   ██║██║╚██╗██║╚════╝██╔══██╗╚════██║╚════██║
██║ ╚████║╚██████╔╝   ██║   ██║╚██████╔╝██║ ╚████║      ██║  ██║███████║███████║
╚═╝  ╚═══╝ ╚═════╝    ╚═╝   ╚═╝ ╚═════╝ ╚═╝  ╚═══╝      ╚═╝  ╚═╝╚══════╝╚══════╝
Build your own RSS Feeds in Notion.
________________________________________________
:  https://github.com/cn-kali-team/notion-rss  :
:  https://blog.kali-team.cn/donate            :
 -----------------------------------------------
"#;

fn main() -> Result<()> {
    println!("{}", BANNER);
    let config = NotionConfig::default();

    // 处理订阅文件
    if let Some(p) = &config.file {
        for f in read_file_to_feed(p) {
            match tauri::async_runtime::block_on(add_subscribe(f)) {
                Ok(t) => println!("Submitted Successfully: {}.", t),
                Err(e) => println!("Submitted Failed: {}.", e),
            }
        }
        tauri::async_runtime::block_on(update(None));
    }

    if config.deleted {
        tauri::async_runtime::block_on(deleted());
    }

    if config.api_server.is_some() {
        run_server(None);
    }

    start(config);
    thread::sleep(Duration::from_secs(10));
    Ok(())
}

fn start(config: NotionConfig) {
    let builder = tauri::Builder::default()
        .setup(move|app| {
            resolve_setup(app);
            let handle = app.app_handle();
            tauri::async_runtime::spawn(async move {
                loop {
                    update(None).await;
                    sleep(Duration::from_secs(60 * 60 * config.hour)).await;
                }
            });

            // 这里调用 MyTray::setup_tray
            MyTray::setup_tray(&handle).expect("Failed to setup system tray");

            Ok(())
        })
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

    let app_handle = app.handle(); // 获取 app_handle

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
}