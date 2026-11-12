//! Your match record from HenrikDev's stored matches: one row of totals per match (agent, map,
//! mode, K/D/A, score, shots, damage, rounds won). It reaches much further back than the matches
//! held in full on this PC, so lifetime stats use it for the matches we don't have in full.
use crate::api::{paths, ApiError, Prio};
use crate::db::{self, now_ms};
use crate::parse::{arr, i, s};
use crate::state::AppState;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const PAGE: u32 = 25;
/// 80 pages of 25: up to 2,000 matches, at most 80 requests for a first full download.
pub const MAX_PAGES: u32 = 80;
static RUNNING: AtomicBool = AtomicBool::new(false);

fn done_key(puuid: &str) -> String {
    format!("history_complete:{puuid}")
}

/// Store rows (stored-match format). Returns how many were new.
pub fn store_rows(c: &Connection, puuid: &str, rows: &[Value]) -> usize {
    let mut n = 0;
    for r in rows {
        let id = s(r, "/meta/id");
        if id.is_empty() {
            continue;
        }
        let t = crate::parse::time_ms(r.pointer("/meta/started_at").unwrap_or(&Value::Null));
        n += c
            .execute(
                "INSERT OR IGNORE INTO history(match_id, puuid, started_at, row_json) VALUES (?1,?2,?3,?4)",
                params![id, puuid, t, r.to_string()],
            )
            .unwrap_or(0);
    }
    n
}

pub fn rows(c: &Connection, puuid: &str) -> Vec<Value> {
    c.prepare("SELECT row_json FROM history WHERE puuid=?1")
        .and_then(|mut q| {
            q.query_map([puuid], |r| r.get::<_, String>(0)).map(|it| {
                it.flatten()
                    .filter_map(|j| serde_json::from_str(&j).ok())
                    .collect()
            })
        })
        .unwrap_or_default()
}

pub fn status(c: &Connection, puuid: &str) -> Value {
    let (n, first, last): (i64, i64, i64) = c
        .query_row(
            "SELECT COUNT(*), COALESCE(MIN(started_at),0), COALESCE(MAX(started_at),0) FROM history WHERE puuid=?1",
            [puuid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap_or((0, 0, 0));
    json!({ "count": n, "first": first, "last": last,
        "complete": db::get_setting(c, &done_key(puuid)).is_some(),
        "running": RUNNING.load(Ordering::SeqCst) })
}

/// Page through my stored matches, newest first. `full`: keep going past pages we already hold
/// (first download); otherwise stop at the first page with nothing new. Background requests wait
/// for budget instead of failing, so this never crowds out match sync.
pub async fn backfill(app: &AppHandle, prio: Prio, full: bool) -> Result<Value, String> {
    let st = app.state::<AppState>();
    let me = st.me().ok_or("Set your Riot ID first.")?;
    if RUNNING.swap(true, Ordering::SeqCst) {
        return Ok(json!({ "busy": true }));
    }
    let r = run(app, &st, &me, prio, full).await;
    RUNNING.store(false, Ordering::SeqCst);
    let s = status(&st.db.lock().unwrap(), &me.puuid);
    let _ = app.emit("history-progress", json!({ "done": true, "status": s }));
    r
}

async fn run(
    app: &AppHandle,
    st: &AppState,
    me: &crate::state::Me,
    prio: Prio,
    full: bool,
) -> Result<Value, String> {
    let (mut page, mut added, mut waits) = (1u32, 0usize, 0u32);
    let mut reached_end = false;
    while page <= MAX_PAGES {
        let path = paths::stored_matches_page(&me.region, &me.puuid, PAGE, page);
        let v = match st.api.get(&path, prio).await {
            Ok(v) => v,
            Err(ApiError::Budget(_)) | Err(ApiError::Status(429, _)) if waits < 40 => {
                waits += 1;
                tokio::time::sleep(Duration::from_secs(20)).await;
                continue;
            }
            Err(ApiError::Status(404, _)) => {
                reached_end = true;
                break;
            }
            Err(e) if added > 0 || page > 1 => {
                eprintln!("history: stopped at page {page}: {e}");
                break;
            }
            Err(e) => return Err(e.to_string()),
        };
        let data = arr(&v, "/data");
        let new = store_rows(&st.db.lock().unwrap(), &me.puuid, data);
        added += new;
        let _ = app.emit(
            "history-progress",
            json!({ "page": page, "added": added, "total": i(&v, "/results/total") }),
        );
        if (data.len() as u32) < PAGE {
            reached_end = true;
            break;
        }
        if !full && new == 0 {
            break;
        }
        page += 1;
    }
    if reached_end {
        let c = st.db.lock().unwrap();
        let _ = db::set_setting(&c, &done_key(&me.puuid), &now_ms().to_string());
    }
    Ok(json!({ "added": added, "pages": page.min(MAX_PAGES) }))
}

/// First run: download the whole record in the background a minute after start; afterwards a
/// one-page top-up every 6 hours (matches older than what we hold in full fill in over time).
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(60)).await;
        loop {
            let st = app.state::<AppState>();
            let ready = st.me().is_some() && st.api.has_key();
            let game = st.game_running.load(Ordering::SeqCst);
            if ready && !game {
                let complete = st
                    .me()
                    .map(|m| db::get_setting(&st.db.lock().unwrap(), &done_key(&m.puuid)).is_some())
                    .unwrap_or(true);
                let _ = backfill(&app, Prio::Background, !complete).await;
            }
            tokio::time::sleep(Duration::from_secs(if ready && !game {
                6 * 3600
            } else {
                300
            }))
            .await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, t: &str) -> Value {
        json!({ "meta": { "id": id, "map": { "name": "Lotus" }, "mode": "Competitive", "started_at": t, "season": { "short": "e9a3" } },
            "stats": { "team": "Red", "character": { "id": "a1", "name": "Omen" }, "kills": 20, "deaths": 10, "assists": 5, "score": 4800,
                "shots": { "head": 10, "body": 30, "leg": 0 }, "damage": { "made": 3000, "received": 2000 } },
            "teams": { "red": 13, "blue": 11 } })
    }

    #[test]
    fn rows_dedupe_and_merge_into_career() {
        let c = db::open_memory().unwrap();
        let rs = vec![
            row("h1", "2026-01-01T10:00:00Z"),
            row("h2", "2026-01-02T10:00:00Z"),
        ];
        assert_eq!(store_rows(&c, "p0", &rs), 2);
        assert_eq!(store_rows(&c, "p0", &rs), 0, "already held");
        let held = rows(&c, "p0");
        assert_eq!(held.len(), 2);
        let st = status(&c, "p0");
        assert_eq!(st["count"], 2);
        // A full match with the same id as a history row is counted once (the full one wins).
        let full = crate::parse::fixture::match_v4("h1", true, "2026-01-01T10:00:00Z");
        let v = crate::career::compute_with(&[full], &held, "p0", &crate::career::Filt::default());
        assert_eq!(v["overall"]["games"], 2);
        assert_eq!(v["sources"]["full"], 1);
        assert_eq!(v["sources"]["history"], 1);
        assert_eq!(v["overall"]["detail_games"], 1);
        let f = crate::career::Filt {
            season: "e9a3".into(),
            queue: "competitive".into(),
            ..Default::default()
        };
        let v = crate::career::compute_with(&[], &held, "p0", &f);
        assert_eq!(v["overall"]["games"], 2);
        assert_eq!(v["overall"]["kd"], 2.0);
        assert_eq!(v["overall"]["win"], 100.0);
        assert!(v["overall"]["kast"].is_null(), "no round data, no KAST");
    }
}
