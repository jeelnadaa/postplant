//! Syncing my matches and the adaptive polling loop (plan section 13).
use crate::api::{paths, Prio};
use crate::db::{self, now_ms};
use crate::parse::{self, arr, i, s};
use crate::state::AppState;
use rusqlite::params;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub const GAME_EXE: &str = "VALORANT-Win64-Shipping.exe";
const STARTERS: &[&str] = &["Brimstone", "Jett", "Phoenix", "Sage", "Sova"];

/// Fetch my newest match ids, store the ones we don't have, refresh rank. Returns {new, checked}.
pub async fn sync_me(app: &AppHandle, prio: Prio, size: u32) -> Result<Value, String> {
    let st = app.state::<AppState>();
    let me = st.me().ok_or("Set your Riot ID first.")?;
    if st.syncing.swap(true, Ordering::SeqCst) {
        return Ok(json!({ "new": 0, "busy": true }));
    }
    let _ = app.emit("sync-state", json!({ "syncing": true }));
    let r = sync_inner(app, &st, &me, prio, size).await;
    st.syncing.store(false, Ordering::SeqCst);
    match &r {
        Ok(v) => {
            let t = now_ms();
            st.last_sync.store(t, Ordering::SeqCst);
            *st.last_error.lock().unwrap() = None;
            let _ = db::set_setting(&st.db.lock().unwrap(), "last_sync", &t.to_string());
            let _ = app.emit(
                "sync-state",
                json!({ "syncing": false, "at": t, "new": v["new"] }),
            );
            if v["new"].as_i64().unwrap_or(0) > 0 {
                st.fast_poll_until.store(0, Ordering::SeqCst);
                // Post-match scoreboard (plan section 4, panel 4): make sure the overlay is on screen.
                let p = st.prefs();
                if p["overlay"]["enabled"].as_bool().unwrap_or(true)
                    && p["overlay"]["auto_post"].as_bool().unwrap_or(true)
                {
                    crate::overlay::show_auto(app);
                    crate::overlay::layout(app, "normal");
                }
                let _ = app.emit("new-match", v.clone());
            }
        }
        Err(e) => {
            *st.last_error.lock().unwrap() = Some(e.clone());
            let _ = app.emit("sync-state", json!({ "syncing": false, "error": e }));
        }
    }
    r
}

