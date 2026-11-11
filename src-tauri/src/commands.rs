//! Tauri commands: the only API the webview has. The HenrikDev key never crosses this boundary.
use crate::advisor::{self, Comfort};
use crate::api::{paths, Prio};
use crate::dataset::KbStore;
use crate::db::{self, now_ms};
use crate::parse::{arr, i, s, unwrap_data};
use crate::queries::{self, Filter};
use crate::state::{self, AppState, Me};
use crate::{overlay, recordings, sync, transfer};
use rusqlite::params;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

type Res = Result<Value, String>;
fn err<E: std::fmt::Display>(e: E) -> String {
    crate::api::redact(&e.to_string())
}
fn need_me(st: &AppState) -> Result<Me, String> {
    st.me().ok_or_else(|| "Set your Riot ID first.".to_string())
}

#[tauri::command]
pub fn app_state(app: AppHandle, st: State<'_, AppState>) -> Res {
    let me = st.me();
    let prefs = st.prefs();
    let rank = me
        .as_ref()
        .map(|m| queries::rank_now(&st.db.lock().unwrap(), &m.puuid));
    let matches: i64 = st
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM matches", [], |r| r.get(0))
        .unwrap_or(0);
    Ok(json!({
        "version": app.package_info().version.to_string(), "has_key": st.api.has_key(), "me": me, "rank": rank, "prefs": prefs,
        "data_dir": st.data_dir.to_string_lossy(), "portable": crate::is_portable(), "game_running": st.game_running.load(Ordering::SeqCst),
        "last_sync": st.last_sync.load(Ordering::SeqCst), "syncing": st.syncing.load(Ordering::SeqCst), "last_error": st.last_error.lock().unwrap().clone(),
        "usage": st.api.usage(), "overlay_visible": st.overlay_visible.load(Ordering::SeqCst), "matches": matches,
    }))
}

#[tauri::command]
pub fn save_prefs(app: AppHandle, st: State<'_, AppState>, patch: Value) -> Res {
    let merged = state::merge(st.prefs(), patch.clone());
    db::set_setting(&st.db.lock().unwrap(), "prefs", &merged.to_string()).map_err(err)?;
    if let Some(cap) = merged["daily_cap"].as_u64() {
        st.api.gov.lock().unwrap().daily_cap = cap as u32;
    }
    if patch.get("hotkeys").is_some() || patch.get("hotkeys_game_only").is_some() {
        overlay::apply_hotkeys(&app);
    }
    let _ = tauri::Emitter::emit(&app, "prefs", merged.clone());
    Ok(merged)
}

