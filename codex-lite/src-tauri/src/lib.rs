mod commands;
mod infra;
mod models;
mod services;

#[cfg(test)]
mod test_support;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    infra::logger::init_logger();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::account::list_codex_accounts,
            commands::account::get_current_codex_account,
            commands::account::delete_codex_account,
            commands::account::update_codex_api_key_account,
            commands::account::switch_codex_account,
            commands::import::import_codex_from_local,
            commands::import::import_codex_from_json,
            commands::import::import_codex_from_files,
            commands::import::start_codex_batch_import_from_files,
            commands::import::confirm_codex_batch_import,
            commands::import::add_codex_account_with_token,
            commands::import::add_codex_account_with_api_key,
            commands::oauth::codex_oauth_login_start,
            commands::oauth::codex_oauth_submit_callback_url,
            commands::oauth::codex_oauth_login_status,
            commands::oauth::codex_oauth_login_completed,
            commands::oauth::codex_oauth_login_cancel,
            commands::oauth::is_codex_oauth_port_in_use,
            commands::quota::refresh_codex_quota,
            commands::quota::refresh_all_codex_quotas,
            commands::session::list_codex_sessions,
            commands::session::restore_codex_sessions_visibility,
            commands::session::delete_codex_sessions,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::detect_codex_paths,
            commands::system::open_data_dir,
            commands::system::open_log_dir,
            commands::system::get_log_snapshot,
            commands::system::get_system_snapshot,
            commands::window::window_start_dragging,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Codex Lite");
}
