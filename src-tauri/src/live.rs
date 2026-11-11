//! Live match module (plan section 11). OPT-IN, off by default, behind a warning the user accepts.
//!
//! Reads the local Riot Client (lockfile → local API on 127.0.0.1) for:
//!   * the game phase, from your own presence (MENUS / PREGAME / INGAME), and the map;
//!   * once the match is IN GAME: who is in it (agents, teams, levels, names unless hidden).
//!
//! Ranks and recent form come from HenrikDev by PUUID through the budget governor.
//!
//! Guardrails, checked here in code:
//!   * During agent select (PREGAME) nothing about other players is requested. The only thing that
//!     happens is opening YOUR Comp Advisor with the map preset; teammates' picks are still typed in.
//!   * Players with a hidden name are shown by agent name and never looked up.
//!   * Tokens stay in memory, are only sent to Riot's own hosts, and are dropped when the toggle is off.
use crate::api::Prio;
use crate::db::now_ms;
use crate::state::AppState;
use base64::Engine;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const PLATFORM: &str = "ew0KCSJwbGF0Zm9ybVR5cGUiOiAiUEMiLA0KCSJwbGF0Zm9ybU9TIjogIldpbmRvd3MiLA0KCSJwbGF0Zm9ybU9TVmVyc2lvbiI6ICIxMC4wLjE5MDQyLjEuMjU2LjY0Yml0IiwNCgkicGxhdGZvcm1DaGlwc2V0IjogIlVua25vd24iDQp9";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Off,
    NoClient,
    Menus,
    Pregame,
    Ingame,
}

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Off => "off",
            Phase::NoClient => "no_client",
            Phase::Menus => "menus",
            Phase::Pregame => "pregame",
            Phase::Ingame => "ingame",
        }
    }
}

#[derive(Clone)]
struct Session {
    port: u16,
    password: String,
    puuid: String,
    access: String,
    entitlement: String,
    region: String,
    shard: String,
    version: String,
    at: Instant,
}

pub struct Live {
    session: Mutex<Option<Session>>,
    pub phase: Mutex<Phase>,
    pub map: Mutex<String>,
    pub match_id: Mutex<String>,
    pub roster: Mutex<Value>,
    pub error: Mutex<Option<String>>,
    gen: AtomicU64,
    pub tab_down: AtomicBool,
    pub table_open: AtomicBool,
    owned_synced: AtomicBool,
}

impl Default for Live {
    fn default() -> Self {
        Self {
            session: Mutex::new(None),
            phase: Mutex::new(Phase::Off),
            map: Mutex::new(String::new()),
            match_id: Mutex::new(String::new()),
            roster: Mutex::new(Value::Null),
            error: Mutex::new(None),
            gen: AtomicU64::new(0),
            tab_down: AtomicBool::new(false),
            table_open: AtomicBool::new(false),
            owned_synced: AtomicBool::new(false),
        }
    }
}

impl Live {
    pub fn status(&self) -> Value {
        json!({ "phase": self.phase.lock().unwrap().as_str(), "map": *self.map.lock().unwrap(),
            "roster": *self.roster.lock().unwrap(), "error": *self.error.lock().unwrap(),
            "table": self.table_open.load(Ordering::SeqCst) })
    }
    fn clear(&self) {
        *self.session.lock().unwrap() = None;
        *self.roster.lock().unwrap() = Value::Null;
        *self.match_id.lock().unwrap() = String::new();
        *self.map.lock().unwrap() = String::new();
        self.gen.fetch_add(1, Ordering::SeqCst);
        self.table_open.store(false, Ordering::SeqCst);
    }
}

fn local_http() -> reqwest::Client {
    // The Riot Client's local API uses a self-signed certificate on 127.0.0.1 only.
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(4))
        .build()
        .expect("local http")
}

fn remote_http() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .expect("remote http")
}

/// `name:pid:port:password:protocol`
pub fn parse_lockfile(s: &str) -> Option<(u16, String)> {
    let p: Vec<&str> = s.trim().split(':').collect();
    if p.len() < 5 {
        return None;
    }
    Some((p[2].parse().ok()?, p[3].to_string()))
}

fn read_lockfile() -> Option<(u16, String)> {
    let base = std::env::var("LOCALAPPDATA").ok()?;
    let path = std::path::Path::new(&base)
        .join("Riot Games")
        .join("Riot Client")
        .join("Config")
        .join("lockfile");
    // The client keeps the file open; read it with shared access.
    let s = std::fs::read_to_string(path).ok()?;
    parse_lockfile(&s)
}