async fn sync_inner(
    app: &AppHandle,
    st: &AppState,
    me: &crate::state::Me,
    prio: Prio,
    size: u32,
) -> Result<Value, String> {
    let have = |id: &str| {
        st.db
            .lock()
            .unwrap()
            .query_row("SELECT 1 FROM matches WHERE match_id=?1", [id], |_| Ok(()))
            .is_ok()
    };
    let mut stored = Vec::new();
    // 1. The match the live module saw end: fetch it by id (1 request, no list needed).
    let pending = st.pending_match.lock().unwrap().clone();
    if !pending.is_empty() {
        if have(&pending) {
            st.pending_match.lock().unwrap().clear();
        } else if let Ok(raw) = st
            .api
            .get(&paths::match_v4(&me.region, &pending), prio)
            .await
        {
            let mut c = st.db.lock().unwrap();
            if let Ok(id) = parse::store_match(&mut c, &raw) {
                stored.push(id);
                st.pending_match.lock().unwrap().clear();
            }
        }
    }
    // 2. Fresh match history (full matches in one request). stored-matches lags behind by minutes
    //    to hours, so it is only the fallback and the path for bigger backfills.
    let mut ids: Vec<String> = Vec::new();
    let mut fresh_ok = false;
    if size <= 10 {
        if let Ok(list) = st
            .api
            .get(&paths::matches_v4(&me.region, &me.puuid, size), prio)
            .await
        {
            fresh_ok = true;
            for m in arr(&list, "/data") {
                let id = s(m, "/metadata/match_id").to_string();
                if id.is_empty() {
                    continue;
                }
                ids.push(id.clone());
                if !have(&id) && !stored.contains(&id) {
                    let mut c = st.db.lock().unwrap();
                    if let Ok(id) = parse::store_match(&mut c, m) {
                        stored.push(id);
                    }
                }
            }
        }
    }
    let missing: Vec<String> = if fresh_ok {
        Vec::new()
    } else {
        let list = match st
            .api
            .get(
                &paths::stored_matches(&me.region, &me.puuid, size, None),
                prio,
            )
            .await
        {
            Ok(l) => l,
            Err(e) if stored.is_empty() => return Err(e.to_string()),
            Err(_) => Value::Null,
        };
        ids = arr(&list, "/data")
            .iter()
            .map(|m| s(m, "/meta/id").to_string())
            .filter(|x| !x.is_empty())
            .collect();
        ids.iter()
            .filter(|id| !have(id) && !stored.contains(id))
            .cloned()
            .collect()
    };
    for id in &missing {
        let raw = match st.api.get(&paths::match_v4(&me.region, id), prio).await {
            Ok(v) => v,
            Err(e) => {
                if stored.is_empty() {
                    return Err(e.to_string());
                }
                break; // keep what we have; the rest comes next sync
            }
        };
        let mut c = st.db.lock().unwrap();
        if let Ok(id) = parse::store_match(&mut c, &raw) {
            stored.push(id);
        }
    }
    let last_snap: i64 = {
        let c = st.db.lock().unwrap();
        c.query_row(
            "SELECT COALESCE(MAX(taken_at),0) FROM mmr_snapshots WHERE puuid=?1",
            [&me.puuid],
            |r| r.get(0),
        )
        .unwrap_or(0)
    };
    if !stored.is_empty() || now_ms() - last_snap > 30 * 60_000 {
        if let Ok(mmr) = st.api.get(&paths::mmr(&me.region, &me.puuid), prio).await {
            let c = st.db.lock().unwrap();
            store_mmr(&c, &me.puuid, &mmr);
        }
    }
    if !stored.is_empty() {
        if let Ok(h) = st
            .api
            .get(&paths::mmr_history(&me.region, &me.puuid), prio)
            .await
        {
            let c = st.db.lock().unwrap();
            for row in arr(&h, "/data") {
                let mid = s(row, "/match_id");
                if !mid.is_empty() {
                    let _ = c.execute(
                        "INSERT OR REPLACE INTO rr_changes(match_id, puuid, rr_change, tier_after, rr_after) VALUES (?1,?2,?3,?4,?5)",
                        params![mid, me.puuid, i(row, "/mmr_change_to_last_game"), i(row, "/currenttier"), i(row, "/ranking_in_tier")],
                    );
                }
            }
        }
        let c = st.db.lock().unwrap();
        update_owned_agents(&c, &me.puuid);
        let _ = crate::recordings::rematch(&c);
    }
    let _ = app; // emits happen in the caller
    Ok(json!({ "new": stored.len(), "checked": ids.len(), "ids": stored }))
}

pub fn store_mmr(c: &rusqlite::Connection, puuid: &str, mmr: &Value) {
    let d = parse::unwrap_data(mmr);
    let _ = c.execute(
        "INSERT OR REPLACE INTO mmr_snapshots(puuid, taken_at, season_id, tier, rr, peak_tier, elo) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![puuid, now_ms(), s(d, "/seasonal/0/season/id"), i(d, "/current/tier/id"), i(d, "/current/rr"), i(d, "/peak/tier/id"), i(d, "/current/elo")],
    );
    let _ = db::cache_put(c, &format!("mmr:{puuid}"), d);
}

/// Agents seen in my matches are owned (plan 14.1a). Never removes manual choices.
pub fn update_owned_agents(c: &rusqlite::Connection, puuid: &str) {
    let _ = c.execute(
        "INSERT OR IGNORE INTO agent_pool(puuid, agent_id, comfort, manual)
         SELECT DISTINCT ?1, agent_id, 2, 0 FROM match_players WHERE puuid=?1 AND agent_id<>''",
        [puuid],
    );
}

