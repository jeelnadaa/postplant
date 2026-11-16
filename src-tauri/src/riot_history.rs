//! Your match record straight from Riot, through the Riot Client running on this PC (live mode
//! only, same opt-in and tokens as `live.rs`). HenrikDev only holds the matches it has seen, often
//! just the last few dozen; Riot's own match history reaches much further back. For each match id
//! we don't hold yet, Riot's match details are reduced to one stored-match row (totals only) and
//! kept in the `history` table, so lifetime stats count them like HenrikDev record rows.
//!
//! Costs no HenrikDev quota: every request goes to Riot's own `pd.*.a.pvp.net` host, paced to
//! about one a second. Field names follow Riot's undocumented client API and are read defensively.
use crate::db::{self, now_ms};
use crate::live::{self, Session};
use crate::parse::{arr, i, s};
use crate::state::AppState;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Riot pages match history 20 at a time.
const PAGE: i64 = 20;
/// Up to 1,000 match ids per run.
const MAX_PAGES: i64 = 50;
static RUNNING: AtomicBool = AtomicBool::new(false);

fn done_key(puuid: &str) -> String {
    format!("riot_history_complete:{puuid}")
}

/// Match ids not worth fetching again (customs, deathmatch): kept so each run skips them.
fn skip_key(puuid: &str) -> String {
    format!("riot_history_skip:{puuid}")
}

/// Names the conversion needs: map path → name, agent id → name, season id → "e9a3".
#[derive(Default)]
pub struct Names {
    pub maps: HashMap<String, String>,
    pub agents: HashMap<String, String>,
    pub seasons: HashMap<String, String>,
}

impl Names {
    fn load(app: &AppHandle, puuid: &str) -> Names {
        let kb = app.state::<crate::dataset::KbStore>().get();
        let mut n = Names::default();
        for m in &kb.maps {
            if !m.map_url.is_empty() {
                n.maps.insert(m.map_url.to_lowercase(), m.name.clone());
            }
        }
        for a in &kb.agents {
            n.agents.insert(a.agent_id.to_lowercase(), a.name.clone());
        }
        let st = app.state::<AppState>();
        let c = st.db.lock().unwrap();
        if let Some((mmr, _)) = db::cache_get(&c, &format!("mmr:{puuid}"), -1) {
            for x in arr(&mmr, "/seasonal") {
                let (id, short) = (s(x, "/season/id"), s(x, "/season/short"));
                if !id.is_empty() {
                    n.seasons.insert(id.to_lowercase(), short.to_string());
                }
            }
        }
        n
    }
}

/// "/Game/Maps/Ascent/Ascent" → "Ascent" when the knowledge base doesn't know the path.
fn map_fallback(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string()
}

