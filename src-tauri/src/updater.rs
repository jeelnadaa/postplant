//! Auto-updates (plan 20.3). Signed updates through tauri-plugin-updater: `latest.json` on the
//! newest GitHub release, every download checked against the public key built into the app.
//! Checks at start and every 6 hours, never while VALORANT runs, and never installs mid-match.
//! If no signed update is published (or this is the portable build), it falls back to telling you
//! a new version exists with a link to the release page.
use crate::state::AppState;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

const RELEASES_API: &str = "https://api.github.com/repos/jeelnadaa/postplant/releases/latest";

fn ver(v: &str) -> (u64, u64, u64) {
    let mut it = v
        .trim_start_matches('v')
        .split(['.', '-'])
        .map(|x| x.parse::<u64>().unwrap_or(0));
    (
        it.next().unwrap_or(0),
        it.next().unwrap_or(0),
        it.next().unwrap_or(0),
    )
}

async fn github_latest(cur: &str) -> Result<Value, String> {
    let r = reqwest::Client::new()
        .get(RELEASES_API)
        .header(
            "User-Agent",
            concat!("Postplant/", env!("CARGO_PKG_VERSION")),
        )
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if r.status().as_u16() == 404 {
        return Ok(json!({ "current": cur, "newer": false }));
    }
    let v: Value = r.json().await.map_err(|e| e.to_string())?;
    let tag = v["tag_name"].as_str().unwrap_or("").to_string();
    Ok(
        json!({ "current": cur, "latest": tag, "newer": ver(&tag) > ver(cur), "url": v["html_url"], "notes": v["body"], "published_at": v["published_at"], "signed": false }),
    )
}

/// { current, latest, newer, signed (one-click install possible), url, notes }
pub async fn check(app: &AppHandle) -> Result<Value, String> {
    let st = app.state::<AppState>();
    if st.game_running.load(Ordering::SeqCst) {
        return Ok(json!({ "skipped": "game running" }));
    }
    let cur = app.package_info().version.to_string();
    if !crate::is_portable() {
        if let Ok(u) = app.updater() {
            match u.check().await {
                Ok(Some(up)) => {
                    return Ok(
                        json!({ "current": cur, "latest": up.version, "newer": true, "signed": true, "notes": up.body,
                        "url": format!("https://github.com/jeelnadaa/postplant/releases/tag/v{}", up.version) }),
                    )
                }
                Ok(None) => return Ok(json!({ "current": cur, "newer": false })),
                Err(_) => {} // no signed manifest on this release: fall back to the releases API
            }
        }
    }
    github_latest(&cur).await
}

/// Download, verify the signature, install and restart. Refuses while the game runs.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    if app.state::<AppState>().game_running.load(Ordering::SeqCst) {
        return Err("VALORANT is running. The update will wait until you close the game.".into());
    }
    if crate::is_portable() {
        return Err(
            "The portable build updates by downloading the new zip from the release page.".into(),
        );
    }
    let up = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Already up to date.")?;
    let app2 = app.clone();
    let mut got: usize = 0;
    up.download_and_install(
        move |chunk, total| {
            got += chunk;
            let _ = app2.emit("update-progress", json!({ "got": got, "total": total }));
        },
        || {},
    )
    .await
    .map_err(|e| e.to_string())?;
    app.restart();
}

/// Background check: 20 s after start, then every 6 hours; while the game runs it waits.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(20)).await;
        let mut told = String::new();
        loop {
            let st = app.state::<AppState>();
            let enabled = st.prefs()["update_check"].as_bool().unwrap_or(true);
            if enabled && !st.game_running.load(Ordering::SeqCst) {
                if let Ok(v) = check(&app).await {
                    let latest = v["latest"].as_str().unwrap_or("").to_string();
                    if v["newer"] == true && latest != told {
                        let _ = app.emit("update-available", v);
                        told = latest;
                    }
                }
                tokio::time::sleep(Duration::from_secs(6 * 3600)).await;
            } else {
                tokio::time::sleep(Duration::from_secs(600)).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions_compare() {
        assert!(super::ver("v0.10.0") > super::ver("0.9.9"));
        assert!(super::ver("1.0.0") > super::ver("v0.99.1"));
        assert_eq!(super::ver("v0.2.0-beta"), (0, 2, 0));
    }
}