pub fn ensure_starters(c: &rusqlite::Connection, puuid: &str, kb: &crate::advisor::Kb) {
    for a in kb
        .agents
        .iter()
        .filter(|a| STARTERS.contains(&a.name.as_str()))
    {
        let _ = c.execute(
            "INSERT OR IGNORE INTO agent_pool(puuid, agent_id, comfort, manual) VALUES (?1,?2,2,0)",
            params![puuid, a.agent_id],
        );
    }
}

/// Background loop: game detection every 10 s; a check every 75 s while the game runs, every 30 s for
/// 10 min after it closes, every 20 s by match id right after the live module sees a match end,
/// backing off to 12 min otherwise.
pub fn start_background(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut sys = sysinfo::System::new();
        let mut prev_running = false;
        let mut interval = Duration::from_secs(180);
        let mut next_poll = Instant::now() + Duration::from_secs(4);
        let mut closed_at: Option<Instant> = None;
        let mut startup = true;
        loop {
            sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            let running = sys
                .processes_by_exact_name(GAME_EXE.as_ref())
                .next()
                .is_some();
            let st = app.state::<AppState>();
            if running != prev_running {
                prev_running = running;
                st.game_running.store(running, Ordering::SeqCst);
                let _ = app.emit("game", json!({ "running": running }));
                crate::overlay::apply_hotkeys(&app);
                let overlay_on = st.prefs()["overlay"]["enabled"].as_bool().unwrap_or(true);
                if running {
                    closed_at = None;
                    interval = Duration::from_secs(120);
                    next_poll = Instant::now() + Duration::from_secs(30);
                    if overlay_on {
                        crate::overlay::set_visible(&app, true);
                    }
                } else {
                    closed_at = Some(Instant::now());
                    next_poll = Instant::now() + Duration::from_secs(15);
                    crate::overlay::layout(&app, "normal");
                    crate::overlay::set_visible(&app, false);
                }
            }
            let recently_closed = closed_at
                .map(|t| t.elapsed() < Duration::from_secs(600))
                .unwrap_or(false);
            let active = running || st.overlay_visible.load(Ordering::SeqCst) || recently_closed;
            let stale = now_ms() - st.last_sync.load(Ordering::SeqCst) > 5 * 60_000;
            let has_me = st.me().is_some() && st.api.has_key();
            // Right after a match ends (seen by the live module), look every 20 s for a few minutes.
            let fast = now_ms() < st.fast_poll_until.load(Ordering::SeqCst);
            if fast && next_poll > Instant::now() + Duration::from_secs(20) {
                next_poll = Instant::now() + Duration::from_secs(20);
            }
            if has_me && Instant::now() >= next_poll && ((startup && stale) || active || fast) {
                let r = sync_me(&app, Prio::Background, 5).await;
                let got_new = r
                    .as_ref()
                    .map(|v| v["new"].as_i64().unwrap_or(0) > 0)
                    .unwrap_or(false);
                // While the game runs, check every 75 s (one light request) so a finished match shows
                // up within a minute or two. Right after the game closes, every 30 s for 10 min.
                // Otherwise back off up to 12 min.
                interval = if running {
                    Duration::from_secs(75)
                } else if recently_closed {
                    Duration::from_secs(30)
                } else if got_new {
                    Duration::from_secs(120)
                } else {
                    (interval * 2).min(Duration::from_secs(720))
                };
                next_poll = Instant::now() + interval;
                if got_new {
                    if let Ok(true) = crate::transfer::weekly_backup_due(&st) {
                        let _ = crate::transfer::make_backup(&st, "weekly");
                    }
                }
            }
            startup = false;
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(10)) => {},
                _ = st.wake.notified() => { next_poll = Instant::now(); startup = true; },
            }
        }
    });
}