/// Riot match details → one stored-match row (the format `career::stored_entry` and
/// `queries::history_rows` read). None for custom games, deathmatch-style modes without two
/// teams, unfinished matches, or matches you weren't in.
pub fn to_row(m: &Value, puuid: &str, names: &Names) -> Option<Value> {
    let info = m.get("matchInfo")?;
    let id = s(info, "/matchId").to_lowercase();
    let queue = s(info, "/queueID").to_lowercase();
    if id.is_empty() || queue.is_empty() || info["isCompleted"] == json!(false) {
        return None;
    }
    let me = arr(m, "/players")
        .iter()
        .find(|p| s(p, "/subject").eq_ignore_ascii_case(puuid))?;
    let teams = arr(m, "/teams");
    if teams.len() != 2 {
        return None;
    }
    let won = |t: &str| -> i64 {
        teams
            .iter()
            .find(|x| s(x, "/teamId").eq_ignore_ascii_case(t))
            .map(|x| i(x, "/roundsWon"))
            .unwrap_or(0)
    };
    let team = s(me, "/teamId").to_string();
    if !team.eq_ignore_ascii_case("red") && !team.eq_ignore_ascii_case("blue") {
        return None;
    }
    let (mut head, mut body, mut leg, mut made, mut recv) = (0, 0, 0, 0, 0);
    for r in arr(m, "/roundResults") {
        for ps in arr(r, "/playerStats") {
            let mine = s(ps, "/subject").eq_ignore_ascii_case(puuid);
            for d in arr(ps, "/damage") {
                if mine {
                    head += i(d, "/headshots");
                    body += i(d, "/bodyshots");
                    leg += i(d, "/legshots");
                    made += i(d, "/damage");
                } else if s(d, "/receiver").eq_ignore_ascii_case(puuid) {
                    recv += i(d, "/damage");
                }
            }
        }
    }
    let agent_id = s(me, "/characterId").to_lowercase();
    let map_path = s(info, "/mapId");
    let started = i(info, "/gameStartMillis");
    let started_iso = chrono::DateTime::from_timestamp_millis(started)
        .map(|d| d.to_rfc3339())
        .unwrap_or_default();
    Some(json!({
        "source": "riot",
        "meta": {
            "id": id,
            "map": { "name": names.maps.get(&map_path.to_lowercase()).cloned().unwrap_or_else(|| map_fallback(map_path)) },
            "mode": queue,
            "started_at": started_iso,
            "length_ms": i(info, "/gameLengthMillis"),
            "version": s(info, "/gameVersion"),
            "season": { "id": s(info, "/seasonId"), "short": names.seasons.get(&s(info, "/seasonId").to_lowercase()).cloned().unwrap_or_default() },
        },
        "stats": {
            "team": team,
            "character": { "id": agent_id, "name": names.agents.get(&agent_id).cloned().unwrap_or_default() },
            "tier": i(me, "/competitiveTier"),
            "kills": i(me, "/stats/kills"), "deaths": i(me, "/stats/deaths"), "assists": i(me, "/stats/assists"),
            "score": i(me, "/stats/score"),
            "shots": { "head": head, "body": body, "leg": leg },
            "damage": { "made": made, "received": recv },
        },
        "teams": { "red": won("Red"), "blue": won("Blue") },
    }))
}

pub fn status(c: &rusqlite::Connection, puuid: &str) -> Value {
    let n: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM history WHERE puuid=?1 AND json_extract(row_json,'$.source')='riot'",
            [puuid],
            |r| r.get(0),
        )
        .unwrap_or(0);
    json!({ "count": n, "complete": db::get_setting(c, &done_key(puuid)).is_some(),
        "running": RUNNING.load(Ordering::SeqCst) })
}

/// Download what Riot's match history lists that we don't hold. `full`: keep paging past pages
/// with nothing new (first run); otherwise stop at the first page we already hold.
pub async fn import(app: &AppHandle, full: bool) -> Result<Value, String> {
    let st = app.state::<AppState>();
    let me = st.me().ok_or("Set your Riot ID first.")?;
    if !live::enabled(&st) {
        return Err("Turn on live mode in Settings first: Riot's match history is read through the Riot Client.".into());
    }
    if RUNNING.swap(true, Ordering::SeqCst) {
        return Ok(json!({ "busy": true }));
    }
    let r = run(app, &me, full).await;
    RUNNING.store(false, Ordering::SeqCst);
    let s = status(&st.db.lock().unwrap(), &me.puuid);
    let _ = app.emit("history-progress", json!({ "done": true, "riot": s }));
    r
}

