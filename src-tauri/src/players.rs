//! Other players' rank, recent form and career stats, shared by match detail, player lookup and the
//! live panel. Everything is cached so the same player never costs twice in a short time.
use crate::api::{paths, ApiError, Prio};
use crate::career;
use crate::db::{self, now_ms};
use crate::parse::{arr, i, s, time_ms, unwrap_data};
use crate::state::AppState;
use rusqlite::{params, Connection};
use serde_json::{json, Value};

/// Rank, peak and last act for a PUUID. `max_age_ms` reuses a cached answer younger than that.
/// Returns { state: ok | private | unranked | error, tier, rr, peak_tier, peak_season, last_act_tier, last_act }.
pub async fn mmr(st: &AppState, region: &str, puuid: &str, max_age_ms: i64, prio: Prio) -> Value {
    let key = format!("mmr:{puuid}");
    let cached = db::cache_get(&st.db.lock().unwrap(), &key, max_age_ms);
    if let Some((v, at)) = cached {
        let mut r = summarize_mmr(&v);
        r["at"] = json!(at);
        return r;
    }
    let private = db::cache_get(
        &st.db.lock().unwrap(),
        &format!("mmr-private:{puuid}"),
        max_age_ms.max(30 * 60_000),
    );
    if let Some((_, at)) = private {
        return json!({ "state": "private", "at": at });
    }
    match st.api.get(&paths::mmr(region, puuid), prio).await {
        Ok(v) => {
            let d = unwrap_data(&v).clone();
            let _ = db::cache_put(&st.db.lock().unwrap(), &key, &d);
            let mut r = summarize_mmr(&d);
            r["at"] = json!(now_ms());
            r
        }
        Err(ApiError::Status(code, _)) if code == 404 || code == 403 => {
            let _ = db::cache_put(
                &st.db.lock().unwrap(),
                &format!("mmr-private:{puuid}"),
                &json!(true),
            );
            json!({ "state": "private", "at": now_ms() })
        }
        Err(e) => json!({ "state": "error", "error": e.to_string() }),
    }
}

pub fn summarize_mmr(d: &Value) -> Value {
    let tier = i(d, "/current/tier/id");
    let seasons = arr(d, "/seasonal");
    // Last finished act: the newest seasonal entry that isn't the current one, if any.
    let last = seasons
        .iter()
        .rev()
        .find(|x| i(x, "/end_tier/id") > 0 && seasons.len() > 1)
        .or_else(|| seasons.last());
    json!({
        "state": if tier > 2 { "ok" } else { "unranked" },
        "tier": tier, "rr": i(d, "/current/rr"), "elo": i(d, "/current/elo"), "last_change": i(d, "/current/last_change"),
        "peak_tier": i(d, "/peak/tier/id"), "peak_season": s(d, "/peak/season/short"),
        "last_act_tier": last.map(|x| i(x, "/end_tier/id")).unwrap_or(0), "last_act": last.map(|x| s(x, "/season/short").to_string()).unwrap_or_default(),
    })
}

/// Recent competitive form from HenrikDev's stored matches (1 request): win %, K/D, HS %, ACS,
/// most played agents and the record on `agent` (experience on the current agent).
pub async fn form(st: &AppState, region: &str, puuid: &str, agent: &str, prio: Prio) -> Value {
    let key = format!("form:{puuid}");
    let cached = db::cache_get(&st.db.lock().unwrap(), &key, 10 * 60_000);
    let rows: Value = match cached {
        Some((v, _)) => v,
        None => match st
            .api
            .get(
                &paths::stored_matches(region, puuid, 10, Some("competitive")),
                prio,
            )
            .await
        {
            Ok(v) => {
                let d = json!(arr(&v, "/data"));
                let _ = db::cache_put(&st.db.lock().unwrap(), &key, &d);
                d
            }
            Err(ApiError::Status(code, _)) if code == 404 || code == 403 => {
                return json!({ "state": "private" })
            }
            Err(e) => return json!({ "state": "error", "error": e.to_string() }),
        },
    };
    let list = rows.as_array().cloned().unwrap_or_default();
    let sum = career::from_stored(&list);
    let o = &sum["overall"];
    let on: Vec<&Value> = list
        .iter()
        .filter(|m| s(m, "/stats/character/name").eq_ignore_ascii_case(agent))
        .collect();
    let on_wins = on
        .iter()
        .filter(|m| {
            let team = s(m, "/stats/team").to_lowercase();
            let (a, b) = if team == "red" {
                (i(m, "/teams/red"), i(m, "/teams/blue"))
            } else {
                (i(m, "/teams/blue"), i(m, "/teams/red"))
            };
            a > b
        })
        .count();
    json!({
        "state": if list.is_empty() { "empty" } else { "ok" },
        "games": o["games"], "win": o["win"], "kd": o["kd"], "hs": o["hs"], "acs": o["acs"],
        "agents": sum["agents"].as_array().map(|a| a.iter().take(3).map(|x| json!({ "name": x["name"], "id": x["id"], "games": x["games"] })).collect::<Vec<_>>()).unwrap_or_default(),
        "on_agent": { "games": on.len(), "wins": on_wins },
    })
}

