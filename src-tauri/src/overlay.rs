//! Overlay window (plan section 4) and global hotkeys.
use crate::db;
use crate::state::AppState;
use serde_json::json;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const LABEL: &str = "overlay";
const W: f64 = 300.0;
const H: f64 = 460.0;

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(LABEL).is_some() {
        return Ok(());
    }
    let win = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("overlay.html".into()))
        .title("Postplant overlay")
        .inner_size(W, H)
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build()?;
    let _ = win.set_ignore_cursor_events(true);
    restore_position(app);
    Ok(())
}

fn default_position(app: &AppHandle) -> Option<PhysicalPosition<i32>> {
    let win = app.get_webview_window(LABEL)?;
    let mon = win
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| win.primary_monitor().ok().flatten())?;
    let s = mon.size();
    let p = mon.position();
    let scale = mon.scale_factor();
    // Right edge, a third down: clear of the minimap (top-left), kill feed (top-right) and crosshair.
    Some(PhysicalPosition::new(
        p.x + s.width as i32 - ((W + 24.0) * scale) as i32,
        p.y + (s.height as f64 * 0.32) as i32,
    ))
}

pub fn restore_position(app: &AppHandle) {
    let st = app.state::<AppState>();
    let prefs = st.prefs();
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };
    let saved = (
        prefs["overlay"]["x"].as_i64(),
        prefs["overlay"]["y"].as_i64(),
    );
    let on_screen = |x: i32, y: i32| {
        win.available_monitors()
            .map(|ms| {
                ms.iter().any(|m| {
                    let (p, s) = (m.position(), m.size());
                    x >= p.x
                        && y >= p.y
                        && x < p.x + s.width as i32 - 40
                        && y < p.y + s.height as i32 - 40
                })
            })
            .unwrap_or(false)
    };
    let pos = match saved {
        (Some(x), Some(y)) if on_screen(x as i32, y as i32) => {
            Some(PhysicalPosition::new(x as i32, y as i32))
        }
        _ => default_position(app),
    };
    if let Some(p) = pos {
        let _ = win.set_position(p);
    }
}

fn save_prefs_patch(app: &AppHandle, patch: serde_json::Value) {
    let st = app.state::<AppState>();
    let merged = crate::state::merge(st.prefs(), patch);
    let _ = db::set_setting(&st.db.lock().unwrap(), "prefs", &merged.to_string());
}

pub fn set_visible(app: &AppHandle, show: bool) {
    let st = app.state::<AppState>();
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };
    if show {
        let _ = win.show();
        let _ = win.set_always_on_top(true);
    } else {
        let _ = win.hide();
    }
    st.overlay_visible.store(show, Ordering::SeqCst);
    let _ = app.emit("overlay-visible", json!({ "visible": show }));
    if show {
        st.wake.notify_one();
    }
}

pub fn toggle(app: &AppHandle) {
    let v = app
        .state::<AppState>()
        .overlay_visible
        .load(Ordering::SeqCst);
    set_visible(app, !v);
}

pub fn set_edit(app: &AppHandle, edit: bool) {
    let st = app.state::<AppState>();
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };
    st.overlay_edit.store(edit, Ordering::SeqCst);
    let _ = win.set_ignore_cursor_events(!edit);
    if edit {
        set_visible(app, true);
        let _ = win.set_focus();
    } else if let Ok(p) = win.outer_position() {
        save_prefs_patch(app, json!({ "overlay": { "x": p.x, "y": p.y } }));
    }
    let _ = app.emit_to(LABEL, "overlay-edit", json!({ "edit": edit }));
}

pub fn reset(app: &AppHandle) {
    save_prefs_patch(app, json!({ "overlay": { "x": null, "y": null } }));
    restore_position(app);
}

pub fn show_main(app: &AppHandle, route: Option<&str>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        if let Some(r) = route {
            let _ = app.emit_to("main", "navigate", r);
        }
    }
}

pub fn run_action(app: &AppHandle, action: &str) {
    match action {
        "toggle_overlay" => toggle(app),
        "edit_overlay" => {
            let e = app.state::<AppState>().overlay_edit.load(Ordering::SeqCst);
            set_edit(app, !e);
        }
        "reset_overlay" => reset(app),
        "cycle_panel" => {
            let _ = app.emit_to(LABEL, "overlay-cycle", ());
            set_visible(app, true);
        }
        "refresh" => app.state::<AppState>().wake.notify_one(),
        "open_app" => show_main(app, None),
        "advisor" => show_main(app, Some("advisor")),
        _ => {}
    }
}

/// (Re)register hotkeys. With "hotkeys only while the game runs" on, only "open app" stays registered
/// when VALORANT is closed, so other apps keep their shortcuts (plan section 4).
pub fn apply_hotkeys(app: &AppHandle) -> Vec<String> {
    let st = app.state::<AppState>();
    let prefs = st.prefs();
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut failed = vec![];
    let mut map = vec![];
    let game = st.game_running.load(Ordering::SeqCst);
    let game_only = prefs["hotkeys_game_only"].as_bool().unwrap_or(true);
    if let Some(h) = prefs["hotkeys"].as_object() {
        for (action, combo) in h {
            let Some(combo) = combo.as_str().filter(|c| !c.is_empty()) else {
                continue;
            };
            if game_only && !game && action != "open_app" {
                continue;
            }
            match Shortcut::from_str(combo) {
                Ok(sc) => {
                    if gs.register(sc).is_ok() {
                        map.push((sc.id(), action.clone()));
                    } else {
                        failed.push(action.clone());
                    }
                }
                Err(_) => failed.push(action.clone()),
            }
        }
    }
    *st.hotkeys.lock().unwrap() = map;
    let _ = app.emit(
        "hotkeys",
        json!({ "failed": failed, "active": !game_only || game }),
    );
    failed
}

pub fn on_shortcut(app: &AppHandle, sc: &Shortcut, state: ShortcutState) {
    if state != ShortcutState::Pressed {
        return;
    }
    let action = app
        .state::<AppState>()
        .hotkeys
        .lock()
        .unwrap()
        .iter()
        .find(|(id, _)| *id == sc.id())
        .map(|(_, a)| a.clone());
    if let Some(a) = action {
        run_action(app, &a);
    }
}
