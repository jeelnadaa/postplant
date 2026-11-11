//! Shared app state and small helpers used by commands.
use crate::api::Henrik;
use crate::db;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64};
use std::sync::Mutex;

pub const KEYRING_SERVICE: &str = "Postplant";
pub const KEYRING_USER: &str = "henrikdev-api-key";

pub struct AppState {
    pub db: Mutex<Connection>,
    pub api: Henrik,
    pub data_dir: PathBuf,
    pub game_running: AtomicBool,
    pub overlay_visible: AtomicBool,
    pub overlay_edit: AtomicBool,
    pub syncing: AtomicBool,
    pub last_sync: AtomicI64,
    pub last_error: Mutex<Option<String>>,
    pub wake: tokio::sync::Notify,
    pub hotkeys: Mutex<Vec<(u32, String)>>,
    /// While the hotkey capture dialog is open, global hotkeys are released.
    pub hotkeys_paused: AtomicBool,
    /// Poll for a just-finished match every 45 s until this time (ms).
    pub fast_poll_until: AtomicI64,
    /// The overlay was shown automatically (post-match board / agent select) and should hide again.
    pub overlay_auto: AtomicBool,
    /// Current overlay layout: normal | advisor | compact | rail | full.
    pub overlay_layout: Mutex<String>,
    /// Your career stats, cached by (match count, newest match, queue).
    pub career_cache: Mutex<Option<(String, Value)>>,
}

impl AppState {
    pub fn new(conn: Connection, data_dir: PathBuf) -> Self {
        let key = load_key();
        let cap = db::get_json(&conn, "prefs")
            .and_then(|p| p.get("daily_cap").and_then(|v| v.as_u64()))
            .unwrap_or(1500) as u32;
        let api = Henrik::new(key);
        api.gov.lock().unwrap().daily_cap = cap;
        let last = db::get_setting(&conn, "last_sync")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        Self {
            db: Mutex::new(conn),
            api,
            data_dir,
            game_running: AtomicBool::new(false),
            overlay_visible: AtomicBool::new(false),
            overlay_edit: AtomicBool::new(false),
            syncing: AtomicBool::new(false),
            last_sync: AtomicI64::new(last),
            last_error: Mutex::new(None),
            wake: tokio::sync::Notify::new(),
            hotkeys: Mutex::new(Vec::new()),
            hotkeys_paused: AtomicBool::new(false),
            fast_poll_until: AtomicI64::new(0),
            overlay_auto: AtomicBool::new(false),
            overlay_layout: Mutex::new("normal".into()),
            career_cache: Mutex::new(None),
        }
    }

    pub fn me(&self) -> Option<Me> {
        let c = self.db.lock().unwrap();
        db::get_json(&c, "me").and_then(|v| serde_json::from_value(v).ok())
    }

    pub fn prefs(&self) -> Value {
        let c = self.db.lock().unwrap();
        merge(
            default_prefs(),
            db::get_json(&c, "prefs").unwrap_or(Value::Null),
        )
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Me {
    pub puuid: String,
    pub name: String,
    pub tag: String,
    pub region: String,
    #[serde(default)]
    pub account_level: i64,
}

pub fn default_prefs() -> Value {
    json!({
        "theme": "ink", "lang": "en", "font": "satoshi", "cvd": false, "reduce_motion": "system", "scale": 100,
        "tilt_after": 3, "daily_cap": 1500,
        "overlay": { "enabled": true, "panel": 0, "opacity": 88, "scale": 100, "x": null, "y": null, "auto_post": true, "post_seconds": 20 },
        "hotkeys_game_only": true,
        "hotkeys": {
            "toggle_overlay": "Ctrl+Shift+H", "edit_overlay": "Ctrl+Shift+M", "reset_overlay": "Ctrl+Shift+Home",
            "cycle_panel": "Ctrl+Shift+PageDown", "refresh": "Ctrl+Shift+F5", "open_app": "Ctrl+Shift+O", "advisor": "Ctrl+Shift+A",
            "live_table": "Ctrl+Shift+L"
        },
        "recordings_dir": null,
        "auto_backup": true, "backups_keep": 4,
        "update_check": true,
        "close_to_tray": true,
        "live_enabled": false, "live_ack": null, "live_tab": true, "live_form": true, "advisor_in_select": true,
        "stack": []
    })
}

/// Deep-merge `over` onto `base` (objects only; other values replace).
pub fn merge(mut base: Value, over: Value) -> Value {
    if let (Some(b), Value::Object(o)) = (base.as_object_mut(), over.clone()) {
        for (k, v) in o {
            let cur = b.remove(&k).unwrap_or(Value::Null);
            b.insert(
                k,
                if cur.is_object() && v.is_object() {
                    merge(cur, v)
                } else {
                    v
                },
            );
        }
        return base;
    }
    if over.is_null() {
        base
    } else {
        over
    }
}

pub fn load_key() -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .ok()?
        .get_password()
        .ok()
        .filter(|k| !k.trim().is_empty())
}

pub fn save_key(key: &str) -> Result<(), String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .and_then(|e| e.set_password(key))
        .map_err(|e| format!("Could not save the key in Windows Credential Manager: {e}"))
}

pub fn delete_key() {
    if let Ok(e) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = e.delete_credential();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merge_keeps_defaults() {
        let m = merge(
            default_prefs(),
            json!({"theme": "paper", "overlay": {"opacity": 95}}),
        );
        assert_eq!(m["theme"], "paper");
        assert_eq!(m["overlay"]["opacity"], 95);
        assert_eq!(m["overlay"]["panel"], 0);
    }
}
