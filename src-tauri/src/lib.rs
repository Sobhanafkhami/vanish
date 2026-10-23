mod commands;
mod config;
mod engine;
mod nodes;
mod proxy;
mod state;
mod tray;

use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // A second launch must not spawn a second process: two instances would fight over
        // the data files and the sing-box ports. Focus the running one instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main(app)))
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(dir.join("bin"))?;
            app.manage(state::AppState::load(dir));
            tray::init(app.handle())?;
            Ok(())
        })
        // Closing the window minimizes to tray; use the tray menu to quit.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_nodes,
            commands::import_text,
            commands::import_subscription,
            commands::remove_node,
            commands::get_settings,
            commands::set_settings,
            commands::select_node,
            commands::connect,
            commands::disconnect,
            commands::core_status,
            commands::tcp_ping,
            commands::tcp_ping_all,
            commands::http_ping,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vanish");
}
