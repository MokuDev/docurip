pub mod commands;
pub mod crawler;
pub mod export;
pub mod fetcher;
pub mod parser;
pub mod converter;
pub mod writer;
pub mod asset_dl;
pub mod events;
pub mod settings;
pub mod state;

use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let persist_dir = app.path().app_data_dir()?.join("jobs");
            let app_state = Arc::new(state::AppState::init(persist_dir)?);
            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_crawl,
            commands::stop_crawl,
            commands::get_job,
            commands::list_jobs,
            commands::delete_job,
            commands::get_settings,
            commands::update_settings,
            commands::open_output_folder,
            commands::export_job,
            commands::search_job_results,
            commands::export_job_zip,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}