/// Every full match we hold that includes `puuid`: your own stored matches plus matches fetched for
/// lookups (`ext_matches`).
pub fn raw_matches_with(c: &Connection, puuid: &str) -> Vec<Value> {
    let mut out = vec![];
    let ids: Vec<String> = c
        .prepare("SELECT match_id FROM match_players WHERE puuid=?1")
        .and_then(|mut q| {
            q.query_map([puuid], |r| r.get(0))
                .map(|x| x.flatten().collect())
        })
        .unwrap_or_default();
    for id in &ids {
        if let Ok(v) = crate::parse::load_raw(c, id) {
            out.push(v);
        }
    }
    let ext: Vec<Vec<u8>> = c
        .prepare("SELECT e.raw_json_zstd FROM ext_matches e JOIN ext_match_players p ON p.match_id=e.match_id WHERE p.puuid=?1")
        .and_then(|mut q| q.query_map([puuid], |r| r.get(0)).map(|x| x.flatten().collect()))
        .unwrap_or_default();
    for blob in ext {
        if let Ok(bytes) = zstd::decode_all(blob.as_slice()) {
            if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
                let id = s(&v, "/metadata/match_id").to_string();
                if !ids.contains(&id) {
                    out.push(v);
                }
            }
        }
    }
    out
}

fn have_match(c: &Connection, id: &str) -> bool {
    c.query_row(
        "SELECT 1 FROM matches WHERE match_id=?1 UNION SELECT 1 FROM ext_matches WHERE match_id=?1",
        [id],
        |_| Ok(()),
    )
    .is_ok()
}

pub fn store_ext(c: &Connection, raw: &Value) {
    let m = unwrap_data(raw);
    let id = s(m, "/metadata/match_id");
    if id.is_empty() {
        return;
    }
    let Ok(z) = zstd::encode_all(m.to_string().as_bytes(), 9) else {
        return;
    };
    let started = time_ms(m.pointer("/metadata/started_at").unwrap_or(&Value::Null));
    let _ = c.execute(
        "INSERT OR REPLACE INTO ext_matches(match_id, started_at, raw_json_zstd, fetched_at) VALUES (?1,?2,?3,?4)",
        params![id, started, z, now_ms()],
    );
    for p in arr(m, "/players") {
        let _ = c.execute(
            "INSERT OR IGNORE INTO ext_match_players(match_id, puuid) VALUES (?1,?2)",
            params![id, s(p, "/puuid")],
        );
    }
}

/// Fetch up to `n` of a player's recent matches in full (1 request each, skipping any we already
/// hold), then return their career over everything we have. Matches are kept, so asking again is free.
pub async fn load_career(
    st: &AppState,
    region: &str,
    puuid: &str,
    n: u32,
    queue: &str,
) -> Result<Value, String> {
    let mode = if queue.is_empty() { None } else { Some(queue) };
    let list = st
        .api
        .get(
            &paths::stored_matches(region, puuid, n.clamp(1, 25), mode),
            Prio::Foreground,
        )
        .await
        .map_err(|e| e.to_string())?;
    let ids: Vec<String> = arr(&list, "/data")
        .iter()
        .map(|m| s(m, "/meta/id").to_string())
        .filter(|x| !x.is_empty())
        .collect();
    let mut fetched = 0;
    let mut failed = None;
    for id in ids.iter().take(n as usize) {
        let have = have_match(&st.db.lock().unwrap(), id);
        if have {
            continue;
        }
        match st
            .api
            .get(&paths::match_v4(region, id), Prio::Foreground)
            .await
        {
            Ok(v) => {
                store_ext(&st.db.lock().unwrap(), &v);
                fetched += 1;
            }
            Err(e) => {
                failed = Some(e.to_string());
                break;
            }
        }
    }
    let c = st.db.lock().unwrap();
    let raws = raw_matches_with(&c, puuid);
    let mut v = career::compute(&raws, puuid, queue);
    v["fetched"] = json!(fetched);
    v["held"] = json!(raws.len());
    v["error"] = json!(failed);
    Ok(v)
}

/// Career from what we already hold (zero requests).
pub fn career_local(c: &Connection, puuid: &str, queue: &str) -> Value {
    career_lifetime(
        c,
        puuid,
        &career::Filt {
            queue: queue.into(),
            ..Default::default()
        },
        false,
    )
}

/// Career from the matches held in full plus, with `with_history`, every match in my HenrikDev
/// record that isn't held in full (totals only).
pub fn career_lifetime(c: &Connection, puuid: &str, f: &career::Filt, with_history: bool) -> Value {
    let raws = raw_matches_with(c, puuid);
    let hist = if with_history {
        crate::history::rows(c, puuid)
    } else {
        vec![]
    };
    let mut v = career::compute_with(&raws, &hist, puuid, f);
    v["held"] = json!(raws.len());
    v
}
