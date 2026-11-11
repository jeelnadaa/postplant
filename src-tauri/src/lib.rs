//! Postplant: open-source Valorant companion. See plan.md for the design.
#![recursion_limit = "256"]
pub mod advisor;
pub mod api;
pub mod career;
pub mod commands;
pub mod db;
pub mod fieldcheck;
pub mod insights;
pub mod live;
pub mod overlay;
pub mod parse;
pub mod players;
pub mod queries;
pub mod recordings;
pub mod state;
pub mod sync;
pub mod transfer;
pub mod updater;

use state::AppState;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};

/// A `portable.flag` file next to the exe keeps all data in `.\data\` (plan 20.1).
pub fn is_portable() -> bool {
    exe_dir()
        .map(|d| d.join("portable.flag").exists())
        .unwrap_or(false)
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(|p| p.to_path_buf())
}

fn data_dir(app: &AppHandle) -> PathBuf {
    if is_portable() {
        if let Some(d) = exe_dir() {
            return d.join("data");
        }
    }
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("data"))
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Postplant", true, None::<&str>)?;
    let ov = MenuItem::with_id(app, "overlay", "Show / hide overlay", true, None::<&str>)?;
    let sync = MenuItem::with_id(app, "sync", "Sync now", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Postplant", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &ov, &sync, &sep, &quit])?;
    let mut b = TrayIconBuilder::with_id("main")
        .tooltip("Postplant")
        .menu(&menu)
        .show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.on_menu_event(|app, e| match e.id().as_ref() {
        "show" => overlay::show_main(app, None),
        "overlay" => overlay::toggle(app),
        "sync" => app.state::<AppState>().wake.notify_one(),
        "quit" => app.exit(0),
        _ => {}
    })
    .on_tray_icon_event(|tray, e| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = e
        {
            overlay::show_main(tray.app_handle(), None);
        }
    })
    .build(app)?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            overlay::show_main(app, None)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, sc, ev| overlay::on_shortcut(app, sc, ev.state()))
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            let dir = data_dir(&handle);
            std::fs::create_dir_all(dir.join("backups"))?;
            let db_path = dir.join("postplant.db");
            // Safety copy before a schema migration (plan 16.1).
            if db_path.exists() {
                if let Ok(c) = rusqlite::Connection::open(&db_path) {
                    let v = db::schema_version(&c);
                    if v > 0 && v < db::SCHEMA_VERSION {
                        let _ = std::fs::copy(
                            &db_path,
                            dir.join("backups")
                                .join(format!("before_migration-v{v}-{}.db", db::now_ms())),
                        );
                    }
                }
            }
            let conn = db::open(&db_path)?;
            app.manage(AppState::new(conn, dir));
            app.manage(live::Live::default());
            let kb = advisor::Kb::load();
            {
                let st = app.state::<AppState>();
                if let Some(me) = st.me() {
                    let c = st.db.lock().unwrap();
                    sync::ensure_starters(&c, &me.puuid, &kb);
                }
            }
            app.manage(kb);
            overlay::create(&handle)?;
            setup_tray(&handle)?;
            overlay::apply_hotkeys(&handle);
            #[cfg(feature = "live")]
            live::start(handle.clone());
            updater::start(handle.clone());
            sync::start_background(handle);
            Ok(())
        })
        .on_window_event(|w, e| {
            if w.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = e {
                    let st = w.app_handle().state::<AppState>();
                    if st.prefs()["close_to_tray"].as_bool().unwrap_or(true) {
                        api.prevent_close();
                        let _ = w.hide();
                    } else {
                        st.overlay_visible.store(false, Ordering::SeqCst);
                        w.app_handle().exit(0);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_state,
            commands::save_prefs,
            commands::set_api_key,
            commands::remove_api_key,
            commands::lookup_account,
            commands::set_me,
            commands::sync_now,
            commands::home,
            commands::matches,
            commands::match_detail,
            commands::analytics,
            commands::habits,
            commands::rank,
            commands::people,
            commands::search_player,
            commands::recent_searches,
            commands::save_note,
            commands::reviews,
            commands::add_review,
            commands::update_review,
            commands::agents_kb,
            commands::set_owned,
            commands::advisor_suggest,
            commands::advisor_learn,
            commands::export_backup,
            commands::export_json,
            commands::inspect_import,
            commands::apply_import,
            commands::list_backups,
            commands::backup_now,
            commands::restore_backup,
            commands::recordings_scan,
            commands::watch,
            commands::overlay_cmd,
            commands::set_autostart,
            commands::get_autostart,
            commands::check_update,
            commands::delete_all_data,
            commands::usage,
            commands::overlay_data,
            commands::live_status,
            commands::my_career,
            commands::player_career,
            commands::player_rank,
            commands::match_ranks,
            commands::test_hotkey,
            commands::stack_people,
            commands::advisor_stack,
            commands::heatmap,
            commands::match_replay,
            commands::trade_stats,
            commands::pattern_notes,
            commands::duo_synergy,
            commands::match_extras,
            commands::field_check,
            commands::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Postplant");
}
