//! Read models for every screen. All of these read SQLite only: zero API quota (plan section 12).
use crate::parse::{self, arr, f, i, s};
use crate::state::Me;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::collections::HashMap;

const SESSION_GAP_MS: i64 = 45 * 60_000;

#[derive(Clone, Debug)]
pub struct Row {
    pub id: String,
    pub started: i64,
    pub length: i64,
    pub map: String,
    pub queue: String,
    pub team: String,
    pub agent: String,
    pub agent_id: String,
    pub my_score: i64,
    pub their_score: i64,
    pub result: char, // W L D
    pub k: i64,
    pub d: i64,
    pub a: i64,
    pub score: i64,
    pub rounds: i64,
    pub hs: i64,
    pub body: i64,
    pub leg: i64,
    pub dmg: i64,
    pub rr: Option<i64>,
    pub party: String,
    pub tier: i64,
}

impl Row {
    pub fn acs(&self) -> f64 {
        if self.rounds > 0 {
            self.score as f64 / self.rounds as f64
        } else {
            0.0
        }
    }
    pub fn hs_pct(&self) -> f64 {
        let t = self.hs + self.body + self.leg;
        if t > 0 {
            100.0 * self.hs as f64 / t as f64
        } else {
            0.0
        }
    }
    pub fn won(&self) -> bool {
        self.result == 'W'
    }
    pub fn json(&self) -> Value {
        json!({
            "id": self.id, "started_at": self.started, "length_ms": self.length, "map": self.map, "queue": self.queue,
            "agent": self.agent, "agent_id": self.agent_id, "result": self.result.to_string(), "score": [self.my_score, self.their_score],
            "k": self.k, "d": self.d, "a": self.a, "acs": self.acs().round(), "hs": self.hs_pct().round(),
            "adr": if self.rounds > 0 { (self.dmg as f64 / self.rounds as f64).round() } else { 0.0 },
            "rr": self.rr, "tier": self.tier, "team": self.team
        })
    }
}

#[derive(Default, Clone)]
pub struct Filter {
    pub queue: String,
    pub map: String,
    pub agent: String,
    pub since: i64,
    pub limit: i64,
}

pub fn my_rows(c: &Connection, puuid: &str, fl: &Filter) -> Vec<Row> {
    let mut st = match c.prepare(
        "SELECT m.match_id, m.started_at, COALESCE(m.length_ms,0), m.map_name, m.queue, m.red_score, m.blue_score, m.winning_team,
                m.rounds_played, mp.team, mp.agent_name, mp.agent_id, mp.kills, mp.deaths, mp.assists, mp.score, mp.headshots,
                mp.bodyshots, mp.legshots, mp.damage_made, rc.rr_change, COALESCE(mp.party_id,''), COALESCE(mp.rank_tier,0)
         FROM matches m JOIN match_players mp ON mp.match_id=m.match_id AND mp.puuid=?1
         LEFT JOIN rr_changes rc ON rc.match_id=m.match_id
         WHERE (?2='' OR m.queue=?2) AND (?3='' OR m.map_name=?3) AND (?4='' OR mp.agent_name=?4) AND m.started_at>=?5
         ORDER BY m.started_at DESC LIMIT ?6",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let limit = if fl.limit <= 0 { 100_000 } else { fl.limit };
    st.query_map(
        params![puuid, fl.queue, fl.map, fl.agent, fl.since, limit],
        |r| {
            let team: String = r.get(9)?;
            let (red, blue): (i64, i64) = (r.get(5)?, r.get(6)?);
            let winner: String = r.get(7)?;
            let is_red = team.eq_ignore_ascii_case("red");
            let (mine, theirs) = if is_red { (red, blue) } else { (blue, red) };
            let result = if winner.is_empty() || mine == theirs {
                if mine > theirs {
                    'W'
                } else if mine < theirs {
                    'L'
                } else {
                    'D'
                }
            } else if winner.eq_ignore_ascii_case(&team) {
                'W'
            } else {
                'L'
            };
            Ok(Row {
                id: r.get(0)?,
                started: r.get(1)?,
                length: r.get(2)?,
                map: r.get(3)?,
                queue: r.get(4)?,
                team,
                agent: r.get(10)?,
                agent_id: r.get(11)?,
                my_score: mine,
                their_score: theirs,
                result,
                k: r.get(12)?,
                d: r.get(13)?,
                a: r.get(14)?,
                score: r.get(15)?,
                rounds: r.get::<_, i64>(8)?.max(1),
                hs: r.get(16)?,
                body: r.get(17)?,
                leg: r.get(18)?,
                dmg: r.get(19)?,
                rr: r.get(20)?,
                party: r.get(21)?,
                tier: r.get(22)?,
            })
        },
    )
    .map(|it| it.filter_map(|x| x.ok()).collect())
    .unwrap_or_default()
}

pub struct Agg {
    pub n: i64,
    pub w: i64,
    pub k: i64,
    pub d: i64,
    pub score: i64,
    pub rounds: i64,
    pub hs: i64,
    pub shots: i64,
    pub dmg: i64,
}
impl Agg {
    pub fn of<'a>(rows: impl IntoIterator<Item = &'a Row>) -> Agg {
        let mut a = Agg {
            n: 0,
            w: 0,
            k: 0,
            d: 0,
            score: 0,
            rounds: 0,
            hs: 0,
            shots: 0,
            dmg: 0,
        };
        for r in rows {
            a.n += 1;
            a.w += r.won() as i64;
            a.k += r.k;
            a.d += r.d;
            a.score += r.score;
            a.rounds += r.rounds;
            a.hs += r.hs;
            a.shots += r.hs + r.body + r.leg;
            a.dmg += r.dmg;
        }
        a
    }
    pub fn json(&self) -> Value {
        let div = |x: i64, y: i64| if y > 0 { x as f64 / y as f64 } else { 0.0 };
        json!({ "games": self.n, "wins": self.w, "win": (100.0 * div(self.w, self.n)).round(), "kd": (100.0 * div(self.k, self.d.max(1))).round() / 100.0,
            "acs": div(self.score, self.rounds).round(), "hs": (100.0 * div(self.hs, self.shots)).round(), "adr": div(self.dmg, self.rounds).round() })
    }
}

