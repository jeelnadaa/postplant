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
        // Never take focus from the game: tao drops its "don't focus" marker after creation, so a
        // focusable overlay would be activated by every later show(), which made the hold-Tab check
        // see VALORANT lose focus and hide/show in a loop. Edit and advisor turn focus back on.
        .focusable(false)
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
    let st = app.state::<AppState>();
    st.overlay_auto.store(false, Ordering::SeqCst);
    let v = st.overlay_visible.load(Ordering::SeqCst);
    set_visible(app, !v);
}

pub fn set_edit(app: &AppHandle, edit: bool) {
    let st = app.state::<AppState>();
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };
    st.overlay_edit.store(edit, Ordering::SeqCst);
    let _ = win.set_ignore_cursor_events(!edit);
    let _ = win.set_focusable(edit || st.overlay_layout.lock().unwrap().as_str() == "advisor");
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

/// Resize and place the overlay window for a layout (plan sections 4, 11, 14.1):
/// normal (panels, saved position), advisor (interactive, right side, agent select),
/// compact (round-1 live panel, left, clear of the minimap), rail (hold Tab, docked right),
/// full (Ctrl+Shift+L table, centred).
pub fn layout(app: &AppHandle, kind: &str) {
    let st = app.state::<AppState>();
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };
    *st.overlay_layout.lock().unwrap() = kind.to_string();
    let mon = win
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| win.primary_monitor().ok().flatten());
    let (w, h, interactive) = match kind {
        "advisor" => (360.0, 600.0, true),
        "compact" => (300.0, 360.0, false),
        "rail" => (440.0, 660.0, false),
        "full" => (1000.0, 640.0, false),
        _ => (W, H, st.overlay_edit.load(Ordering::SeqCst)),
    };
    let _ = win.set_size(tauri::LogicalSize::new(w, h));
    let _ = win.set_ignore_cursor_events(!interactive);
    let _ = win.set_focusable(interactive);
    if kind == "normal" {
        restore_position(app);
    } else if let Some(m) = mon {
        let (p, sz, sc) = (m.position(), m.size(), m.scale_factor());
        let (pw, ph) = ((w * sc) as i32, (h * sc) as i32);
        let (x, y) = match kind {
            "advisor" => (
                p.x + sz.width as i32 - pw - (24.0 * sc) as i32,
                p.y + (sz.height as f64 * 0.14) as i32,
            ),
            "compact" => (
                p.x + (24.0 * sc) as i32,
                p.y + (sz.height as f64 * 0.38) as i32,
            ),
            "rail" => (
                p.x + sz.width as i32 - pw - (16.0 * sc) as i32,
                p.y + (sz.height as i32 - ph) / 2,
            ),
            _ => (
                p.x + (sz.width as i32 - pw) / 2,
                p.y + (sz.height as i32 - ph) / 2,
            ),
        };
        let _ = win.set_position(PhysicalPosition::new(x, y));
    }
    let _ = app.emit_to(LABEL, "overlay-layout", json!({ "kind": kind }));
}

/// Show the overlay for something automatic (post-match board, agent select, live panel). It hides
/// again afterwards if it was hidden before.
pub fn show_auto(app: &AppHandle) {
    let st = app.state::<AppState>();
    if !st.overlay_visible.load(Ordering::SeqCst) {
        st.overlay_auto.store(true, Ordering::SeqCst);
        set_visible(app, true);
    }
}

pub fn auto_done(app: &AppHandle) {
    let st = app.state::<AppState>();
    // Hide before switching back, or the normal panel flashes at its old spot for a frame.
    if st.overlay_auto.swap(false, Ordering::SeqCst) {
        set_visible(app, false);
    }
    layout(app, "normal");
}

pub fn open_advisor(app: &AppHandle, map: Option<&str>, auto: bool) {
    layout(app, "advisor");
    let _ = app.emit_to(LABEL, "advisor-open", json!({ "map": map, "auto": auto }));
    if auto {
        show_auto(app);
    } else {
        set_visible(app, true);
    }
}

pub fn close_advisor(app: &AppHandle, _auto: bool) {
    let _ = app.emit_to(LABEL, "advisor-close", ());
    if app
        .state::<AppState>()
        .overlay_layout
        .lock()
        .unwrap()
        .as_str()
        == "advisor"
    {
        auto_done(app);
    }
}

pub fn toggle_advisor(app: &AppHandle) {
    let open = app
        .state::<AppState>()
        .overlay_layout
        .lock()
        .unwrap()
        .as_str()
        == "advisor";
    if open {
        close_advisor(app, false);
    } else {
        let map = app.state::<crate::live::Live>().map.lock().unwrap().clone();
        open_advisor(app, if map.is_empty() { None } else { Some(&map) }, false);
    }
}

/// Live round-1 panel: shown for 25 s when the match starts (the overlay page runs the timer).
pub fn show_live_compact(app: &AppHandle) {
    layout(app, "compact");
    let _ = app.emit_to(
        LABEL,
        "live-show",
        json!({ "mode": "compact", "seconds": 25 }),
    );
    show_auto(app);
}

pub fn live_tab(app: &AppHandle, down: bool) {
    let live = app.state::<crate::live::Live>();
    if live.table_open.load(Ordering::SeqCst) {
        return;
    }
    if down {
        // Size, place and switch the page first, then show, so the hidden window never appears
        // with its previous layout.
        layout(app, "rail");
        let _ = app.emit_to(LABEL, "live-show", json!({ "mode": "rail" }));
        show_auto(app);
    } else {
        let _ = app.emit_to(LABEL, "live-show", json!({ "mode": "none" }));
        auto_done(app);
    }
}

pub fn toggle_live_table(app: &AppHandle) {
    let live = app.state::<crate::live::Live>();
    if live.roster.lock().unwrap().is_null() {
        let _ = app.emit("toast", json!({ "kind": "info", "title": "No live match", "text": "The live table shows once a match is in progress with Live match turned on." }));
        return;
    }
    let open = !live.table_open.load(Ordering::SeqCst);
    live.table_open.store(open, Ordering::SeqCst);
    if open {
        layout(app, "full");
        let _ = app.emit_to(LABEL, "live-show", json!({ "mode": "full" }));
        show_auto(app);
    } else {
        let _ = app.emit_to(LABEL, "live-show", json!({ "mode": "none" }));
        auto_done(app);
    }
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
        "advisor" => toggle_advisor(app),
        "live_table" => toggle_live_table(app),
        "post_done" => auto_done(app),
        "hotkeys_pause" => {
            app.state::<AppState>()
                .hotkeys_paused
                .store(true, Ordering::SeqCst);
            apply_hotkeys(app);
        }
        "hotkeys_resume" => {
            app.state::<AppState>()
                .hotkeys_paused
                .store(false, Ordering::SeqCst);
            apply_hotkeys(app);
        }
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
    if st.hotkeys_paused.load(Ordering::SeqCst) {
        *st.hotkeys.lock().unwrap() = vec![];
        return failed;
    }
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

/// Can this combination be registered right now? Used by the hotkey capture dialog (hotkeys are
/// paused while it is open, so a failure means another app owns the combination).
pub fn test_combo(app: &AppHandle, combo: &str) -> Result<(), String> {
    let sc = Shortcut::from_str(combo)
        .map_err(|_| format!("{combo} isn't a key combination Windows accepts."))?;
    let gs = app.global_shortcut();
    if gs.is_registered(sc) {
        return Ok(());
    }
    gs.register(sc)
        .map_err(|_| format!("{combo} is already used by another app."))?;
    let _ = gs.unregister(sc);
    Ok(())
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
