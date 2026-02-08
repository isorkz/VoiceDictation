use crate::toggle_recording_impl;
use image::GenericImageView;
use tauri::menu::{Menu, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItemBuilder::with_id("toggle", "Start/Stop").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = Menu::with_items(app, &[&toggle, &settings, &quit])?;

    let img = image::load_from_memory(include_bytes!("../icons/32x32.png"))
        .map_err(|e| tauri::Error::AssetNotFound(format!("failed to decode tray icon: {e}")))?;
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8().into_raw();
    let icon = tauri::image::Image::new_owned(rgba, w, h);

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = toggle_recording_impl(app).await;
                });
            }
            "settings" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}