/// Group rows (newest first) into sessions: next match starts < 45 min after the previous ended.
pub fn sessions(rows_desc: &[Row]) -> Vec<Vec<Row>> {
    let mut asc: Vec<Row> = rows_desc.to_vec();
    asc.sort_by_key(|r| r.started);
    let mut out: Vec<Vec<Row>> = vec![];
    for r in asc {
        match out.last_mut() {
            Some(cur)
                if r.started - (cur.last().unwrap().started + cur.last().unwrap().length)
                    < SESSION_GAP_MS =>
            {
                cur.push(r)
            }
            _ => out.push(vec![r]),
        }
    }
    out
}

pub fn home(c: &Connection, me: &Me, prefs: &Value) -> Value {
    let rows = my_rows(
        c,
        &me.puuid,
        &Filter {
            limit: 400,
            ..Default::default()
        },
    );
    let comp: Vec<&Row> = rows.iter().filter(|r| r.queue == "competitive").collect();
    let sess = sessions(&rows);
    let now = crate::db::now_ms();
    let session = sess.last().map(|s| {
        let last = s.last().unwrap();
        let active = now - (last.started + last.length) < 3 * 3_600_000;
        let mut streak = 0i64;
        for r in s.iter().rev() {
            if r.result == 'L' { streak += 1 } else { break }
        }
        let tilt_after = prefs["tilt_after"].as_i64().unwrap_or(3);
        let a = Agg::of(s.iter());
        json!({ "active": active, "agg": a.json(), "results": s.iter().map(|r| r.result.to_string()).collect::<Vec<_>>(),
            "net_rr": s.iter().filter_map(|r| r.rr).sum::<i64>(), "minutes": (s.iter().map(|r| r.length).sum::<i64>() / 60_000),
            "loss_streak": streak, "tilt": streak >= tilt_after, "started_at": s[0].started })
    });
    let form: Vec<Value> = comp
        .iter()
        .take(20)
        .rev()
        .map(|r| json!({ "acs": r.acs().round(), "result": r.result.to_string() }))
        .collect();
    let season_acs = Agg::of(comp.iter().copied()).json()["acs"].clone();
    json!({
        "rank": rank_now(c, &me.puuid),
        "last": rows.first().map(|r| r.json()),
        "recent": rows.iter().take(5).map(|r| r.json()).collect::<Vec<_>>(),
        "session": session,
        "form": form, "season_acs": season_acs,
        "insights": insights(&rows),
        "total": rows.len(),
    })
}

/// Plain factual notes from recent matches. Never instructions.
pub fn insights(rows: &[Row]) -> Vec<Value> {
    let mut out = vec![];
    let comp: Vec<&Row> = rows
        .iter()
        .filter(|r| r.queue == "competitive")
        .take(60)
        .collect();
    let mut by_map: HashMap<&str, (i64, i64)> = HashMap::new();
    let mut by_agent: HashMap<&str, (i64, i64)> = HashMap::new();
    for r in &comp {
        let e = by_map.entry(&r.map).or_default();
        e.0 += 1;
        e.1 += r.won() as i64;
        let e = by_agent.entry(&r.agent).or_default();
        e.0 += 1;
        e.1 += r.won() as i64;
    }
    let pct = |w: i64, n: i64| (100 * w) as f64 / n as f64;
    if let Some((m, (n, w))) = by_map.iter().filter(|(_, (n, _))| *n >= 4).max_by(|a, b| {
        pct(a.1 .1, a.1 .0)
            .partial_cmp(&pct(b.1 .1, b.1 .0))
            .unwrap()
    }) {
        out.push(json!({ "icon": "map", "text": format!("Best map lately: {m}, {w}W {}L in {n} games.", n - w) }));
    }
    if let Some((m, (n, w))) = by_map.iter().filter(|(_, (n, _))| *n >= 4).min_by(|a, b| {
        pct(a.1 .1, a.1 .0)
            .partial_cmp(&pct(b.1 .1, b.1 .0))
            .unwrap()
    }) {
        if pct(*w, *n) < 45.0 {
            out.push(json!({ "icon": "map-pinned", "text": format!("{m} is your weakest map lately: {w}W {}L.", n - w) }));
        }
    }
    if let Some((a, (n, w))) = by_agent
        .iter()
        .filter(|(_, (n, _))| *n >= 5)
        .max_by(|a, b| {
            pct(a.1 .1, a.1 .0)
                .partial_cmp(&pct(b.1 .1, b.1 .0))
                .unwrap()
        })
    {
        out.push(json!({ "icon": "user", "text": format!("{a}: {:.0}% win rate over {n} games.", pct(*w, *n)) }));
    }
    out
}