async fn local_get(
    http: &reqwest::Client,
    port: u16,
    pw: &str,
    path: &str,
) -> Result<Value, String> {
    let r = http
        .get(format!("https://127.0.0.1:{port}{path}"))
        .basic_auth("riot", Some(pw))
        .send()
        .await
        .map_err(|_| "Riot Client not reachable".to_string())?;
    if !r.status().is_success() {
        return Err(format!("Riot Client answered {}", r.status().as_u16()));
    }
    r.json()
        .await
        .map_err(|_| "Unexpected Riot Client response".to_string())
}

/// From `/product-session/v1/external-sessions`: region (ares deployment), shard and client version.
pub fn parse_sessions(v: &Value) -> Option<(String, String, String)> {
    let obj = v.as_object()?;
    for sess in obj.values() {
        if sess["productId"] != "valorant" {
            continue;
        }
        let args: Vec<&str> = sess
            .pointer("/launchConfiguration/arguments")
            .and_then(|a| a.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
            .unwrap_or_default();
        let region = args
            .iter()
            .find_map(|a| a.strip_prefix("-ares-deployment="))
            .unwrap_or("")
            .to_string();
        let shard = args
            .iter()
            .find_map(|a| a.strip_prefix("-config-endpoint=https://shared."))
            .and_then(|x| x.split('.').next())
            .unwrap_or(region.as_str())
            .to_string();
        let version = sess["version"].as_str().unwrap_or("").to_string();
        if !region.is_empty() {
            return Some((region, shard, version));
        }
    }
    None
}

/// Your own presence → (phase, map path, queue). Handles the older flat and the newer nested formats.
pub fn parse_presence(presences: &Value, puuid: &str) -> Option<(Phase, String, String)> {
    let list = presences["presences"].as_array()?;
    let me = list
        .iter()
        .find(|p| p["puuid"] == puuid && p["product"] == "valorant")?;
    let raw = me["private"].as_str()?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(raw).ok()?;
    let p: Value = serde_json::from_slice(&bytes).ok()?;
    let pick = |a: &str, b: &str| -> String {
        p.pointer(a)
            .and_then(|x| x.as_str())
            .or_else(|| p.pointer(b).and_then(|x| x.as_str()))
            .unwrap_or("")
            .to_string()
    };
    let state = pick("/sessionLoopState", "/matchPresenceData/sessionLoopState");
    let map = pick("/matchMap", "/matchPresenceData/matchMap");
    let queue = pick("/queueId", "/queueId");
    let phase = match state.as_str() {
        "PREGAME" => Phase::Pregame,
        "INGAME" => Phase::Ingame,
        _ => Phase::Menus,
    };
    Some((phase, map, queue))
}

async fn connect(local: &reqwest::Client) -> Result<Session, String> {
    let (port, password) = read_lockfile().ok_or("Riot Client isn't running")?;
    let tok = local_get(local, port, &password, "/entitlements/v1/token").await?;
    let sessions = local_get(
        local,
        port,
        &password,
        "/product-session/v1/external-sessions",
    )
    .await?;
    let (region, shard, version) = parse_sessions(&sessions).ok_or("VALORANT session not found")?;
    Ok(Session {
        port,
        password,
        puuid: tok["subject"].as_str().unwrap_or("").to_string(),
        access: tok["accessToken"].as_str().unwrap_or("").to_string(),
        entitlement: tok["token"].as_str().unwrap_or("").to_string(),
        region,
        shard,
        version,
        at: Instant::now(),
    })
}

async fn riot_get(http: &reqwest::Client, s: &Session, url: &str) -> Result<Value, u16> {
    let r = http
        .get(url)
        .bearer_auth(&s.access)
        .header("X-Riot-Entitlements-JWT", &s.entitlement)
        .header("X-Riot-ClientPlatform", PLATFORM)
        .header("X-Riot-ClientVersion", &s.version)
        .send()
        .await
        .map_err(|_| 0u16)?;
    let code = r.status().as_u16();
    if code >= 400 {
        return Err(code);
    }
    r.json().await.map_err(|_| 0)
}

async fn riot_put(
    http: &reqwest::Client,
    s: &Session,
    url: &str,
    body: &Value,
) -> Result<Value, u16> {
    let r = http
        .put(url)
        .bearer_auth(&s.access)
        .header("X-Riot-Entitlements-JWT", &s.entitlement)
        .header("X-Riot-ClientPlatform", PLATFORM)
        .header("X-Riot-ClientVersion", &s.version)
        .json(body)
        .send()
        .await
        .map_err(|_| 0u16)?;
    let code = r.status().as_u16();
    if code >= 400 {
        return Err(code);
    }
    r.json().await.map_err(|_| 0)
}

fn glz(s: &Session) -> String {
    format!("https://glz-{}-1.{}.a.pvp.net", s.region, s.shard)
}
fn pd(s: &Session) -> String {
    format!("https://pd.{}.a.pvp.net", s.shard)
}

fn enabled(st: &AppState) -> bool {
    let p = st.prefs();
    p["live_enabled"].as_bool().unwrap_or(false) && p["live_ack"]["at"].is_string()
}

fn set_phase(app: &AppHandle, live: &Live, ph: Phase) {
    let prev = { std::mem::replace(&mut *live.phase.lock().unwrap(), ph) };
    if prev != ph {
        let _ = app.emit("live-phase", json!({ "phase": ph.as_str(), "prev": prev.as_str(), "map": *live.map.lock().unwrap() }));
    }
}

/// Map path from presence ("/Game/Maps/Duality/Duality") → map name via the knowledge base.
fn map_name(app: &AppHandle, path: &str) -> String {
    let kb = app.state::<crate::advisor::Kb>();
    kb.maps
        .iter()
        .find(|m| !m.map_url.is_empty() && m.map_url.eq_ignore_ascii_case(path))
        .map(|m| m.name.clone())
        .unwrap_or_default()
}

/// Agents you own, from the client's entitlements (plan 14.1a, live mode only). Once per app run.
async fn sync_owned(app: &AppHandle, s: &Session) {
    let live = app.state::<Live>();
    if live.owned_synced.swap(true, Ordering::SeqCst) {
        return;
    }
    let url = format!(
        "{}/store/v1/entitlements/{}/01bb38e1-da47-4e6a-9b3d-945fe4655707",
        pd(s),
        s.puuid
    );
    if let Ok(v) = riot_get(&remote_http(), s, &url).await {
        let st = app.state::<AppState>();
        let c = st.db.lock().unwrap();
        for e in v["Entitlements"].as_array().cloned().unwrap_or_default() {
            if let Some(id) = e["ItemID"].as_str() {
                let _ = c.execute(
                    "INSERT OR IGNORE INTO agent_pool(puuid, agent_id, comfort, manual) VALUES (?1,?2,2,0)",
                    rusqlite::params![s.puuid, id.to_lowercase()],
                );
            }
        }
    }
}

/// Start the live loop. Polls only your own presence on 127.0.0.1 every 2 s while the game runs
/// and the toggle is on; sleeps otherwise.
pub fn start(app: AppHandle) {
    #[cfg(windows)]
    start_tab_watch(app.clone());
    tauri::async_runtime::spawn(async move {
        let local = local_http();
        let remote = remote_http();
        loop {
            let st = app.state::<AppState>();
            let live = app.state::<Live>();
            let on = enabled(&st) && st.game_running.load(Ordering::SeqCst);
            if !on {
                if *live.phase.lock().unwrap() != Phase::Off {
                    on_leave(&app, *live.phase.lock().unwrap());
                    live.clear();
                    set_phase(&app, &live, Phase::Off);
                }
                tokio::time::sleep(Duration::from_secs(4)).await;
                continue;
            }
            // (Re)connect: tokens are refreshed every 10 minutes or after any auth error.
            let need = live
                .session
                .lock()
                .unwrap()
                .as_ref()
                .map(|s| s.at.elapsed() > Duration::from_secs(600))
                .unwrap_or(true);
            if need {
                match connect(&local).await {
                    Ok(s) => {
                        *live.session.lock().unwrap() = Some(s);
                        *live.error.lock().unwrap() = None;
                    }
                    Err(e) => {
                        *live.error.lock().unwrap() = Some(e);
                        *live.session.lock().unwrap() = None;
                        set_phase(&app, &live, Phase::NoClient);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                }
            }
            let Some(sess) = live.session.lock().unwrap().clone() else {
                continue;
            };
            sync_owned(&app, &sess).await;
            let pres = local_get(&local, sess.port, &sess.password, "/chat/v4/presences").await;
            let (ph, map_path) = match pres
                .as_ref()
                .ok()
                .and_then(|p| parse_presence(p, &sess.puuid))
            {
                Some((ph, m, _)) => (ph, m),
                None => {
                    if pres.is_err() {
                        *live.session.lock().unwrap() = None;
                    }
                    (Phase::Menus, String::new())
                }
            };
            let prev = *live.phase.lock().unwrap();
            if !map_path.is_empty() {
                *live.map.lock().unwrap() = map_name(&app, &map_path);
            }
            if ph != prev {
                on_leave(&app, prev);
                set_phase(&app, &live, ph);
                match ph {
                    Phase::Pregame => on_pregame(&app),
                    Phase::Ingame => {
                        let gen = live.gen.fetch_add(1, Ordering::SeqCst) + 1;
                        if let Err(e) = on_ingame(&app, &remote, &sess, gen).await {
                            *live.error.lock().unwrap() = Some(e);
                        }
                    }
                    _ => {}
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

/// Agent select: open YOUR Comp Advisor with the map preset. Nothing about other players is requested.
fn on_pregame(app: &AppHandle) {
    let st = app.state::<AppState>();
    if st.prefs()["advisor_in_select"].as_bool().unwrap_or(true) {
        let map = app.state::<Live>().map.lock().unwrap().clone();
        crate::overlay::open_advisor(app, Some(&map), true);
    }
}

fn on_leave(app: &AppHandle, prev: Phase) {
    match prev {
        Phase::Pregame => crate::overlay::close_advisor(app, true),
        Phase::Ingame => {
            let live = app.state::<Live>();
            let id = live.match_id.lock().unwrap().clone();
            let roster = live.roster.lock().unwrap().clone();
            if !id.is_empty() && !roster.is_null() {
                let st = app.state::<AppState>();
                let _ = st.db.lock().unwrap().execute(
                    "INSERT OR REPLACE INTO live_rosters(match_id, saved_at, roster_json) VALUES (?1,?2,?3)",
                    rusqlite::params![id.to_lowercase(), now_ms(), roster.to_string()],
                );
                // The match just ended: look for it every 45 s for a few minutes (plan: scoreboard after the match).
                st.fast_poll_until
                    .store(now_ms() + 8 * 60_000, Ordering::SeqCst);
                st.wake.notify_one();
            }
            *live.roster.lock().unwrap() = Value::Null;
            *live.match_id.lock().unwrap() = String::new();
            live.table_open.store(false, Ordering::SeqCst);
            crate::overlay::layout(app, "normal");
            let _ = app.emit("live", Value::Null);
        }
        _ => {}
    }
}

/// In game: read the roster (free, from Riot's own services), then fetch ranks and recent form from
/// HenrikDev one at a time, ranks first, spread over at least a minute.
async fn on_ingame(
    app: &AppHandle,
    http: &reqwest::Client,
    s: &Session,
    gen: u64,
) -> Result<(), String> {
    let live = app.state::<Live>();
    let st = app.state::<AppState>();
    let player = riot_get(
        http,
        s,
        &format!("{}/core-game/v1/players/{}", glz(s), s.puuid),
    )
    .await
    .map_err(|c| format!("core-game {c}"))?;
    let mid = player["MatchID"].as_str().unwrap_or("").to_string();
    if mid.is_empty() {
        return Err("No match id".into());
    }
    let m = riot_get(http, s, &format!("{}/core-game/v1/matches/{}", glz(s), mid))
        .await
        .map_err(|c| format!("core-game match {c}"))?;
    *live.match_id.lock().unwrap() = mid.clone();
    if let Some(path) = m["MapID"].as_str() {
        let n = map_name(app, path);
        if !n.is_empty() {
            *live.map.lock().unwrap() = n;
        }
    }
    let players = m["Players"].as_array().cloned().unwrap_or_default();
    let my_team = players
        .iter()
        .find(|p| p["Subject"] == s.puuid.as_str())
        .and_then(|p| p["TeamID"].as_str())
        .unwrap_or("")
        .to_string();
    // Names: only for players who didn't hide theirs.
    let visible: Vec<String> = players
        .iter()
        .filter(|p| {
            !p.pointer("/PlayerIdentity/Incognito")
                .and_then(|x| x.as_bool())
                .unwrap_or(false)
        })
        .filter_map(|p| p["Subject"].as_str().map(|x| x.to_string()))
        .collect();
    let names = riot_put(
        http,
        s,
        &format!("{}/name-service/v2/players", pd(s)),
        &json!(visible),
    )
    .await
    .unwrap_or(Value::Null);
    let kb = app.state::<crate::advisor::Kb>();
    let me = st.me();
    let (notes, met) = {
        let c = st.db.lock().unwrap();
        let notes = crate::queries::notes_map(&c);
        let mut met = std::collections::HashMap::new();
        for p in &players {
            let pu = p["Subject"].as_str().unwrap_or("");
            let row: (i64, i64, i64) = c
                .query_row(
                    "SELECT COUNT(*), SUM(mp.team=mine.team), SUM(m.started_at > ?3) FROM match_players mp
                     JOIN match_players mine ON mine.match_id=mp.match_id AND mine.puuid=?2 JOIN matches m ON m.match_id=mp.match_id WHERE mp.puuid=?1",
                    rusqlite::params![pu, me.as_ref().map(|m| m.puuid.as_str()).unwrap_or(""), now_ms() - 18 * 3_600_000],
                    |r| Ok((r.get(0)?, r.get::<_, Option<i64>>(1)?.unwrap_or(0), r.get::<_, Option<i64>>(2)?.unwrap_or(0))),
                )
                .unwrap_or((0, 0, 0));
            met.insert(pu.to_string(), row);
        }
        (notes, met)
    };
    let mut roster: Vec<Value> = players
        .iter()
        .map(|p| {
            let pu = p["Subject"].as_str().unwrap_or("").to_string();
            let hidden = p.pointer("/PlayerIdentity/Incognito").and_then(|x| x.as_bool()).unwrap_or(false);
            let agent_id = p["CharacterID"].as_str().unwrap_or("").to_lowercase();
            let agent = kb.agents.iter().find(|a| a.agent_id.eq_ignore_ascii_case(&agent_id)).map(|a| a.name.clone()).unwrap_or_default();
            let nm = names.as_array().and_then(|a| a.iter().find(|x| x["Subject"] == pu.as_str())).cloned().unwrap_or(Value::Null);
            let hide_level = p.pointer("/PlayerIdentity/HideAccountLevel").and_then(|x| x.as_bool()).unwrap_or(false);
            let (games, with, today) = met.get(&pu).copied().unwrap_or((0, 0, 0));
            json!({
                "puuid": pu, "team": p["TeamID"], "mine": p["TeamID"] == my_team.as_str(), "me": pu == s.puuid,
                "agent": agent, "agent_id": agent_id, "hidden": hidden,
                "name": if hidden { Value::Null } else { nm["GameName"].clone() }, "tag": if hidden { Value::Null } else { nm["TagLine"].clone() },
                "level": if hide_level { Value::Null } else { p.pointer("/PlayerIdentity/AccountLevel").cloned().unwrap_or(Value::Null) },
                "rank": { "state": if hidden { "hidden" } else { "queued" } },
                "form": { "state": if hidden { "hidden" } else { "queued" } },
                "note": notes.get(&pu).cloned(), "met": games, "with": with, "against": games - with, "today": today,
            })
        })
        .collect();
    roster.sort_by_key(|p| (!p["mine"].as_bool().unwrap_or(false)) as i32);
    let ctx = context_line(
        &st,
        me.as_ref(),
        &live.map.lock().unwrap(),
        roster
            .iter()
            .find(|p| p["me"] == true)
            .and_then(|p| p["agent"].as_str())
            .unwrap_or(""),
    );
    let rematch = rematch_line(&st, me.as_ref(), &roster);
    *live.roster.lock().unwrap() = json!({ "match_id": mid, "map": *live.map.lock().unwrap(), "players": roster, "context": ctx, "rematch": rematch, "started": now_ms() });
    emit_roster(app);
    crate::overlay::show_live_compact(app);
    // Fetch ranks, then recent form, one request every 3.5 s.
    let app2 = app.clone();
    let region = me.map(|m| m.region).unwrap_or_else(|| s.region.clone());
    tauri::async_runtime::spawn(async move {
        let fetch_form = app2.state::<AppState>().prefs()["live_form"]
            .as_bool()
            .unwrap_or(true);
        for pass in 0..2 {
            if pass == 1 && !fetch_form {
                break;
            }
            let list: Vec<(String, String, bool)> = {
                let live = app2.state::<Live>();
                let r = live.roster.lock().unwrap();
                r["players"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .map(|p| {
                        (
                            p["puuid"].as_str().unwrap_or("").to_string(),
                            p["agent"].as_str().unwrap_or("").to_string(),
                            p["hidden"].as_bool().unwrap_or(false)
                                || p["me"].as_bool().unwrap_or(false),
                        )
                    })
                    .collect()
            };
            for (pu, agent, skip) in list {
                if app2.state::<Live>().gen.load(Ordering::SeqCst) != gen {
                    return;
                }
                if skip {
                    continue;
                }
                let st = app2.state::<AppState>();
                let field = if pass == 0 { "rank" } else { "form" };
                set_field(&app2, &pu, field, json!({ "state": "loading" }));
                let v = if pass == 0 {
                    crate::players::mmr(&st, &region, &pu, 10 * 60_000, Prio::Foreground).await
                } else {
                    crate::players::form(&st, &region, &pu, &agent, Prio::Foreground).await
                };
                set_field(&app2, &pu, field, v);
                tokio::time::sleep(Duration::from_millis(3500)).await;
            }
        }
        // Your own row: from local data, zero requests.
        let st = app2.state::<AppState>();
        if let Some(me) = st.me() {
            let rk = crate::queries::rank_now(&st.db.lock().unwrap(), &me.puuid);
            let mut v = json!({ "state": "ok" });
            for k in ["tier", "rr", "peak_tier", "peak_season"] {
                v[k] = rk[k].clone();
            }
            set_field(&app2, &me.puuid, "rank", v);
        }
    });
    Ok(())
}

fn set_field(app: &AppHandle, puuid: &str, field: &str, v: Value) {
    let live = app.state::<Live>();
    {
        let mut r = live.roster.lock().unwrap();
        if let Some(ps) = r.get_mut("players").and_then(|p| p.as_array_mut()) {
            if let Some(p) = ps.iter_mut().find(|p| p["puuid"] == puuid) {
                p[field] = v;
            }
        }
    }
    emit_roster(app);
}

fn emit_roster(app: &AppHandle) {
    let r = app.state::<Live>().roster.lock().unwrap().clone();
    let _ = app.emit("live", r);
}

/// "D2 · 64 RR · L2 · Lotus + Omen 12–7"
fn context_line(st: &AppState, me: Option<&crate::state::Me>, map: &str, agent: &str) -> Value {
    let Some(me) = me else { return Value::Null };
    let c = st.db.lock().unwrap();
    let rk = crate::queries::rank_now(&c, &me.puuid);
    let rows = crate::queries::my_rows(
        &c,
        &me.puuid,
        &crate::queries::Filter {
            queue: "competitive".into(),
            limit: 400,
            ..Default::default()
        },
    );
    let mut streak = 0i64;
    let first = rows.first().map(|r| r.result);
    for r in &rows {
        if Some(r.result) == first {
            streak += 1
        } else {
            break;
        }
    }
    let on: Vec<_> = rows
        .iter()
        .filter(|r| r.map.eq_ignore_ascii_case(map) && r.agent.eq_ignore_ascii_case(agent))
        .collect();
    let w = on.iter().filter(|r| r.won()).count();
    json!({ "tier": rk["tier"], "rr": rk["rr"], "streak": streak, "streak_kind": first.map(|c| c.to_string()),
        "map": map, "agent": agent, "wins": w, "losses": on.len() - w })
}

/// "You played 3 of them earlier today (lost 9–13)".
fn rematch_line(st: &AppState, me: Option<&crate::state::Me>, roster: &[Value]) -> Value {
    let Some(me) = me else { return Value::Null };
    let today: Vec<&Value> = roster
        .iter()
        .filter(|p| p["today"].as_i64().unwrap_or(0) > 0 && p["me"] != true)
        .collect();
    if today.is_empty() {
        return Value::Null;
    }
    let c = st.db.lock().unwrap();
    let ids: Vec<String> = today
        .iter()
        .filter_map(|p| p["puuid"].as_str().map(|s| s.to_string()))
        .collect();
    let last = crate::queries::my_rows(
        &c,
        &me.puuid,
        &crate::queries::Filter {
            limit: 15,
            ..Default::default()
        },
    )
    .into_iter()
    .find(|r| {
        c.query_row(
            &format!(
                "SELECT COUNT(*) FROM match_players WHERE match_id=?1 AND puuid IN ({})",
                ids.iter()
                    .map(|x| format!("'{}'", x.replace('\'', "")))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            [&r.id],
            |x| x.get::<_, i64>(0),
        )
        .unwrap_or(0)
            > 0
    });
    json!({ "count": today.len(), "names": today.iter().map(|p| p["name"].as_str().unwrap_or(p["agent"].as_str().unwrap_or(""))).collect::<Vec<_>>(),
        "last": last.map(|r| json!({ "result": r.result.to_string(), "score": [r.my_score, r.their_score] })) })
}

/// Hold-Tab detection: a passive key-state check, only while VALORANT is the focused window and the
/// live module is in game. It never intercepts or changes the key.
#[cfg(windows)]
pub fn start_tab_watch(app: AppHandle) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_TAB};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
    std::thread::spawn(move || loop {
        let live = app.state::<Live>();
        let active = *live.phase.lock().unwrap() == Phase::Ingame
            && app.state::<AppState>().prefs()["live_tab"]
                .as_bool()
                .unwrap_or(true);
        if !active {
            if live.tab_down.swap(false, Ordering::SeqCst) {
                crate::overlay::live_tab(&app, false);
            }
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }
        let focused = unsafe {
            let h = GetForegroundWindow();
            let mut buf = [0u16; 64];
            let n = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32);
            String::from_utf16_lossy(&buf[..n.max(0) as usize]).starts_with("VALORANT")
        };
        let down = focused && unsafe { (GetAsyncKeyState(VK_TAB as i32) as u16 & 0x8000) != 0 };
        if live.tab_down.swap(down, Ordering::SeqCst) != down {
            crate::overlay::live_tab(&app, down);
        }
        std::thread::sleep(Duration::from_millis(60));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn presence(private: Value) -> Value {
        let b = base64::engine::general_purpose::STANDARD.encode(private.to_string());
        json!({ "presences": [
            { "puuid": "other", "product": "valorant", "private": base64::engine::general_purpose::STANDARD.encode(json!({"sessionLoopState": "INGAME"}).to_string()) },
            { "puuid": "me", "product": "league_of_legends", "private": "" },
            { "puuid": "me", "product": "valorant", "private": b }
        ]})
    }

    #[test]
    fn lockfile() {
        assert_eq!(
            parse_lockfile("Riot Client:1234:51234:secret:https"),
            Some((51234, "secret".into()))
        );
        assert_eq!(parse_lockfile("bad"), None);
    }

    #[test]
    fn phases_from_presence_old_and_new_format() {
        let p = presence(
            json!({"sessionLoopState": "PREGAME", "matchMap": "/Game/Maps/Duality/Duality"}),
        );
        let (ph, map, _) = parse_presence(&p, "me").unwrap();
        assert_eq!(ph, Phase::Pregame);
        assert_eq!(map, "/Game/Maps/Duality/Duality");
        let p = presence(
            json!({"matchPresenceData": {"sessionLoopState": "INGAME", "matchMap": "/Game/Maps/Juliett/Juliett"}}),
        );
        assert_eq!(parse_presence(&p, "me").unwrap().0, Phase::Ingame);
        let p = presence(json!({"sessionLoopState": "MENUS"}));
        assert_eq!(parse_presence(&p, "me").unwrap().0, Phase::Menus);
        // Another player's in-game presence never counts as ours.
        assert!(parse_presence(&presence(json!({})), "nobody").is_none());
    }

    #[test]
    fn sessions() {
        let v = json!({ "abc": { "productId": "valorant", "version": "release-13.09-shipping-1-2",
            "launchConfiguration": { "arguments": ["-ares-deployment=ap", "-config-endpoint=https://shared.ap.a.pvp.net"] } } });
        assert_eq!(
            parse_sessions(&v),
            Some((
                "ap".into(),
                "ap".into(),
                "release-13.09-shipping-1-2".into()
            ))
        );
    }

    #[test]
    fn pregame_never_reads_players() {
        // The phase handler for agent select only opens the advisor; it has no session or HTTP client
        // to call with. This test pins that contract: on_pregame takes only the app handle.
        let _f: fn(&AppHandle) = on_pregame;
    }
}
