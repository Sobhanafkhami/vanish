use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::engine;

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Open Vanish", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Connect / Disconnect", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &toggle, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().expect("window icon").clone())
        .tooltip("Vanish")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "toggle" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = if engine::status(&app).running { engine::stop(&app).await } else { engine::start(&app).await };
                });
            }
            "quit" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = engine::stop(&app).await; // kills sing-box and restores the system proxy
                    app.exit(0);
                });
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