pub fn rank_now(c: &Connection, puuid: &str) -> Value {
    let snap = c
        .query_row("SELECT tier, rr, peak_tier, elo, taken_at FROM mmr_snapshots WHERE puuid=?1 ORDER BY taken_at DESC LIMIT 1", [puuid], |r| {
            Ok(json!({ "tier": r.get::<_, i64>(0)?, "rr": r.get::<_, i64>(1)?, "peak_tier": r.get::<_, i64>(2)?, "elo": r.get::<_, i64>(3)?, "at": r.get::<_, i64>(4)? }))
        })
        .ok();
    let cached = crate::db::cache_get(c, &format!("mmr:{puuid}"), -1).map(|x| x.0);
    let mut v = snap.unwrap_or(Value::Null);
    if let (Some(o), Some(m)) = (v.as_object_mut(), cached) {
        o.insert("peak_season".into(), json!(s(&m, "/peak/season/short")));
        o.insert("last_change".into(), json!(i(&m, "/current/last_change")));
    }
    v
}

pub fn match_list(c: &Connection, me: &Me, fl: &Filter) -> Value {
    let rows = my_rows(c, &me.puuid, fl);
    let maps: Vec<String> = c
        .prepare("SELECT DISTINCT map_name FROM matches ORDER BY 1")
        .and_then(|mut s| {
            s.query_map([], |r| r.get(0))
                .map(|x| x.filter_map(|y| y.ok()).collect())
        })
        .unwrap_or_default();
    let agents: Vec<String> = c
        .prepare("SELECT DISTINCT agent_name FROM match_players WHERE puuid=?1 ORDER BY 1")
        .and_then(|mut s| {
            s.query_map([&me.puuid], |r| r.get(0))
                .map(|x| x.filter_map(|y| y.ok()).collect())
        })
        .unwrap_or_default();
    let queues: Vec<String> = c
        .prepare("SELECT DISTINCT queue FROM matches ORDER BY 1")
        .and_then(|mut s| {
            s.query_map([], |r| r.get(0))
                .map(|x| x.filter_map(|y| y.ok()).collect())
        })
        .unwrap_or_default();
    json!({ "rows": rows.iter().map(|r| r.json()).collect::<Vec<_>>(), "maps": maps, "agents": agents, "queues": queues })
}

pub fn notes_map(c: &Connection) -> HashMap<String, Value> {
    let mut m = HashMap::new();
    if let Ok(mut st) = c.prepare("SELECT puuid, tags_json, note FROM player_notes") {
        let _ = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, Option<String>>(2)?))).map(|it| {
            for (p, t, n) in it.flatten() {
                m.insert(p, json!({ "tags": t.and_then(|t| serde_json::from_str::<Value>(&t).ok()).unwrap_or(json!([])), "note": n.unwrap_or_default() }));
            }
        });
    }
    m
}

fn dist_m(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt() / 100.0
}

