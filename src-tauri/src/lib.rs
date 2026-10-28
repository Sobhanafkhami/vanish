mod autostart;
mod commands;
mod config;
mod engine;
mod netcheck;
mod nodes;
mod probe;
mod proxy;
mod state;
mod subs;
mod tray;
mod updater;
mod usage;

use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    // A second launch must not spawn a second process: two instances would fight over
    // the data files and the sing-box ports. Focus the running one instead.
    // Dev builds are exempt (and use their own data folder, see setup) so `tauri dev` can run
    // next to an installed Vanish.
    #[cfg(not(debug_assertions))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main(app)));
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            #[allow(unused_mut)]
            let mut dir = app.path().app_data_dir()?;
            // Separate nodes/settings so the dev copy cannot overwrite the installed app's files.
            #[cfg(debug_assertions)]
            {
                dir.set_file_name(format!("{}.dev", dir.file_name().and_then(|n| n.to_str()).unwrap_or("app.vanish.client")));
            }
            std::fs::create_dir_all(dir.join("bin"))?;
            app.manage(state::AppState::load(dir));
            tray::init(app.handle())?;

            // Launched by the OS at login: stay in the tray.
            if std::env::args().any(|a| a == "--minimized") {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            let handle = app.handle().clone();
            {
                use tauri::Listener;
                let h = handle.clone();
                app.listen("restart-core", move |_| {
                    let h = h.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = engine::stop(&h).await;
                        if let Err(e) = engine::start(&h).await {
                            eprintln!("restart failed: {e}");
                        }
                    });
                });
            }
            tauri::async_runtime::spawn(subs::refresh_loop(handle.clone()));
            tauri::async_runtime::spawn(updater::check_loop(handle.clone()));
            let connect = {
                let st = handle.state::<state::AppState>();
                let s = st.settings.lock().unwrap();
                s.connect_on_launch && s.selected_node.is_some()
            };
            if connect {
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = engine::start(&handle).await {
                        eprintln!("connect on launch failed: {e}");
                    }
                });
            }
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
            commands::list_subscriptions,
            commands::add_subscription,
            commands::refresh_subscription,
            commands::rename_subscription,
            commands::remove_subscription,
            commands::pick_fastest,
            commands::exit_info,
            commands::real_delay,
            commands::speed_test,
            commands::diagnostics,
            commands::reset_settings,
            commands::list_running_apps,
            commands::pick_app_exe,
            commands::app_version,
            commands::check_update,
            commands::install_update,
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

