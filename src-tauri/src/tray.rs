use crate::toggle_recording_impl;
use crate::app_state::Status;
use resvg::{tiny_skia, usvg};
use std::sync::OnceLock;
use tauri::menu::{Menu, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

const TRAY_ID: &str = "main";
const ICON_SIZE: u32 = 32;
const TOGGLE_MENU_ID: &str = "toggle";

// Lucide icons (MIT License) - https://lucide.dev/
const MIC_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 13a2 2 0 0 0 2-2V7a2 2 0 0 1 4 0v13a2 2 0 0 0 4 0V4a2 2 0 0 1 4 0v13a2 2 0 0 0 4 0v-4a2 2 0 0 1 2-2" /></svg>"#;
const DISC_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10" /><circle cx="12" cy="12" r="2" /></svg>"#;
const LOADER_CIRCLE_SVG: &str =
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56" /></svg>"#;
const CIRCLE_ALERT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10" /><line x1="12" x2="12" y1="8" y2="12" /><line x1="12" x2="12.01" y1="16" y2="16" /></svg>"#;

#[cfg(target_os = "macos")]
const TRAY_STROKE: &str = "#000000";

#[cfg(not(target_os = "macos"))]
const TRAY_STROKE: &str = "#ffffff";

#[derive(Clone)]
struct TrayIcons {
    idle: tauri::image::Image<'static>,
    recording: tauri::image::Image<'static>,
    busy: tauri::image::Image<'static>,
    error: tauri::image::Image<'static>,
}

static ICONS: OnceLock<Result<TrayIcons, String>> = OnceLock::new();
static TOGGLE_ITEM: OnceLock<tauri::menu::MenuItem<tauri::Wry>> = OnceLock::new();

fn lucide_svg_with_stroke(svg: &str, stroke: &str) -> String {
    svg.replace(r#"stroke="currentColor""#, &format!(r#"stroke="{stroke}""#))
}

fn render_svg_icon(svg: &str) -> tauri::Result<tauri::image::Image<'static>> {
    let svg = lucide_svg_with_stroke(svg, TRAY_STROKE);
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg.as_bytes(), &opt)
        .map_err(|e| tauri::Error::AssetNotFound(format!("failed to parse tray svg: {e}")))?;

    let mut pixmap = tiny_skia::Pixmap::new(ICON_SIZE, ICON_SIZE)
        .ok_or_else(|| tauri::Error::AssetNotFound("failed to allocate tray pixmap".to_string()))?;

    let w = tree.size().width();
    let h = tree.size().height();
    if w <= 0.0 || h <= 0.0 {
        return Err(tauri::Error::AssetNotFound(
            "tray svg has invalid size".to_string(),
        ));
    }

    let scale_x = ICON_SIZE as f32 / w;
    let scale_y = ICON_SIZE as f32 / h;
    let scale = scale_x.min(scale_y);

    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    Ok(tauri::image::Image::new_owned(
        pixmap.data().to_vec(),
        ICON_SIZE,
        ICON_SIZE,
    ))
}

fn icons() -> tauri::Result<&'static TrayIcons> {
    let result = ICONS.get_or_init(|| {
        Ok::<_, String>(TrayIcons {
            idle: render_svg_icon(MIC_SVG).map_err(|e| e.to_string())?,
            recording: render_svg_icon(DISC_SVG).map_err(|e| e.to_string())?,
            busy: render_svg_icon(LOADER_CIRCLE_SVG).map_err(|e| e.to_string())?,
            error: render_svg_icon(CIRCLE_ALERT_SVG).map_err(|e| e.to_string())?,
        })
    });

    match result {
        Ok(icons) => Ok(icons),
        Err(e) => Err(tauri::Error::AssetNotFound(e.clone())),
    }
}

fn toggle_menu_state(status: &Status) -> (&'static str, bool) {
    if status.state == "Recording" {
        return ("Stop", true);
    }

    let busy = matches!(status.state.as_str(), "Transcribing" | "Inserting");
    ("Start", !busy)
}

pub fn update_for_status(app: &AppHandle, status: &Status) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };

    let icons = icons()?;
    let (icon, tooltip_state) = if status.last_error.is_some() {
        (&icons.error, "Error")
    } else {
        match status.state.as_str() {
            "Recording" => (&icons.recording, "Recording"),
            "Transcribing" | "Inserting" => (&icons.busy, status.state.as_str()),
            _ => (&icons.idle, "Idle"),
        }
    };

    tray.set_icon(Some(icon.clone()))?;
    let _ = tray.set_tooltip(Some(format!("VoiceDictation ({tooltip_state})")));

    if let Some(item) = TOGGLE_ITEM.get() {
        let (text, enabled) = toggle_menu_state(status);
        let _ = item.set_text(text);
        let _ = item.set_enabled(enabled);
    }
    Ok(())
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItemBuilder::with_id(TOGGLE_MENU_ID, "Start").build(app)?;
    let _ = TOGGLE_ITEM.set(toggle.clone());
    let settings = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = Menu::with_items(app, &[&toggle, &settings, &quit])?;

    let icon = icons()?.idle.clone();

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_tray_icons() {
        let mic = render_svg_icon(MIC_SVG).expect("mic icon");
        assert_eq!(mic.width(), ICON_SIZE);
        assert_eq!(mic.height(), ICON_SIZE);
        assert_eq!(mic.rgba().len(), (ICON_SIZE * ICON_SIZE * 4) as usize);

        let disc = render_svg_icon(DISC_SVG).expect("disc icon");
        assert_eq!(disc.width(), ICON_SIZE);
        assert_eq!(disc.height(), ICON_SIZE);
        assert_eq!(disc.rgba().len(), (ICON_SIZE * ICON_SIZE * 4) as usize);

        let loader = render_svg_icon(LOADER_CIRCLE_SVG).expect("loader icon");
        assert_eq!(loader.width(), ICON_SIZE);
        assert_eq!(loader.height(), ICON_SIZE);
        assert_eq!(loader.rgba().len(), (ICON_SIZE * ICON_SIZE * 4) as usize);

        let alert = render_svg_icon(CIRCLE_ALERT_SVG).expect("alert icon");
        assert_eq!(alert.width(), ICON_SIZE);
        assert_eq!(alert.height(), ICON_SIZE);
        assert_eq!(alert.rgba().len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
    }

    #[test]
    fn toggle_menu_state_maps_status() {
        let mut status = Status {
            state: "Idle".to_string(),
            last_error: None,
        };
        assert_eq!(toggle_menu_state(&status), ("Start", true));

        status.state = "Recording".to_string();
        assert_eq!(toggle_menu_state(&status), ("Stop", true));

        status.state = "Transcribing".to_string();
        assert_eq!(toggle_menu_state(&status), ("Start", false));

        status.state = "Inserting".to_string();
        assert_eq!(toggle_menu_state(&status), ("Start", false));
    }
}