pub fn match_detail(c: &Connection, me: &Me, id: &str) -> Result<Value, String> {
    let raw = parse::load_raw(c, id).map_err(|e| e.to_string())?;
    let m = parse::unwrap_data(&raw);
    let rounds_n = arr(m, "/rounds").len().max(1) as f64;
    let players = arr(m, "/players");
    let my_team = players
        .iter()
        .find(|p| s(p, "/puuid") == me.puuid)
        .map(|p| s(p, "/team_id").to_string())
        .unwrap_or_default();
    let my_party = players
        .iter()
        .find(|p| s(p, "/puuid") == me.puuid)
        .map(|p| s(p, "/party_id").to_string())
        .unwrap_or_default();
    let kills = arr(m, "/kills");
    // First bloods per player.
    let mut first: HashMap<i64, (&str, i64)> = HashMap::new();
    for k in kills {
        let r = i(k, "/round");
        let t = i(k, "/time_in_round_in_ms");
        let e = first.entry(r).or_insert((s(k, "/killer/puuid"), t));
        if t < e.1 {
            *e = (s(k, "/killer/puuid"), t);
        }
    }
    let notes = notes_map(c);
    let met = |p: &str| -> i64 {
        c.query_row(
            "SELECT COUNT(DISTINCT match_id) FROM match_players WHERE puuid=?1 AND match_id<>?2",
            params![p, id],
            |r| r.get(0),
        )
        .unwrap_or(0)
    };
    let team_of: HashMap<&str, &str> = players
        .iter()
        .map(|p| (s(p, "/puuid"), s(p, "/team_id")))
        .collect();
    let agent_of: HashMap<&str, &str> = players
        .iter()
        .map(|p| (s(p, "/puuid"), s(p, "/agent/name")))
        .collect();
    let name_of: HashMap<&str, String> = players
        .iter()
        .map(|p| (s(p, "/puuid"), s(p, "/name").to_string()))
        .collect();
    let mut ps: Vec<Value> = players
        .iter()
        .map(|p| {
            let pu = s(p, "/puuid");
            let shots = i(p, "/stats/headshots") + i(p, "/stats/bodyshots") + i(p, "/stats/legshots");
            json!({
                "puuid": pu, "name": s(p, "/name"), "tag": s(p, "/tag"), "team": s(p, "/team_id"), "agent": s(p, "/agent/name"), "agent_id": s(p, "/agent/id"),
                "tier": i(p, "/tier/id"), "tier_name": s(p, "/tier/name"), "level": i(p, "/account_level"),
                "acs": (i(p, "/stats/score") as f64 / rounds_n).round(), "k": i(p, "/stats/kills"), "d": i(p, "/stats/deaths"), "a": i(p, "/stats/assists"),
                "hs": if shots > 0 { (100.0 * i(p, "/stats/headshots") as f64 / shots as f64).round() } else { 0.0 },
                "adr": (i(p, "/stats/damage/dealt") as f64 / rounds_n).round(),
                "fb": first.values().filter(|(k, _)| *k == pu).count(),
                "me": pu == me.puuid, "party": !my_party.is_empty() && s(p, "/party_id") == my_party && pu != me.puuid && players.iter().filter(|q| s(q, "/party_id") == my_party).count() > 1,
                "note": notes.get(pu).cloned(), "met": if pu == me.puuid { 0 } else { met(pu) },
            })
        })
        .collect();
    ps.sort_by(|a, b| {
        b["acs"]
            .as_f64()
            .partial_cmp(&a["acs"].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // Round start (time in match) = kill time in match - time in round.
    let mut round_start: HashMap<i64, i64> = HashMap::new();
    for k in kills {
        let st = i(k, "/time_in_match_in_ms") - i(k, "/time_in_round_in_ms");
        round_start
            .entry(i(k, "/round"))
            .and_modify(|v| *v = (*v).min(st))
            .or_insert(st);
    }
    let rounds: Vec<Value> = arr(m, "/rounds")
        .iter()
        .enumerate()
        .map(|(n, r)| {
            let n = n as i64;
            let mine: Vec<&Value> = kills.iter().filter(|k| i(k, "/round") == n).collect();
            json!({ "n": n, "won": s(r, "/winning_team").eq_ignore_ascii_case(&my_team), "result": s(r, "/result"),
                "side": parse::side(&my_team, n), "plant": s(r, "/plant/site"), "start_ms": round_start.get(&n),
                "my_kills": mine.iter().filter(|k| s(k, "/killer/puuid") == me.puuid).count(),
                "died": mine.iter().any(|k| s(k, "/victim/puuid") == me.puuid),
                "loadout": arr(r, "/stats").iter().find(|x| s(x, "/player/puuid") == me.puuid).map(|x| i(x, "/economy/loadout_value")) })
        })
        .collect();
    let loadout = |round: i64, pu: &str| -> i64 {
        arr(m, "/rounds")
            .get(round as usize)
            .map(|r| {
                arr(r, "/stats")
                    .iter()
                    .find(|x| s(x, "/player/puuid") == pu)
                    .map(|x| i(x, "/economy/loadout_value"))
                    .unwrap_or(0)
            })
            .unwrap_or(0)
    };
    let deaths: Vec<Value> = kills
        .iter()
        .filter(|k| s(k, "/victim/puuid") == me.puuid)
        .map(|k| {
            let killer = s(k, "/killer/puuid");
            let t = i(k, "/time_in_match_in_ms");
            let round = i(k, "/round");
            let traded = kills.iter().any(|x| s(x, "/victim/puuid") == killer && team_of.get(s(x, "/killer/puuid")) == Some(&my_team.as_str()) && { let d = i(x, "/time_in_match_in_ms") - t; (0..=3000).contains(&d) });
            let me_loc = (f(k, "/location/x"), f(k, "/location/y"));
            let nearest = match me_loc {
                (Some(x), Some(y)) => arr(k, "/player_locations")
                    .iter()
                    .filter(|l| { let p = s(l, "/player/puuid"); p != me.puuid && team_of.get(p) == Some(&my_team.as_str()) })
                    .filter_map(|l| Some(dist_m((x, y), (f(l, "/location/x")?, f(l, "/location/y")?))))
                    .fold(None, |acc: Option<f64>, d| Some(acc.map_or(d, |a| a.min(d)))),
                _ => None,
            };
            json!({ "round": round, "time_in_round_ms": i(k, "/time_in_round_in_ms"), "time_in_match_ms": t,
                "killer": name_of.get(killer).cloned().unwrap_or_default(), "killer_agent": agent_of.get(killer).copied().unwrap_or(""),
                "weapon": s(k, "/weapon/name"), "traded": traded, "nearest_ally_m": nearest.map(|d| d.round()),
                "my_loadout": loadout(round, &me.puuid), "their_loadout": loadout(round, killer) })
        })
        .collect();
    let teams: Vec<Value> = arr(m, "/teams").iter().map(|t| json!({ "team": s(t, "/team_id"), "won": i(t, "/rounds/won"), "mine": s(t, "/team_id").eq_ignore_ascii_case(&my_team) })).collect();
    let rr: Option<i64> = c
        .query_row(
            "SELECT rr_change FROM rr_changes WHERE match_id=?1",
            [id],
            |r| r.get(0),
        )
        .ok();
    let recording = crate::recordings::for_match(c, id);
    let reviews: Vec<Value> = c
        .prepare("SELECT id, round_no, kind, note, done_at FROM review_items WHERE match_id=?1")
        .and_then(|mut st| st.query_map([id], |r| Ok(json!({ "id": r.get::<_, i64>(0)?, "round": r.get::<_, Option<i64>>(1)?, "kind": r.get::<_, String>(2)?, "note": r.get::<_, Option<String>>(3)?, "done": r.get::<_, Option<i64>>(4)?.is_some() }))).map(|x| x.flatten().collect()))
        .unwrap_or_default();
    Ok(json!({
        "id": id, "map": s(m, "/metadata/map/name"), "queue": s(m, "/metadata/queue/name"), "started_at": parse::time_ms(m.pointer("/metadata/started_at").unwrap_or(&Value::Null)),
        "length_ms": i(m, "/metadata/game_length_in_ms"), "version": s(m, "/metadata/game_version"), "my_team": my_team,
        "teams": teams, "players": ps, "rounds": rounds, "deaths": deaths, "rr": rr, "recording": recording, "reviews": reviews,
    }))
}

pub fn analytics(c: &Connection, me: &Me, since: i64, queue: &str) -> Value {
    let rows = my_rows(
        c,
        &me.puuid,
        &Filter {
            queue: queue.into(),
            since,
            ..Default::default()
        },
    );
    let mut by_agent: HashMap<String, Vec<&Row>> = HashMap::new();
    let mut by_map: HashMap<String, Vec<&Row>> = HashMap::new();
    for r in &rows {
        by_agent.entry(r.agent.clone()).or_default().push(r);
        by_map.entry(r.map.clone()).or_default().push(r);
    }
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    let team_by_match: HashMap<&str, &str> = rows
        .iter()
        .map(|r| (r.id.as_str(), r.team.as_str()))
        .collect();
    let map_by_match: HashMap<&str, &str> = rows
        .iter()
        .map(|r| (r.id.as_str(), r.map.as_str()))
        .collect();
    // Side round win rates per map, and economy buckets.
    let mut side: HashMap<(String, &'static str), (i64, i64)> = HashMap::new();
    let mut econ: HashMap<&'static str, (i64, i64)> = HashMap::new();
    let mut weapons: HashMap<String, i64> = HashMap::new();
    let (mut deaths, mut untraded) = (0i64, 0i64);
    for id in &ids {
        let team = team_by_match[id];
        if let Ok(mut st) = c.prepare("SELECT r.round_no, r.winning_team, COALESCE(s.loadout_value,-1) FROM rounds r LEFT JOIN round_player_stats s ON s.match_id=r.match_id AND s.round_no=r.round_no AND s.puuid=?2 WHERE r.match_id=?1") {
            let it = st.query_map(params![id, me.puuid], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?)));
            if let Ok(it) = it {
                for (n, w, lo) in it.flatten() {
                    let won = w.eq_ignore_ascii_case(team);
                    let e = side.entry((map_by_match[id].to_string(), parse::side(team, n))).or_default();
                    e.0 += 1;
                    e.1 += won as i64;
                    if lo >= 0 {
                        let b = if n == 0 || n == 12 { "Pistol" } else if lo < 2000 { "Eco" } else if lo < 3900 { "Force" } else { "Full buy" };
                        let e = econ.entry(b).or_default();
                        e.0 += 1;
                        e.1 += won as i64;
                    }
                }
            }
        }
        if let Ok(mut st) =
            c.prepare("SELECT weapon_name FROM kills WHERE match_id=?1 AND killer=?2")
        {
            if let Ok(it) = st.query_map(params![id, me.puuid], |r| r.get::<_, String>(0)) {
                for w in it.flatten() {
                    *weapons
                        .entry(if w.is_empty() { "Ability".into() } else { w })
                        .or_default() += 1;
                }
            }
        }
        // Untraded deaths: no teammate killed my killer within 3 s.
        if let Ok(mut st) = c.prepare(
            "SELECT k.killer, k.time_in_match_ms, (SELECT COUNT(*) FROM kills t JOIN match_players tp ON tp.match_id=t.match_id AND tp.puuid=t.killer
                WHERE t.match_id=k.match_id AND t.victim=k.killer AND tp.team=?3 AND t.time_in_match_ms BETWEEN k.time_in_match_ms AND k.time_in_match_ms+3000)
             FROM kills k WHERE k.match_id=?1 AND k.victim=?2",
        ) {
            if let Ok(it) = st.query_map(params![id, me.puuid, team], |r| r.get::<_, i64>(2)) {
                for t in it.flatten() {
                    deaths += 1;
                    untraded += (t == 0) as i64;
                }
            }
        }
    }
    let pct = |(n, w): (i64, i64)| {
        if n > 0 {
            json!((100.0 * w as f64 / n as f64).round())
        } else {
            Value::Null
        }
    };
    let mut agents: Vec<Value> = by_agent
        .iter()
        .map(|(a, rs)| {
            let mut j = Agg::of(rs.iter().copied()).json();
            j["name"] = json!(a);
            j["agent_id"] = json!(rs[0].agent_id);
            j
        })
        .collect();
    agents.sort_by_key(|j| -j["games"].as_i64().unwrap_or(0));
    let mut maps: Vec<Value> = by_map
        .iter()
        .map(|(m, rs)| {
            let mut j = Agg::of(rs.iter().copied()).json();
            j["name"] = json!(m);
            j["atk"] = pct(*side.get(&(m.clone(), "atk")).unwrap_or(&(0, 0)));
            j["def"] = pct(*side.get(&(m.clone(), "def")).unwrap_or(&(0, 0)));
            j
        })
        .collect();
    maps.sort_by_key(|j| -j["games"].as_i64().unwrap_or(0));
    let total_kills: i64 = weapons.values().sum();
    let mut w: Vec<(String, i64)> = weapons.into_iter().collect();
    w.sort_by_key(|x| -x.1);
    json!({
        "overall": Agg::of(rows.iter()).json(), "agents": agents, "maps": maps,
        "economy": (["Pistol", "Eco", "Force", "Full buy"]).iter().map(|b| json!({ "name": b, "rounds": econ.get(b).map(|x| x.0).unwrap_or(0), "win": pct(*econ.get(b).unwrap_or(&(0, 0))) })).collect::<Vec<_>>(),
        "weapons": w.iter().take(6).map(|(n, k)| json!({ "name": n, "kills": k, "share": if total_kills > 0 { 100 * k / total_kills } else { 0 } })).collect::<Vec<_>>(),
        "untraded": if deaths > 0 { json!((100.0 * untraded as f64 / deaths as f64).round()) } else { Value::Null },
        "deaths": deaths,
    })
}

/// "When you play best" (plan 25.1).
pub fn habits(c: &Connection, me: &Me) -> Value {
    let rows = my_rows(
        c,
        &me.puuid,
        &Filter {
            queue: "competitive".into(),
            ..Default::default()
        },
    );
    let party_size = |id: &str, party: &str| -> i64 {
        if party.is_empty() {
            return 1;
        }
        c.query_row(
            "SELECT COUNT(*) FROM match_players WHERE match_id=?1 AND party_id=?2",
            params![id, party],
            |r| r.get(0),
        )
        .unwrap_or(1)
    };
    let mut idx: [(i64, i64); 5] = [(0, 0); 5];
    let mut hour: [(i64, i64); 4] = [(0, 0); 4];
    let mut after2 = (0i64, 0i64);
    let mut party: [(i64, i64); 3] = [(0, 0); 3];
    let (mut first_acs, mut later_acs) = ((0f64, 0i64), (0f64, 0i64));
    let mut last_day = String::new();
    for s in sessions(&rows) {
        for (n, r) in s.iter().enumerate() {
            let won = r.won() as i64;
            let b = &mut idx[n.min(4)];
            b.0 += 1;
            b.1 += won;
            let local = chrono::DateTime::from_timestamp_millis(r.started)
                .map(|d| d.with_timezone(&chrono::Local));
            if let Some(t) = local {
                use chrono::Timelike;
                let h = t.hour();
                let bi = if (6..12).contains(&h) {
                    0
                } else if (12..18).contains(&h) {
                    1
                } else if h >= 18 {
                    2
                } else {
                    3
                };
                hour[bi].0 += 1;
                hour[bi].1 += won;
                let day = t.format("%Y-%m-%d").to_string();
                if day != last_day {
                    first_acs.0 += r.acs();
                    first_acs.1 += 1;
                    last_day = day;
                } else {
                    later_acs.0 += r.acs();
                    later_acs.1 += 1;
                }
            }
            if n >= 2 && s[n - 1].result == 'L' && s[n - 2].result == 'L' {
                after2.0 += 1;
                after2.1 += won;
            }
            let ps = party_size(&r.id, &r.party);
            let pi = if ps <= 1 {
                0
            } else if ps == 2 {
                1
            } else {
                2
            };
            party[pi].0 += 1;
            party[pi].1 += won;
        }
    }
    let row = |label: &str, (n, w): (i64, i64)| json!({ "label": label, "games": n, "win": if n > 0 { json!((100.0 * w as f64 / n as f64).round()) } else { Value::Null } });
    let total = rows.len() as i64;
    let all_w = rows.iter().filter(|r| r.won()).count() as i64;
    let early = (
        idx[0].0 + idx[1].0 + idx[2].0,
        idx[0].1 + idx[1].1 + idx[2].1,
    );
    let late = (idx[3].0 + idx[4].0, idx[3].1 + idx[4].1);
    let p = |(n, w): (i64, i64)| {
        if n > 0 {
            100.0 * w as f64 / n as f64
        } else {
            0.0
        }
    };
    let verdict = if total >= 40 && late.0 >= 10 && p(early) - p(late) >= 8.0 {
        Some(format!("You play best in your first three matches: {:.0}% win ({} matches) vs {:.0}% from the 4th match on ({} matches).", p(early), early.0, p(late), late.0))
    } else if total >= 40 && after2.0 >= 10 && p((total, all_w)) - p(after2) >= 8.0 {
        Some(format!(
            "After 2 losses in a row you win {:.0}% ({} matches) vs {:.0}% overall.",
            p(after2),
            after2.0,
            p((total, all_w))
        ))
    } else {
        None
    };
    json!({
        "total": total, "enough": total >= 40, "verdict": verdict,
        "by_index": (["1st", "2nd", "3rd", "4th", "5th +"]).iter().zip(idx).map(|(l, x)| row(l, x)).collect::<Vec<_>>(),
        "by_hour": (["Morning", "Afternoon", "Evening", "After midnight"]).iter().zip(hour).map(|(l, x)| row(l, x)).collect::<Vec<_>>(),
        "after_two_losses": row("After 2+ losses", after2), "overall": row("Overall", (total, all_w)),
        "party": (["Solo", "Duo", "3+"]).iter().zip(party).map(|(l, x)| row(l, x)).collect::<Vec<_>>(),
        "first_of_day_acs": if first_acs.1 > 0 { json!((first_acs.0 / first_acs.1 as f64).round()) } else { Value::Null },
        "later_acs": if later_acs.1 > 0 { json!((later_acs.0 / later_acs.1 as f64).round()) } else { Value::Null },
    })
}

pub fn rank(c: &Connection, me: &Me) -> Value {
    let snaps: Vec<Value> = c
        .prepare("SELECT taken_at, tier, rr, elo FROM mmr_snapshots WHERE puuid=?1 ORDER BY taken_at")
        .and_then(|mut st| st.query_map([&me.puuid], |r| Ok(json!({ "at": r.get::<_, i64>(0)?, "tier": r.get::<_, i64>(1)?, "rr": r.get::<_, i64>(2)?, "elo": r.get::<_, i64>(3)? }))).map(|x| x.flatten().collect()))
        .unwrap_or_default();
    let rows = my_rows(
        c,
        &me.puuid,
        &Filter {
            queue: "competitive".into(),
            limit: 40,
            ..Default::default()
        },
    );
    let changes: Vec<Value> = rows
        .iter()
        .filter(|r| r.rr.is_some())
        .map(|r| r.json())
        .collect();
    let gains: Vec<i64> = rows
        .iter()
        .filter_map(|r| r.rr)
        .filter(|x| *x > 0)
        .collect();
    let now = rank_now(c, &me.puuid);
    let rr = now["rr"].as_i64().unwrap_or(0);
    let avg_gain = if gains.is_empty() {
        None
    } else {
        Some(gains.iter().sum::<i64>() as f64 / gains.len() as f64)
    };
    let to_next = avg_gain.map(|g| ((100 - rr).max(1) as f64 / g).ceil() as i64);
    let mmr = crate::db::cache_get(c, &format!("mmr:{}", me.puuid), -1)
        .map(|x| x.0)
        .unwrap_or(Value::Null);
    let seasons: Vec<Value> = arr(&mmr, "/seasonal").iter().map(|x| json!({ "season": s(x, "/season/short"), "wins": i(x, "/wins"), "games": i(x, "/games"), "end_tier": i(x, "/end_tier/id"), "end_rr": i(x, "/end_rr") })).collect();
    json!({ "now": now, "snapshots": snaps, "changes": changes, "wins_to_next": to_next, "avg_gain": avg_gain.map(|g| g.round()), "seasons": seasons })
}

pub fn people(c: &Connection, me: &Me) -> Value {
    let notes = notes_map(c);
    let mut out: Vec<Value> = c
        .prepare(
            "SELECT mp.puuid, COALESCE(p.name, mp.name), COALESCE(p.tag, mp.tag), COUNT(*),
               SUM(mp.team=mine.team), SUM(CASE WHEN mp.team=mine.team AND mine.team=m.winning_team THEN 1 ELSE 0 END),
               SUM(mp.team<>mine.team), SUM(CASE WHEN mp.team<>mine.team AND mine.team=m.winning_team THEN 1 ELSE 0 END),
               MAX(m.started_at), SUM(CASE WHEN mine.party_id<>'' AND mp.party_id=mine.party_id THEN 1 ELSE 0 END)
             FROM match_players mp JOIN match_players mine ON mine.match_id=mp.match_id AND mine.puuid=?1
             JOIN matches m ON m.match_id=mp.match_id LEFT JOIN players p ON p.puuid=mp.puuid
             WHERE mp.puuid<>?1 GROUP BY mp.puuid
             HAVING COUNT(*)>=2 OR mp.puuid IN (SELECT puuid FROM player_notes)
             ORDER BY 10 DESC, 4 DESC LIMIT 300",
        )
        .and_then(|mut st| {
            st.query_map([&me.puuid], |r| {
                Ok(json!({ "puuid": r.get::<_, String>(0)?, "name": r.get::<_, String>(1)?, "tag": r.get::<_, String>(2)?, "games": r.get::<_, i64>(3)?,
                    "with": r.get::<_, i64>(4)?, "with_wins": r.get::<_, i64>(5)?, "vs": r.get::<_, i64>(6)?, "vs_wins": r.get::<_, i64>(7)?,
                    "last": r.get::<_, i64>(8)?, "party": r.get::<_, i64>(9)? }))
            })
            .map(|x| x.flatten().collect())
        })
        .unwrap_or_default();
    for p in out.iter_mut() {
        let pu = p["puuid"].as_str().unwrap_or("").to_string();
        p["note"] = notes.get(&pu).cloned().unwrap_or(Value::Null);
    }
    json!({ "people": out })
}

pub fn reviews(c: &Connection, me: &Me, done: bool) -> Value {
    let items: Vec<Value> = c
        .prepare(&format!(
            "SELECT r.id, r.match_id, r.round_no, r.kind, r.note, r.time_in_match_ms, r.created_at, m.map_name, m.started_at, mp.agent_name
             FROM review_items r JOIN matches m ON m.match_id=r.match_id LEFT JOIN match_players mp ON mp.match_id=r.match_id AND mp.puuid=?1
             WHERE r.done_at IS {} NULL ORDER BY r.created_at DESC",
            if done { "NOT" } else { "" }
        ))
        .and_then(|mut st| {
            st.query_map([&me.puuid], |r| {
                Ok(json!({ "id": r.get::<_, i64>(0)?, "match_id": r.get::<_, String>(1)?, "round": r.get::<_, Option<i64>>(2)?, "kind": r.get::<_, String>(3)?,
                    "note": r.get::<_, Option<String>>(4)?, "t": r.get::<_, Option<i64>>(5)?, "created_at": r.get::<_, i64>(6)?, "map": r.get::<_, String>(7)?,
                    "started_at": r.get::<_, i64>(8)?, "agent": r.get::<_, Option<String>>(9)? }))
            })
            .map(|x| x.flatten().collect())
        })
        .unwrap_or_default();
    let items = items
        .into_iter()
        .map(|mut v| {
            let rec = crate::recordings::for_match(c, v["match_id"].as_str().unwrap_or(""));
            v["recording"] = rec;
            v
        })
        .collect::<Vec<_>>();
    json!({ "items": items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{fixture, store_match};
    fn setup() -> (Connection, Me) {
        let mut c = crate::db::open_memory().unwrap();
        for (n, won) in [(0, true), (1, false), (2, false), (3, true)] {
            store_match(
                &mut c,
                &fixture::match_v4(&format!("m{n}"), won, &format!("2026-11-11T1{n}:00:00Z")),
            )
            .unwrap();
        }
        (
            c,
            Me {
                puuid: "p0".into(),
                name: "Player0".into(),
                tag: "t".into(),
                region: "ap".into(),
                account_level: 1,
            },
        )
    }
    #[test]
    fn rows_and_results() {
        let (c, me) = setup();
        let rows = my_rows(&c, &me.puuid, &Filter::default());
        assert_eq!(rows.len(), 4);
        assert_eq!(rows.iter().filter(|r| r.won()).count(), 2);
        assert_eq!(rows[0].id, "m3");
    }
    #[test]
    fn sessions_split_on_gap() {
        let (c, me) = setup();
        // matches are 60 min apart with 40 min length: 20 min gap, same session
        let rows = my_rows(&c, &me.puuid, &Filter::default());
        assert_eq!(sessions(&rows).len(), 1);
    }
    #[test]
    fn match_detail_finds_untraded_and_traded_deaths() {
        let (c, me) = setup();
        let d = match_detail(&c, &me, "m0").unwrap();
        assert_eq!(d["players"].as_array().unwrap().len(), 10);
        let death = &d["deaths"][0];
        assert_eq!(death["traded"], true);
        assert_eq!(death["nearest_ally_m"], 25.0);
    }
    #[test]
    fn analytics_and_habits_run() {
        let (c, me) = setup();
        let a = analytics(&c, &me, 0, "competitive");
        assert_eq!(a["overall"]["games"], 4);
        let h = habits(&c, &me);
        assert_eq!(h["total"], 4);
        assert_eq!(h["enough"], false);
        assert!(people(&c, &me)["people"].as_array().unwrap().len() >= 9);
    }
}
