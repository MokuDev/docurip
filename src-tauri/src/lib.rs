pub mod commands;
pub mod crawler;
pub mod fetcher;
pub mod parser;
pub mod converter;
pub mod writer;
pub mod asset_dl;
pub mod events;
pub mod settings;
pub mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::start_crawl,
            commands::stop_crawl,
            commands::get_job,
            commands::list_jobs,
            commands::get_settings,
            commands::update_settings,
            commands::open_output_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}