#[tauri::command]
pub async fn set_api_key(st: State<'_, AppState>, key: String) -> Res {
    let key = key.trim().to_string();
    if key.len() < 8 {
        return Err("That doesn't look like a HenrikDev key.".into());
    }
    let old = st.api.key.lock().unwrap().replace(key.clone());
    match st.api.get("/valorant/v1/status/eu", Prio::Foreground).await {
        Ok(_) | Err(crate::api::ApiError::Status(404, _)) => {
            state::save_key(&key)?;
            Ok(st.api.usage())
        }
        Err(e) => {
            *st.api.key.lock().unwrap() = old;
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub fn remove_api_key(st: State<'_, AppState>) -> Res {
    state::delete_key();
    *st.api.key.lock().unwrap() = None;
    Ok(json!(true))
}

#[tauri::command]
pub async fn lookup_account(st: State<'_, AppState>, name: String, tag: String) -> Res {
    let v = st
        .api
        .get(
            &paths::account(name.trim(), tag.trim().trim_start_matches('#')),
            Prio::Foreground,
        )
        .await
        .map_err(|e| e.to_string())?;
    let d = unwrap_data(&v);
    Ok(
        json!({ "puuid": s(d, "/puuid"), "name": s(d, "/name"), "tag": s(d, "/tag"), "region": s(d, "/region"), "account_level": i(d, "/account_level"), "card": s(d, "/card") }),
    )
}

#[tauri::command]
pub fn set_me(st: State<'_, AppState>, kb: State<'_, KbStore>, me: Me) -> Res {
    let kb = kb.get();
    let c = st.db.lock().unwrap();
    db::set_setting(&c, "me", &serde_json::to_string(&me).map_err(err)?).map_err(err)?;
    sync::ensure_starters(&c, &me.puuid, &kb);
    sync::update_owned_agents(&c, &me.puuid);
    drop(c);
    st.wake.notify_one();
    Ok(json!(me))
}

#[tauri::command]
pub async fn sync_now(app: AppHandle, backfill: Option<u32>) -> Res {
    let size = backfill.unwrap_or(5).clamp(1, 20);
    sync::sync_me(&app, Prio::Foreground, size).await
}

#[tauri::command]
pub fn home(st: State<'_, AppState>) -> Res {
    let me = need_me(&st)?;
    let prefs = st.prefs();
    Ok(queries::home(&st.db.lock().unwrap(), &me, &prefs))
}

#[tauri::command]
pub fn matches(
    st: State<'_, AppState>,
    queue: Option<String>,
    map: Option<String>,
    agent: Option<String>,
    limit: Option<i64>,
) -> Res {
    let me = need_me(&st)?;
    let f = Filter {
        queue: queue.unwrap_or_default(),
        map: map.unwrap_or_default(),
        agent: agent.unwrap_or_default(),
        since: 0,
        limit: limit.unwrap_or(200),
    };
    Ok(queries::match_list(&st.db.lock().unwrap(), &me, &f))
}

#[tauri::command]
pub fn match_detail(st: State<'_, AppState>, id: String) -> Res {
    let me = need_me(&st)?;
    queries::match_detail(&st.db.lock().unwrap(), &me, &id)
}

#[tauri::command]
pub fn analytics(st: State<'_, AppState>, days: Option<i64>, queue: Option<String>) -> Res {
    let me = need_me(&st)?;
    let since = days
        .filter(|d| *d > 0)
        .map(|d| now_ms() - d * 86_400_000)
        .unwrap_or(0);
    Ok(queries::analytics(
        &st.db.lock().unwrap(),
        &me,
        since,
        &queue.unwrap_or_else(|| "competitive".into()),
    ))
}

#[tauri::command]
pub fn habits(st: State<'_, AppState>) -> Res {
    let me = need_me(&st)?;
    Ok(queries::habits(&st.db.lock().unwrap(), &me))
}

#[tauri::command]
pub fn rank(st: State<'_, AppState>) -> Res {
    let me = need_me(&st)?;
    Ok(queries::rank(&st.db.lock().unwrap(), &me))
}

#[tauri::command]
pub fn people(st: State<'_, AppState>) -> Res {
    let me = need_me(&st)?;
    Ok(queries::people(&st.db.lock().unwrap(), &me))
}

/// Look up any player: account, rank and recent matches (3 requests), cached 10 minutes.
#[tauri::command]
pub async fn search_player(
    st: State<'_, AppState>,
    name: String,
    tag: String,
    force: Option<bool>,
) -> Res {
    let (name, tag) = (
        name.trim().to_string(),
        tag.trim().trim_start_matches('#').to_string(),
    );
    let key = format!("player:{}#{}", name.to_lowercase(), tag.to_lowercase());
    if !force.unwrap_or(false) {
        if let Some((v, at)) = db::cache_get(&st.db.lock().unwrap(), &key, 10 * 60_000) {
            return Ok(json!({ "data": v, "cached_at": at, "cached": true }));
        }
    }
    let acc = st
        .api
        .get(&paths::account(&name, &tag), Prio::Foreground)
        .await
        .map_err(|e| e.to_string())?;
    let a = unwrap_data(&acc).clone();
    let (puuid, region) = (s(&a, "/puuid").to_string(), s(&a, "/region").to_string());
    let mmr = match st
        .api
        .get(&paths::mmr(&region, &puuid), Prio::Foreground)
        .await
    {
        Ok(v) => json!({ "ok": unwrap_data(&v) }),
        Err(crate::api::ApiError::Status(code, _)) if code == 404 || code == 403 => {
            json!({ "private": true })
        }
        Err(e) => json!({ "error": e.to_string() }),
    };
    let mut summary = Value::Null;
    let recent = match st
        .api
        .get(
            &paths::stored_matches(&region, &puuid, 25, Some("competitive")),
            Prio::Foreground,
        )
        .await
    {
        Ok(v) => {
            summary = crate::career::from_stored(arr(&v, "/data"));
            json!(arr(&v, "/data").iter().map(|m| {
            let team = s(m, "/stats/team").to_lowercase();
            let (mine, theirs) = if team == "red" { (i(m, "/teams/red"), i(m, "/teams/blue")) } else { (i(m, "/teams/blue"), i(m, "/teams/red")) };
            let shots = i(m, "/stats/shots/head") + i(m, "/stats/shots/body") + i(m, "/stats/shots/leg");
            let rounds = (mine + theirs).max(1);
            json!({ "id": s(m, "/meta/id"), "map": s(m, "/meta/map/name"), "started_at": crate::parse::time_ms(m.pointer("/meta/started_at").unwrap_or(&Value::Null)),
                "agent": s(m, "/stats/character/name"), "agent_id": s(m, "/stats/character/id"), "tier": i(m, "/stats/tier"),
                "k": i(m, "/stats/kills"), "d": i(m, "/stats/deaths"), "a": i(m, "/stats/assists"), "acs": i(m, "/stats/score") / rounds,
                "hs": if shots > 0 { 100 * i(m, "/stats/shots/head") / shots } else { 0 }, "score": [mine, theirs],
                "result": if mine > theirs { "W" } else if mine < theirs { "L" } else { "D" } })
        }).collect::<Vec<_>>())
        }
        Err(crate::api::ApiError::Status(code, _)) if code == 404 || code == 403 => {
            json!({ "private": true })
        }
        Err(e) => json!({ "error": e.to_string() }),
    };
    let data = json!({ "account": a, "mmr": mmr, "recent": recent, "summary": summary });
    let c = st.db.lock().unwrap();
    let _ = db::cache_put(&c, &key, &data);
    let mut recent_list: Vec<Value> = db::get_json(&c, "recent_searches")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    recent_list.retain(|x| x["puuid"] != puuid);
    recent_list.insert(
        0,
        json!({ "puuid": puuid, "name": s(&a, "/name"), "tag": s(&a, "/tag"), "at": now_ms() }),
    );
    recent_list.truncate(12);
    let _ = db::set_setting(&c, "recent_searches", &json!(recent_list).to_string());
    Ok(json!({ "data": data, "cached_at": now_ms(), "cached": false }))
}

#[tauri::command]
pub fn recent_searches(st: State<'_, AppState>) -> Res {
    let c = st.db.lock().unwrap();
    let notes = queries::notes_map(&c);
    let list: Vec<Value> = db::get_json(&c, "recent_searches")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    Ok(json!(list
        .into_iter()
        .map(|mut v| {
            let p = v["puuid"].as_str().unwrap_or("").to_string();
            v["note"] = notes.get(&p).cloned().unwrap_or(Value::Null);
            v
        })
        .collect::<Vec<_>>()))
}

#[tauri::command]
pub fn save_note(st: State<'_, AppState>, puuid: String, note: String, tags: Vec<String>) -> Res {
    let c = st.db.lock().unwrap();
    if note.trim().is_empty() && tags.is_empty() {
        c.execute("DELETE FROM player_notes WHERE puuid=?1", [&puuid])
            .map_err(err)?;
    } else {
        c.execute(
            "INSERT INTO player_notes(puuid, tags_json, note, updated_at) VALUES (?1,?2,?3,?4)
             ON CONFLICT(puuid) DO UPDATE SET tags_json=excluded.tags_json, note=excluded.note, updated_at=excluded.updated_at",
            params![puuid, json!(tags).to_string(), note.trim(), now_ms()],
        )
        .map_err(err)?;
    }
    Ok(json!(true))
}

#[tauri::command]
pub fn reviews(st: State<'_, AppState>, done: Option<bool>) -> Res {
    let me = need_me(&st)?;
    Ok(queries::reviews(
        &st.db.lock().unwrap(),
        &me,
        done.unwrap_or(false),
    ))
}

#[tauri::command]
pub fn add_review(
    st: State<'_, AppState>,
    match_id: String,
    round: Option<i64>,
    kind: String,
    note: Option<String>,
    t: Option<i64>,
) -> Res {
    let c = st.db.lock().unwrap();
    c.execute("INSERT INTO review_items(match_id, round_no, kind, note, time_in_match_ms, created_at) VALUES (?1,?2,?3,?4,?5,?6)", params![match_id, round, kind, note, t, now_ms()]).map_err(err)?;
    Ok(json!(c.last_insert_rowid()))
}

#[tauri::command]
pub fn update_review(
    st: State<'_, AppState>,
    id: i64,
    note: Option<String>,
    done: Option<bool>,
    delete: Option<bool>,
) -> Res {
    let c = st.db.lock().unwrap();
    if delete.unwrap_or(false) {
        c.execute("DELETE FROM review_items WHERE id=?1", [id])
            .map_err(err)?;
        return Ok(json!(true));
    }
    if let Some(n) = note {
        c.execute(
            "UPDATE review_items SET note=?2 WHERE id=?1",
            params![id, n],
        )
        .map_err(err)?;
    }
    if let Some(d) = done {
        c.execute(
            "UPDATE review_items SET done_at=?2 WHERE id=?1",
            params![id, if d { Some(now_ms()) } else { None }],
        )
        .map_err(err)?;
    }
    Ok(json!(true))
}

fn comfort_map(st: &AppState, me: &Me, map: Option<&str>) -> HashMap<String, Comfort> {
    let rows = queries::my_rows(
        &st.db.lock().unwrap(),
        &me.puuid,
        &Filter {
            queue: "competitive".into(),
            ..Default::default()
        },
    );
    let mut m: HashMap<String, Comfort> = HashMap::new();
    for r in rows {
        let e = m.entry(r.agent.clone()).or_default();
        e.games += 1;
        e.wins += r.won() as i64;
        if map.map(|x| x.eq_ignore_ascii_case(&r.map)).unwrap_or(false) {
            e.map_games += 1;
            e.map_wins += r.won() as i64;
        }
    }
    m
}

fn owned(st: &AppState, me: &Me) -> HashSet<String> {
    let c = st.db.lock().unwrap();
    c.prepare("SELECT agent_id FROM agent_pool WHERE puuid=?1")
        .and_then(|mut s| {
            s.query_map([&me.puuid], |r| r.get::<_, String>(0))
                .map(|x| x.flatten().collect())
        })
        .unwrap_or_default()
}

#[tauri::command]
pub fn agents_kb(st: State<'_, AppState>, kb: State<'_, KbStore>) -> Res {
    let kb = kb.get();
    let mut v = kb.json();
    if let Some(me) = st.me() {
        let o = owned(&st, &me);
        let cm = comfort_map(&st, &me, None);
        v["owned"] = json!(o);
        v["comfort"] = json!(cm
            .iter()
            .map(|(k, c)| (k.clone(), json!({ "games": c.games, "wins": c.wins })))
            .collect::<serde_json::Map<_, _>>());
    }
    Ok(v)
}

#[tauri::command]
pub fn set_owned(st: State<'_, AppState>, agent_id: String, owned: bool) -> Res {
    let me = need_me(&st)?;
    let c = st.db.lock().unwrap();
    if owned {
        c.execute("INSERT INTO agent_pool(puuid, agent_id, comfort, manual) VALUES (?1,?2,2,1) ON CONFLICT DO UPDATE SET manual=1", params![me.puuid, agent_id]).map_err(err)?;
    } else {
        c.execute(
            "DELETE FROM agent_pool WHERE puuid=?1 AND agent_id=?2",
            params![me.puuid, agent_id],
        )
        .map_err(err)?;
    }
    Ok(json!(true))
}

#[tauri::command]
pub fn advisor_suggest(
    st: State<'_, AppState>,
    kb: State<'_, KbStore>,
    map: String,
    team: Vec<String>,
) -> Res {
    let kb = kb.get();
    let me = need_me(&st)?;
    let inp = advisor::Input {
        map: &map,
        team,
        owned: owned(&st, &me),
        comfort: comfort_map(&st, &me, Some(&map)),
    };
    advisor::suggest(&kb, &inp)
}

#[tauri::command]
pub fn advisor_learn(st: State<'_, AppState>, kb: State<'_, KbStore>, map: String) -> Res {
    let kb = kb.get();
    let me = need_me(&st)?;
    advisor::learn(
        &kb,
        &map,
        &owned(&st, &me),
        &comfort_map(&st, &me, Some(&map)),
    )
}

#[tauri::command]
pub fn export_backup(st: State<'_, AppState>, path: String) -> Res {
    let me = st.me();
    let c = st.db.lock().unwrap();
    transfer::write_backup(
        &c,
        me.as_ref(),
        &st.data_dir.join("tmp"),
        &PathBuf::from(path),
    )
    .map_err(err)
}

#[tauri::command]
pub fn export_json(
    st: State<'_, AppState>,
    path: String,
    anonymise: bool,
    days: Option<i64>,
) -> Res {
    let me = st.me();
    let since = days
        .filter(|d| *d > 0)
        .map(|d| now_ms() - d * 86_400_000)
        .unwrap_or(0);
    transfer::export_json(
        &st.db.lock().unwrap(),
        me.as_ref(),
        &PathBuf::from(path),
        anonymise,
        since,
    )
    .map_err(err)
}

#[tauri::command]
pub fn inspect_import(st: State<'_, AppState>, path: String) -> Res {
    transfer::inspect(&st, &PathBuf::from(path)).map_err(err)
}

#[tauri::command]
pub fn apply_import(
    st: State<'_, AppState>,
    path: String,
    mode: String,
    notes: String,
    include_settings: bool,
) -> Res {
    let p = PathBuf::from(&path);
    if p.extension().and_then(|e| e.to_str()) == Some("json") {
        return transfer::import_json(&mut st.db.lock().unwrap(), &p).map_err(err);
    }
    let r = transfer::apply(&st, &p, &mode, &notes, include_settings).map_err(err)?;
    st.wake.notify_one();
    Ok(r)
}

#[tauri::command]
pub fn list_backups(st: State<'_, AppState>) -> Res {
    Ok(transfer::list_backups(&st))
}

#[tauri::command]
pub fn backup_now(st: State<'_, AppState>) -> Res {
    transfer::make_backup(&st, "manual")
        .map(|p| json!(p.to_string_lossy()))
        .map_err(err)
}

#[tauri::command]
pub fn restore_backup(st: State<'_, AppState>, id: i64) -> Res {
    let path: String = st
        .db
        .lock()
        .unwrap()
        .query_row("SELECT path FROM backups WHERE id=?1", [id], |r| r.get(0))
        .map_err(err)?;
    transfer::apply(&st, &PathBuf::from(path), "replace", "newer", true).map_err(err)
}

#[tauri::command]
pub fn recordings_scan(st: State<'_, AppState>, dir: Option<String>) -> Res {
    let dir = match dir {
        Some(d) => {
            let merged = state::merge(st.prefs(), json!({ "recordings_dir": d }));
            db::set_setting(&st.db.lock().unwrap(), "prefs", &merged.to_string()).map_err(err)?;
            d
        }
        None => st.prefs()["recordings_dir"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or("Choose your recordings folder first.")?,
    };
    recordings::scan(&st.db.lock().unwrap(), &PathBuf::from(dir)).map_err(err)
}

/// Open the recording for a match at a moment (ms since match start), 5 s early.
#[tauri::command]
pub fn watch(st: State<'_, AppState>, match_id: String, t: i64) -> Res {
    let rec = recordings::for_match(&st.db.lock().unwrap(), &match_id);
    let path = rec["path"]
        .as_str()
        .ok_or("No recording found for this match.")?
        .to_string();
    let secs = (rec["offset_ms"].as_i64().unwrap_or(0) + t) / 1000 - 5;
    let mut r = recordings::open_at(&path, secs);
    if r["how"] == "default" {
        let _ = tauri_plugin_opener::open_path(&path, None::<&str>);
    }
    r["seconds"] = json!(secs.max(0));
    Ok(r)
}

#[tauri::command]
pub fn overlay_cmd(app: AppHandle, action: String) -> Res {
    match action.as_str() {
        "show" => overlay::set_visible(&app, true),
        "hide" => overlay::set_visible(&app, false),
        "edit_on" => overlay::set_edit(&app, true),
        "edit_off" => overlay::set_edit(&app, false),
        a => overlay::run_action(&app, a),
    }
    Ok(json!(true))
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, on: bool) -> Res {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    if on { al.enable() } else { al.disable() }.map_err(err)?;
    Ok(json!(al.is_enabled().unwrap_or(on)))
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> Res {
    use tauri_plugin_autostart::ManagerExt;
    Ok(json!(app.autolaunch().is_enabled().unwrap_or(false)))
}

/// Compare with the latest GitHub release (plan 20.3, v0.1 behaviour). Never during a match.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Res {
    crate::updater::check(&app).await
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Res {
    crate::updater::install(&app).await.map(|_| json!(true))
}

#[tauri::command]
pub fn delete_all_data(app: AppHandle, st: State<'_, AppState>, remove_key: bool) -> Res {
    {
        let c = st.db.lock().unwrap();
        for t in db::DATA_TABLES
            .iter()
            .chain(["cache", "settings", "fetch_log", "api_usage", "recordings"].iter())
        {
            c.execute(&format!("DELETE FROM {t}"), []).map_err(err)?;
        }
    }
    if remove_key {
        state::delete_key();
        *st.api.key.lock().unwrap() = None;
    }
    let _ = tauri::Emitter::emit(&app, "reset", ());
    Ok(json!(true))
}

#[tauri::command]
pub fn usage(st: State<'_, AppState>) -> Res {
    Ok(st.api.usage())
}

#[tauri::command]
pub fn overlay_data(st: State<'_, AppState>, kb: State<'_, KbStore>) -> Res {
    let kb = kb.get();
    let Some(me) = st.me() else {
        return Ok(Value::Null);
    };
    let prefs = st.prefs();
    let c = st.db.lock().unwrap();
    let h = queries::home(&c, &me, &prefs);
    let last_id = h["last"]["id"].as_str().map(|s| s.to_string());
    let board = last_id.and_then(|id| {
        let mut b = queries::match_detail(&c, &me, &id).ok()?;
        b["facts"] = crate::insights::match_extras(&c, &kb, &me, &id)["facts"].clone();
        Some(b)
    });
    Ok(json!({ "home": h, "board": board, "prefs": prefs, "me": me }))
}

// ---- Live match, careers, peaks, stack planner, heatmaps, replay ----

#[tauri::command]
pub fn live_status(live: State<'_, crate::live::Live>) -> Res {
    Ok(live.status())
}

/// Your lifetime stats from stored matches (no requests). Cached until a new match arrives.
#[tauri::command]
pub fn my_career(st: State<'_, AppState>, queue: Option<String>) -> Res {
    let me = need_me(&st)?;
    let q = queue.unwrap_or_default();
    let c = st.db.lock().unwrap();
    let (n, newest): (i64, i64) = c
        .query_row(
            "SELECT COUNT(*), COALESCE(MAX(started_at),0) FROM matches",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((0, 0));
    let key = format!("{n}:{newest}:{q}");
    if let Some((k, v)) = st.career_cache.lock().unwrap().as_ref() {
        if *k == key {
            return Ok(v.clone());
        }
    }
    let v = crate::players::career_local(&c, &me.puuid, &q);
    *st.career_cache.lock().unwrap() = Some((key, v.clone()));
    Ok(v)
}

/// Any player's career. `n` > 0 fetches up to n recent matches in full (1 request each, kept
/// forever); n = 0 returns what we already hold.
#[tauri::command]
pub async fn player_career(
    st: State<'_, AppState>,
    puuid: String,
    region: String,
    n: Option<u32>,
    queue: Option<String>,
) -> Res {
    let q = queue.unwrap_or_default();
    if st.me().map(|m| m.puuid == puuid).unwrap_or(false) || n.unwrap_or(0) == 0 {
        return Ok(crate::players::career_local(
            &st.db.lock().unwrap(),
            &puuid,
            &q,
        ));
    }
    crate::players::load_career(&st, &region, &puuid, n.unwrap_or(10), &q).await
}

/// Current rank, peak rank and last act for anyone. Cached 6 hours.
#[tauri::command]
pub async fn player_rank(st: State<'_, AppState>, puuid: String, region: Option<String>) -> Res {
    let region = region
        .filter(|r| !r.is_empty())
        .or_else(|| st.me().map(|m| m.region))
        .unwrap_or_else(|| "eu".into());
    Ok(crate::players::mmr(&st, &region, &puuid, 6 * 3_600_000, Prio::Foreground).await)
}

/// Ranks and peaks of everyone named in a stored match, cached so reopening is free. Hidden-name
/// players are skipped: Postplant never looks them up.
#[tauri::command]
pub async fn match_ranks(st: State<'_, AppState>, id: String) -> Res {
    let me = need_me(&st)?;
    let pus: Vec<(String, bool)> = {
        let c = st.db.lock().unwrap();
        let raw = crate::parse::load_raw(&c, &id).map_err(err)?;
        arr(unwrap_data(&raw), "/players")
            .iter()
            .map(|p| (s(p, "/puuid").to_string(), s(p, "/name").is_empty()))
            .collect()
    };
    let mut out = serde_json::Map::new();
    for (pu, hidden) in pus {
        if hidden || pu.is_empty() {
            continue;
        }
        let v = crate::players::mmr(&st, &me.region, &pu, 6 * 3_600_000, Prio::Foreground).await;
        out.insert(pu, v);
    }
    Ok(Value::Object(out))
}

#[tauri::command]
pub fn test_hotkey(app: AppHandle, combo: String) -> Res {
    overlay::test_combo(&app, &combo).map(|_| json!(true))
}

/// People you have queued with, for the stack planner: name, games together and their agents.
#[tauri::command]
pub fn stack_people(st: State<'_, AppState>) -> Res {
    let me = need_me(&st)?;
    let c = st.db.lock().unwrap();
    let mut q = c
        .prepare(
            "SELECT p.puuid, MAX(p.name), MAX(p.tag), COUNT(*) FROM match_players p
             JOIN match_players me ON me.match_id=p.match_id AND me.puuid=?1
             WHERE p.puuid<>?1 AND p.party_id IS NOT NULL AND p.party_id<>'' AND p.party_id=me.party_id
             GROUP BY p.puuid ORDER BY COUNT(*) DESC LIMIT 20",
        )
        .map_err(err)?;
    let rows: Vec<(String, String, String, i64)> = q
        .query_map([&me.puuid], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .map_err(err)?
        .flatten()
        .collect();
    let agents = |pu: &str| -> Vec<Value> {
        c.prepare("SELECT agent_name, COUNT(*) FROM match_players WHERE puuid=?1 GROUP BY agent_name ORDER BY COUNT(*) DESC LIMIT 6")
            .and_then(|mut q| {
                q.query_map([pu], |r| Ok(json!({ "name": r.get::<_, String>(0)?, "games": r.get::<_, i64>(1)? })))
                    .map(|x| x.flatten().collect())
            })
            .unwrap_or_default()
    };
    Ok(json!(rows
        .iter()
        .map(|(pu, n, t, g)| json!({ "puuid": pu, "name": n, "tag": t, "games": g, "agents": agents(pu) }))
        .collect::<Vec<_>>()))
}

fn comfort_for(c: &rusqlite::Connection, puuid: &str, map: &str) -> HashMap<String, Comfort> {
    let mut m: HashMap<String, Comfort> = HashMap::new();
    let Ok(mut q) = c.prepare(
        "SELECT mp.agent_name, m.map_name, CASE WHEN lower(m.winning_team)=lower(mp.team) THEN 1 ELSE 0 END
         FROM match_players mp JOIN matches m ON m.match_id=mp.match_id WHERE mp.puuid=?1",
    ) else {
        return m;
    };
    if let Ok(rows) = q.query_map([puuid], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    }) {
        for (agent, mp, won) in rows.flatten() {
            let e = m.entry(agent).or_default();
            e.games += 1;
            e.wins += won;
            if mp.eq_ignore_ascii_case(map) {
                e.map_games += 1;
                e.map_wins += won;
            }
        }
    }
    m
}

/// Stack planner. `players`: [{ name, puuid?, me?, pool: [agent names] }].
#[tauri::command]
pub fn advisor_stack(
    st: State<'_, AppState>,
    kb: State<'_, KbStore>,
    map: String,
    players: Vec<Value>,
) -> Res {
    let kb = kb.get();
    let me = st.me();
    let mut list = vec![];
    for p in &players {
        let mut pool: Vec<String> = p["pool"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let comfort = if p["me"].as_bool().unwrap_or(false) {
            let me = me.as_ref().ok_or("Set your Riot ID first.")?;
            if pool.is_empty() {
                let o = owned(&st, me);
                pool = kb
                    .agents
                    .iter()
                    .filter(|a| o.contains(&a.agent_id))
                    .map(|a| a.name.clone())
                    .collect();
            }
            comfort_map(&st, me, Some(&map))
        } else {
            match p["puuid"].as_str() {
                Some(pu) if !pu.is_empty() => comfort_for(&st.db.lock().unwrap(), pu, &map),
                _ => HashMap::new(),
            }
        };
        list.push(advisor::StackPlayer {
            name: p["name"].as_str().unwrap_or("Player").to_string(),
            pool,
            comfort,
        });
    }
    advisor::stack(&kb, &map, &list)
}

/// Heatmap points and the callouts they cluster in, from your stored matches (zero requests).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn heatmap(
    st: State<'_, AppState>,
    kb: State<'_, KbStore>,
    map: String,
    event: Option<String>,
    side: Option<String>,
    layer: Option<String>,
    agent: Option<String>,
    outcome: Option<String>,
    rtype: Option<String>,
) -> Res {
    let kb = kb.get();
    let me = need_me(&st)?;
    let o = crate::insights::HeatOpts {
        map,
        event: event.unwrap_or_else(|| "deaths".into()),
        side: side.unwrap_or_default(),
        layer: layer.unwrap_or_else(|| "me".into()),
        agent: agent.unwrap_or_default(),
        outcome: outcome.unwrap_or_default(),
        rtype: rtype.unwrap_or_default(),
    };
    Ok(crate::insights::heatmap(
        &st.db.lock().unwrap(),
        &kb,
        &me,
        &o,
    ))
}

#[tauri::command]
pub fn trade_stats(st: State<'_, AppState>) -> Res {
    let me = need_me(&st)?;
    Ok(crate::insights::trades(&st.db.lock().unwrap(), &me))
}

#[tauri::command]
pub fn pattern_notes(st: State<'_, AppState>, kb: State<'_, KbStore>) -> Res {
    let kb = kb.get();
    let me = need_me(&st)?;
    Ok(crate::insights::patterns(&st.db.lock().unwrap(), &kb, &me))
}

#[tauri::command]
pub fn duo_synergy(st: State<'_, AppState>) -> Res {
    let me = need_me(&st)?;
    Ok(crate::insights::duos(&st.db.lock().unwrap(), &me))
}

/// Post-match facts and ability usage vs your average on that agent.
#[tauri::command]
pub fn match_extras(st: State<'_, AppState>, kb: State<'_, KbStore>, id: String) -> Res {
    let kb = kb.get();
    let me = need_me(&st)?;
    Ok(crate::insights::match_extras(
        &st.db.lock().unwrap(),
        &kb,
        &me,
        &id,
    ))
}

/// Round-by-round positions for the 2D replay: every kill with where everyone stood at that moment.
#[tauri::command]
pub fn match_replay(st: State<'_, AppState>, id: String) -> Res {
    use crate::parse::f;
    let c = st.db.lock().unwrap();
    let raw = crate::parse::load_raw(&c, &id).map_err(err)?;
    let m = unwrap_data(&raw);
    let xy = |v: &Value, p: &str| json!([f(v, &format!("{p}/x")), f(v, &format!("{p}/y"))]);
    let players: Vec<Value> = arr(m, "/players")
        .iter()
        .map(|p| json!({ "puuid": s(p, "/puuid"), "team": s(p, "/team_id"), "agent": s(p, "/agent/name"), "agent_id": s(p, "/agent/id"), "name": s(p, "/name") }))
        .collect();
    let rounds: Vec<Value> = arr(m, "/rounds")
        .iter()
        .enumerate()
        .map(|(n, r)| {
            let kills: Vec<Value> = arr(m, "/kills")
                .iter()
                .filter(|k| i(k, "/round") == n as i64)
                .map(|k| {
                    json!({ "t": i(k, "/time_in_round_in_ms"), "killer": s(k, "/killer/puuid"), "victim": s(k, "/victim/puuid"),
                        "weapon": s(k, "/weapon/name"), "at": xy(k, "/location"),
                        "pos": arr(k, "/player_locations").iter().map(|l| json!([s(l, "/player/puuid"), f(l, "/location/x"), f(l, "/location/y")])).collect::<Vec<_>>() })
                })
                .collect();
            let ev = |key: &str| {
                r.pointer(key)
                    .filter(|p| !p.is_null())
                    .map(|p| json!({ "t": i(p, "/round_time_in_ms"), "site": s(p, "/site"), "at": xy(p, "/location") }))
            };
            json!({ "n": n, "winner": s(r, "/winning_team"), "result": s(r, "/result"), "kills": kills, "plant": ev("/plant"), "defuse": ev("/defuse") })
        })
        .collect();
    Ok(json!({ "id": id, "map": s(m, "/metadata/map/name"), "players": players, "rounds": rounds }))
}

/// Compare HenrikDev's real responses with the fields Postplant reads (1 request).
#[tauri::command]
pub async fn field_check(st: State<'_, AppState>) -> Res {
    crate::fieldcheck::run(&st).await
}

#[tauri::command]
pub fn dataset_status(app: AppHandle) -> Res {
    Ok(crate::dataset::status(&app))
}

#[tauri::command]
pub async fn check_dataset(app: AppHandle) -> Res {
    crate::dataset::check(&app).await
}
