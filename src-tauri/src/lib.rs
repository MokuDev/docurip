pub mod commands;
pub mod crawler;
pub mod export;
pub mod exports;
pub mod fetcher;
pub mod parser;
pub mod converter;
pub mod writer;
pub mod asset_dl;
pub mod events;
pub mod settings;
pub mod state;
pub mod system;
pub mod importer;

use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let persist_dir = app.path().app_data_dir()?.join("jobs");
            let app_state = Arc::new(state::AppState::init(persist_dir)?);
            app.manage(app_state);

            use tauri_plugin_store::StoreExt;
            if let Ok(store) = app.store("settings.json") {
                if let Some(val) = store.get("settings") {
                    if let Ok(s) = serde_json::from_value::<settings::config::AppSettings>(val) {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
                                s.window_width as f64,
                                s.window_height as f64,
                            )));
                            let _ = win.center();
                        }
                    }
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_crawl,
            commands::stop_crawl,
            commands::pause_crawl,
            commands::resume_crawl,
            commands::get_job,
            commands::list_jobs,
            commands::get_dashboard_stats,
            commands::delete_job,
            commands::get_settings,
            commands::update_settings,
            commands::set_theme,
            commands::open_output_folder,
            commands::export_job,
            commands::export_job_v2,
            commands::check_headless_support,
            commands::read_page_content,
            commands::search_job_results,
            commands::export_job_zip,
            commands::list_exports,
            commands::get_system_stats,
            commands::get_session_info,
            commands::set_window_size,
            commands::import_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}