/// GET from Riot, waiting out a few 429s.
async fn get(http: &reqwest::Client, sess: &Session, url: &str) -> Result<Value, u16> {
    let mut waits = 0;
    loop {
        match live::riot_get(http, sess, url).await {
            Err(429) if waits < 6 => {
                waits += 1;
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
            r => return r,
        }
    }
}

/// Fetch one match's details and keep it as a record row. Ok(true) = stored (or already held),
/// Ok(false) = not a usable match (custom, deathmatch…), Err = Riot refused.
async fn fetch_one(
    app: &AppHandle,
    http: &reqwest::Client,
    sess: &Session,
    names: &Names,
    puuid: &str,
    id: &str,
) -> Result<(bool, usize), u16> {
    let url = format!("{}/match-details/v1/matches/{id}", live::pd(sess));
    let m = get(http, sess, &url).await?;
    let st = app.state::<AppState>();
    Ok(match to_row(&m, puuid, names) {
        Some(row) => (
            true,
            crate::history::store_rows(&st.db.lock().unwrap(), puuid, &[row]),
        ),
        None => (false, 0),
    })
}

/// Riot's per-mode, per-act totals (games and wins for competitive, unrated, swiftplay…), kept
/// for the Lifetime tab. One request.
async fn fetch_mode_totals(app: &AppHandle, http: &reqwest::Client, sess: &Session, puuid: &str) {
    let url = format!("{}/mmr/v1/players/{}", live::pd(sess), sess.puuid);
    if let Ok(v) = get(http, sess, &url).await {
        if v.get("QueueSkills").is_some() {
            let st = app.state::<AppState>();
            let _ = db::cache_put(&st.db.lock().unwrap(), &format!("riot_mmr:{puuid}"), &v);
        }
    }
}

/// One page of Riot's competitive updates: (match ids, rows stored in rr_changes).
pub fn store_comp_updates(c: &rusqlite::Connection, puuid: &str, v: &Value) -> Vec<String> {
    let mut ids = vec![];
    for u in arr(v, "/Matches") {
        let id = s(u, "/MatchID").to_lowercase();
        if id.is_empty() {
            continue;
        }
        let _ = c.execute(
            "INSERT OR IGNORE INTO rr_changes(match_id, puuid, rr_change, tier_after, rr_after) VALUES (?1,?2,?3,?4,?5)",
            rusqlite::params![id, puuid, i(u, "/RankedRatingEarned"), i(u, "/TierAfterUpdate"), i(u, "/RankedRatingAfterUpdate")],
        );
        ids.push(id);
    }
    ids
}

async fn run(app: &AppHandle, me: &crate::state::Me, full: bool) -> Result<Value, String> {
    let sess: Session = live::connect(&live::local_http())
        .await
        .map_err(|e| format!("{e}. Start VALORANT, then try again."))?;
    if !sess.puuid.eq_ignore_ascii_case(&me.puuid) {
        return Err("The Riot Client is signed in to a different account.".into());
    }
    let http = live::remote_http();
    let names = Names::load(app, &me.puuid);
    let st = app.state::<AppState>();
    fetch_mode_totals(app, &http, &sess, &me.puuid).await;
    let mut skip: HashSet<String> = db::get_json(&st.db.lock().unwrap(), &skip_key(&me.puuid))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let held = |id: &str| -> bool {
        let c = st.db.lock().unwrap();
        c.query_row(
            "SELECT (SELECT COUNT(*) FROM matches WHERE match_id=?1) + (SELECT COUNT(*) FROM history WHERE match_id=?1)",
            [id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
            > 0
    };
    let pace = || tokio::time::sleep(Duration::from_millis(1100));
    let (mut added, mut skipped, mut total) = (0usize, 0usize, 0i64);
    let mut reached_end = false;
    let mut seen: HashSet<String> = HashSet::new();
    // Two lists of match ids: the general match history, then the competitive updates (every
    // ranked match with its RR change), which can list ranked matches the first one no longer does.
    for (pass, source) in ["history", "competitive"].into_iter().enumerate() {
        let mut page = 0i64;
        let mut pass_end = false;
        while page < MAX_PAGES {
            let (from, to) = (page * PAGE, page * PAGE + PAGE);
            let url = if source == "history" {
                format!(
                    "{}/match-history/v1/history/{}?startIndex={from}&endIndex={to}",
                    live::pd(&sess),
                    sess.puuid
                )
            } else {
                format!("{}/mmr/v1/players/{}/competitiveupdates?startIndex={from}&endIndex={to}&queue=competitive", live::pd(&sess), sess.puuid)
            };
            let v = match get(&http, &sess, &url).await {
                Ok(v) => v,
                Err(code) if page == 0 && pass == 0 => {
                    return Err(format!("Riot's match history answered {code}."))
                }
                Err(code) => {
                    eprintln!("riot history: {source} stopped at page {page}: {code}");
                    break;
                }
            };
            let listed: Vec<String> = if source == "history" {
                total = total.max(i(&v, "/Total"));
                arr(&v, "/History")
                    .iter()
                    .map(|h| s(h, "/MatchID").to_lowercase())
                    .collect()
            } else {
                store_comp_updates(&st.db.lock().unwrap(), &me.puuid, &v)
            };
            let n_listed = listed.len() as i64;
            let ids: Vec<String> = listed
                .into_iter()
                .filter(|x| !x.is_empty() && seen.insert(x.clone()))
                .collect();
            let mut new_here = 0;
            for id in &ids {
                if skip.contains(id) || held(id) {
                    continue;
                }
                new_here += 1;
                pace().await;
                match fetch_one(app, &http, &sess, &names, &me.puuid, id).await {
                    Ok((true, n)) => added += n,
                    Ok((false, _)) => {
                        skipped += 1;
                        skip.insert(id.clone());
                    }
                    Err(code) => {
                        skipped += 1;
                        // Riot no longer serves details this old: don't ask again.
                        if code == 404 {
                            skip.insert(id.clone());
                        }
                        eprintln!("riot history: match {id}: {code}");
                    }
                }
                let _ = app.emit(
                    "history-progress",
                    json!({ "source": "riot", "page": page + 1, "added": added, "total": total }),
                );
            }
            page += 1;
            if n_listed < PAGE || (source == "history" && total > 0 && page * PAGE >= total) {
                pass_end = true;
                break;
            }
            if !full && new_here == 0 {
                break;
            }
            pace().await;
        }
        if pass == 0 {
            reached_end = pass_end;
        }
    }
    {
        let c = st.db.lock().unwrap();
        let _ = db::set_setting(&c, &skip_key(&me.puuid), &json!(skip).to_string());
    }
    if reached_end {
        let c = st.db.lock().unwrap();
        let _ = db::set_setting(&c, &done_key(&me.puuid), &now_ms().to_string());
    }
    Ok(json!({ "added": added, "skipped": skipped, "total": total }))
}

/// While VALORANT runs with live mode on: once per app run, in the menus, top up from Riot
/// (a full pass the first time, afterwards until the first page we already hold).
#[cfg_attr(not(feature = "live"), allow(dead_code))]
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(90)).await;
        loop {
            let st = app.state::<AppState>();
            let phase = *app.state::<live::Live>().phase.lock().unwrap();
            let ready = st.me().is_some()
                && live::enabled(&st)
                && st.game_running.load(Ordering::SeqCst)
                && phase == live::Phase::Menus;
            if ready {
                let complete = st
                    .me()
                    .map(|m| db::get_setting(&st.db.lock().unwrap(), &done_key(&m.puuid)).is_some())
                    .unwrap_or(true);
                if import(&app, !complete).await.is_ok() {
                    // Done for this app run; new matches arrive through the normal sync.
                    return;
                }
                tokio::time::sleep(Duration::from_secs(15 * 60)).await;
            } else {
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details() -> Value {
        json!({
            "matchInfo": { "matchId": "ABC-1", "mapId": "/Game/Maps/Ascent/Ascent", "gameStartMillis": 1767225600000i64,
                "gameLengthMillis": 2100000, "queueID": "competitive", "seasonId": "S1", "isCompleted": true, "gameVersion": "release-09.10" },
            "players": [
                { "subject": "ME", "teamId": "Blue", "characterId": "AG1", "competitiveTier": 18,
                  "stats": { "score": 5200, "kills": 22, "deaths": 15, "assists": 6 } },
                { "subject": "OTHER", "teamId": "Red", "characterId": "AG2", "stats": { "score": 3000, "kills": 10, "deaths": 18, "assists": 3 } }
            ],
            "teams": [ { "teamId": "Red", "roundsWon": 9 }, { "teamId": "Blue", "roundsWon": 13 } ],
            "roundResults": [
                { "playerStats": [
                    { "subject": "ME", "damage": [ { "receiver": "OTHER", "damage": 150, "headshots": 1, "bodyshots": 2, "legshots": 0 } ] },
                    { "subject": "OTHER", "damage": [ { "receiver": "ME", "damage": 80, "headshots": 0, "bodyshots": 3, "legshots": 1 } ] }
                ] }
            ]
        })
    }

    #[test]
    fn riot_details_become_a_record_row() {
        let mut n = Names::default();
        n.maps
            .insert("/game/maps/ascent/ascent".into(), "Ascent".into());
        n.agents.insert("ag1".into(), "Omen".into());
        n.seasons.insert("s1".into(), "e10a1".into());
        let r = to_row(&details(), "me", &n).unwrap();
        assert_eq!(r["meta"]["id"], "abc-1");
        assert_eq!(r["meta"]["map"]["name"], "Ascent");
        assert_eq!(r["meta"]["season"]["short"], "e10a1");
        assert_eq!(r["stats"]["character"]["name"], "Omen");
        assert_eq!(r["stats"]["shots"]["head"], 1);
        assert_eq!(r["stats"]["damage"]["received"], 80);
        assert_eq!(r["teams"]["blue"], 13);

        // Counted by lifetime stats like a HenrikDev record row.
        let v = crate::career::compute_with(
            &[],
            &[r],
            "me",
            &crate::career::Filt {
                queue: "competitive".into(),
                ..Default::default()
            },
        );
        assert_eq!(v["overall"]["games"], 1);
        assert_eq!(v["overall"]["win"], 100.0);
        assert_eq!(v["overall"]["acs"], 236.0);
    }

    #[test]
    fn skips_customs_and_unknown_map_falls_back() {
        let mut m = details();
        m["matchInfo"]["queueID"] = json!("");
        assert!(to_row(&m, "me", &Names::default()).is_none());
        let r = to_row(&details(), "me", &Names::default()).unwrap();
        assert_eq!(r["meta"]["map"]["name"], "Ascent");
        assert!(to_row(&details(), "nobody", &Names::default()).is_none());
    }

    #[test]
    fn comp_updates_give_ids_and_rr() {
        let c = db::open_memory().unwrap();
        let v = json!({ "Matches": [
            { "MatchID": "M-1", "RankedRatingEarned": 18, "TierAfterUpdate": 15, "RankedRatingAfterUpdate": 40 },
            { "MatchID": "" }
        ] });
        assert_eq!(store_comp_updates(&c, "me", &v), vec!["m-1".to_string()]);
        let rr: i64 = c
            .query_row(
                "SELECT rr_change FROM rr_changes WHERE match_id='m-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rr, 18);
    }

    #[test]
    fn mode_totals_sum_every_act() {
        let c = db::open_memory().unwrap();
        let riot = json!({ "QueueSkills": {
            "competitive": { "SeasonalInfoBySeasonID": {
                "S1": { "SeasonID": "S1", "NumberOfGames": 50, "NumberOfWins": 24, "NumberOfWinsWithPlacements": 26 },
                "S2": { "SeasonID": "S2", "NumberOfGames": 10, "NumberOfWins": 6, "NumberOfWinsWithPlacements": 6 } } },
            "unrated": { "SeasonalInfoBySeasonID": { "S1": { "SeasonID": "S1", "NumberOfGames": 20, "NumberOfWins": 0, "NumberOfWinsWithPlacements": 11 } } },
            "deathmatch": { "SeasonalInfoBySeasonID": null }
        } });
        db::cache_put(&c, "riot_mmr:me", &riot).unwrap();
        let mmr = json!({ "seasonal": [ { "season": { "id": "s1", "short": "e9a1" } } ] });
        let m = crate::queries::mode_totals(&c, "me", &mmr);
        let list = m["list"].as_array().unwrap();
        assert_eq!(list.len(), 2, "modes with no games are left out");
        assert_eq!(list[0]["name"], "competitive");
        assert_eq!(list[0]["games"], 60);
        assert_eq!(list[0]["wins"], 32);
        assert_eq!(list[1]["wins"], 11);
    }
}
