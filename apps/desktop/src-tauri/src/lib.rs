mod commands;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new().expect("failed to init app state"))
        .invoke_handler(tauri::generate_handler![
            commands::process_image_path,
            commands::process_image_bytes,
            commands::prepare_asset,
            commands::steam_doctor,
            commands::publish_asset,
            commands::list_projects,
            commands::get_project,
            commands::list_history,
            commands::save_project,
            commands::app_paths,
            commands::wipe_local_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
