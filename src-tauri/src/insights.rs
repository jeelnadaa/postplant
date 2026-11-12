//! Post-match insights from stored matches only (plan section 16): heatmap layers, where rounds end,
//! opening duels, trade stats, pattern notes, duo synergy, post-match facts and ability usage.
//! Zero API requests; everything reads the local database.
use crate::advisor::{Kb, MapInfo};
use crate::parse::{arr, f, i, load_raw, s, side, unwrap_data};
use crate::queries::{my_rows, Filter, Row};
use crate::state::Me;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::collections::HashMap;

const TRADE_MS: i64 = 5000;

/// "B Main", "Mid Courtyard", "Attacker Spawn". Nearest valorant-api.com callout to a game position.
pub fn callout(map: Option<&MapInfo>, x: f64, y: f64) -> String {
    let Some(m) = map else { return String::new() };
    m.callouts
        .iter()
        .map(|(sup, reg, cx, cy)| ((cx - x).powi(2) + (cy - y).powi(2), sup, reg))
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, sup, reg)| {
            if sup.is_empty() || sup.ends_with("Side") || reg.starts_with(sup.as_str()) {
                reg.clone()
            } else {
                format!("{sup} {reg}")
            }
        })
        .unwrap_or_default()
}

fn loc(v: &Value, p: &str) -> Option<(f64, f64)> {
    Some((f(v, &format!("{p}/x"))?, f(v, &format!("{p}/y"))?))
}

fn pos_of(kill: &Value, puuid: &str) -> Option<(f64, f64)> {
    arr(kill, "/player_locations")
        .iter()
        .find(|l| s(l, "/player/puuid") == puuid)
        .and_then(|l| loc(l, "/location"))
}

/// Kills of one round, sorted by time.
fn round_kills(m: &Value, n: i64) -> Vec<&Value> {
    let mut ks: Vec<&Value> = arr(m, "/kills")
        .iter()
        .filter(|k| i(k, "/round") == n)
        .collect();
    ks.sort_by_key(|k| i(k, "/time_in_round_in_ms"));
    ks
}

struct Ctx<'a> {
    m: &'a Value,
    team_of: HashMap<String, String>,
    agent_of: HashMap<String, String>,
    name_of: HashMap<String, String>,
    my_team: String,
}

fn ctx<'a>(m: &'a Value, me: &str) -> Ctx<'a> {
    let players = arr(m, "/players");
    let team_of: HashMap<String, String> = players
        .iter()
        .map(|p| (s(p, "/puuid").to_string(), s(p, "/team_id").to_lowercase()))
        .collect();
    let agent_of = players
        .iter()
        .map(|p| (s(p, "/puuid").to_string(), s(p, "/agent/name").to_string()))
        .collect();
    let name_of = players
        .iter()
        .map(|p| (s(p, "/puuid").to_string(), s(p, "/name").to_string()))
        .collect();
    let my_team = team_of.get(me).cloned().unwrap_or_default();
    Ctx {
        m,
        team_of,
        agent_of,
        name_of,
        my_team,
    }
}

impl Ctx<'_> {
    fn ally(&self, pu: &str) -> bool {
        self.team_of
            .get(pu)
            .map(|t| *t == self.my_team)
            .unwrap_or(false)
    }
    fn round_won(&self, n: i64) -> bool {
        arr(self.m, "/rounds")
            .get(n as usize)
            .map(|r| s(r, "/winning_team").eq_ignore_ascii_case(&self.my_team))
            .unwrap_or(false)
    }
    fn loadout(&self, n: i64, pu: &str) -> i64 {
        arr(self.m, "/rounds")
            .get(n as usize)
            .and_then(|r| {
                arr(r, "/stats")
                    .iter()
                    .find(|x| s(x, "/player/puuid") == pu)
                    .map(|x| i(x, "/economy/loadout_value"))
            })
            .unwrap_or(0)
    }
}

fn load_rows(c: &Connection, me: &Me, map: &str, queue: &str, limit: i64) -> Vec<(Row, Value)> {
    let rows = my_rows(
        c,
        &me.puuid,
        &Filter {
            map: map.into(),
            queue: queue.into(),
            limit,
            ..Default::default()
        },
    );
    rows.into_iter()
        .filter_map(|r| {
            let raw = load_raw(c, &r.id).ok()?;
            let m = unwrap_data(&raw).clone();
            Some((r, m))
        })
        .collect()
}

pub struct HeatOpts {
    pub map: String,
    pub event: String,
    pub side: String,
    pub layer: String,
    pub agent: String,
    pub outcome: String,
    pub rtype: String,
    /// Only this match (empty: every stored match on the map).
    pub match_id: String,
    /// Queue id ("" = all modes) and earliest start time in ms (0 = any).
    pub queue: String,
    pub since: i64,
}

/// Points for the heatmap plus the callouts they cluster in.
/// events: kills, deaths, opening (first duel of the round), round_end (last kill), plants, defuses.
/// layer: me / team / enemy. Points are [x, y, good] where good = round won (duel won for opening).
pub fn heatmap(c: &Connection, kb: &Kb, me: &Me, o: &HeatOpts) -> Value {
    let mi = kb.map(&o.map);
    let mut data = load_rows(c, me, &o.map, &o.queue, 300);
    if !o.match_id.is_empty() {
        data.retain(|(r, _)| r.id == o.match_id);
    }
    data.retain(|(r, _)| r.started >= o.since);
    let mut pts: Vec<(f64, f64, bool)> = vec![];
    for (r, m) in &data {
        let cx = ctx(m, &me.puuid);
        let in_layer = |pu: &str| -> bool {
            let ok = match o.layer.as_str() {
                "team" => cx.ally(pu),
                "enemy" => !pu.is_empty() && !cx.ally(pu),
                _ => pu == me.puuid,
            };
            ok && (o.agent.is_empty()
                || cx
                    .agent_of
                    .get(pu)
                    .map(|a| a.eq_ignore_ascii_case(&o.agent))
                    .unwrap_or(false))
        };
        for (n, round) in arr(m, "/rounds").iter().enumerate() {
            let n = n as i64;
            if !o.side.is_empty() && side(&r.team, n) != o.side {
                continue;
            }
            let won = cx.round_won(n);
            if (o.outcome == "won" && !won) || (o.outcome == "lost" && won) {
                continue;
            }
            if !o.rtype.is_empty() {
                let lv = cx.loadout(n, &me.puuid);
                let t = if n == 0 || n == 12 {
                    "pistol"
                } else if lv < 2000 {
                    "eco"
                } else if lv < 3900 {
                    "force"
                } else {
                    "full"
                };
                if t != o.rtype {
                    continue;
                }
            }
            let ks = round_kills(m, n);
            match o.event.as_str() {
                "kills" => {
                    for k in &ks {
                        let pu = s(k, "/killer/puuid");
                        if in_layer(pu) {
                            if let Some((x, y)) = pos_of(k, pu) {
                                pts.push((x, y, won));
                            }
                        }
                    }
                }
                "opening" => {
                    if let Some(k) = ks.first() {
                        let (kl, vc) = (s(k, "/killer/puuid"), s(k, "/victim/puuid"));
                        if in_layer(kl) {
                            if let Some((x, y)) = pos_of(k, kl) {
                                pts.push((x, y, true));
                            }
                        } else if in_layer(vc) {
                            if let Some((x, y)) = loc(k, "/location") {
                                pts.push((x, y, false));
                            }
                        }
                    }
                }
                "round_end" => {
                    if let Some(k) = ks.last() {
                        if let Some((x, y)) = loc(k, "/location") {
                            pts.push((x, y, won));
                        }
                    }
                }
                "plants" | "defuses" => {
                    let key = if o.event == "plants" {
                        "/plant"
                    } else {
                        "/defuse"
                    };
                    if let Some(ev) = round.pointer(key).filter(|v| !v.is_null()) {
                        if in_layer(s(ev, "/player/puuid")) {
                            if let Some((x, y)) = loc(ev, "/location") {
                                pts.push((x, y, won));
                            }
                        }
                    }
                }
                _ => {
                    for k in &ks {
                        if in_layer(s(k, "/victim/puuid")) {
                            if let Some((x, y)) = loc(k, "/location") {
                                pts.push((x, y, won));
                            }
                        }
                    }
                }
            }
        }
    }
    let mut spots: HashMap<String, (i64, i64)> = HashMap::new();
    for (x, y, g) in &pts {
        let e = spots.entry(callout(mi, *x, *y)).or_default();
        e.0 += 1;
        e.1 += *g as i64;
    }
    let mut sp: Vec<Value> = spots
        .into_iter()
        .filter(|(k, _)| !k.is_empty())
        .map(|(k, (n, g))| json!({ "name": k, "n": n, "good": g, "pct": if n > 0 { 100 * g / n } else { 0 } }))
        .collect();
    sp.sort_by_key(|v| -v["n"].as_i64().unwrap_or(0));
    sp.truncate(8);
    let agents: Vec<String> = {
        let mut a: Vec<String> = data.iter().map(|(r, _)| r.agent.clone()).collect();
        a.sort();
        a.dedup();
        a
    };
    json!({ "map": o.map, "event": o.event, "matches": data.len(), "points": pts.iter().map(|(x, y, g)| json!([x, y, g])).collect::<Vec<_>>(),
        "spots": sp, "agents": agents })
}

/// Trade stats: how often and how fast your deaths are traded, and your best trading partners.
pub fn trades(c: &Connection, me: &Me) -> Value {
    let data = load_rows(c, me, "", "competitive", 100);
    let (mut deaths, mut traded, mut total_ms) = (0i64, 0i64, 0i64);
    let mut partners: HashMap<String, (String, i64, i64)> = HashMap::new();
    let mut team_deaths = (0i64, 0i64);
    for (_, m) in &data {
        let cx = ctx(m, &me.puuid);
        let all = arr(m, "/kills");
        for k in all {
            let (kl, vc) = (s(k, "/killer/puuid"), s(k, "/victim/puuid"));
            let (n, t) = (i(k, "/round"), i(k, "/time_in_round_in_ms"));
            let trade = all.iter().filter(|x| i(x, "/round") == n).find(|x| {
                s(x, "/victim/puuid") == kl
                    && (0..=TRADE_MS).contains(&(i(x, "/time_in_round_in_ms") - t))
                    && cx.ally(s(x, "/killer/puuid")) == cx.ally(vc)
            });
            if cx.ally(vc) {
                team_deaths.0 += 1;
                team_deaths.1 += trade.is_some() as i64;
            }
            if vc == me.puuid {
                deaths += 1;
                if let Some(x) = trade {
                    traded += 1;
                    total_ms += i(x, "/time_in_round_in_ms") - t;
                    let p = s(x, "/killer/puuid");
                    let e = partners.entry(p.to_string()).or_insert((
                        cx.name_of.get(p).cloned().unwrap_or_default(),
                        0,
                        0,
                    ));
                    e.1 += 1;
                }
            } else if cx.ally(vc) {
                if let Some(x) = trade {
                    if s(x, "/killer/puuid") == me.puuid {
                        let e = partners.entry(vc.to_string()).or_insert((
                            cx.name_of.get(vc).cloned().unwrap_or_default(),
                            0,
                            0,
                        ));
                        e.2 += 1;
                    }
                }
            }
        }
    }
    let mut ps: Vec<Value> = partners
        .into_iter()
        .filter(|(_, v)| !v.0.is_empty() && v.1 + v.2 >= 2)
        .map(|(pu, (n, a, b))| json!({ "puuid": pu, "name": n, "traded_me": a, "i_traded": b }))
        .collect();
    ps.sort_by_key(|v| {
        -(v["traded_me"].as_i64().unwrap_or(0) + v["i_traded"].as_i64().unwrap_or(0))
    });
    ps.truncate(5);
    json!({ "matches": data.len(), "deaths": deaths, "traded": traded, "traded_pct": if deaths > 0 { 100 * traded / deaths } else { 0 },
        "avg_trade_ms": if traded > 0 { total_ms / traded } else { 0 },
        "team_traded_pct": if team_deaths.0 > 0 { 100 * team_deaths.1 / team_deaths.0 } else { 0 }, "partners": ps })
}

/// (map, side, spot)
type SpotKey = (String, &'static str, String);

/// Plain-language notes from recent matches, each with the rounds it came from. Factual, post-match.
pub fn patterns(c: &Connection, kb: &Kb, me: &Me) -> Value {
    let data = load_rows(c, me, "", "competitive", 40);
    // (map, side) -> rounds; (map, side, spot) -> (first deaths, untraded, refs); (map, side, spot) -> deaths
    let mut rounds: HashMap<(String, &'static str), i64> = HashMap::new();
    let mut deaths_total: HashMap<(String, &'static str), i64> = HashMap::new();
    let mut first: HashMap<SpotKey, (i64, i64, Vec<Value>)> = HashMap::new();
    let mut spot_deaths: HashMap<SpotKey, (i64, Vec<Value>)> = HashMap::new();
    let mut opening: HashMap<String, (i64, i64)> = HashMap::new();
    for (r, m) in &data {
        let cx = ctx(m, &me.puuid);
        let mi = kb.map(&r.map);
        for n in 0..arr(m, "/rounds").len() as i64 {
            let sd = side(&r.team, n);
            *rounds.entry((r.map.clone(), sd)).or_default() += 1;
            let ks = round_kills(m, n);
            let rf = json!({ "match_id": r.id, "round": n });
            if let Some(k0) = ks.first() {
                let (kl, vc) = (s(k0, "/killer/puuid"), s(k0, "/victim/puuid"));
                if kl == me.puuid || vc == me.puuid {
                    let e = opening.entry(r.map.clone()).or_default();
                    e.0 += 1;
                    e.1 += (kl == me.puuid) as i64;
                }
            }
            let mut team_first_death_seen = false;
            for k in &ks {
                let vc = s(k, "/victim/puuid");
                if !cx.ally(vc) {
                    continue;
                }
                let is_first = !team_first_death_seen;
                team_first_death_seen = true;
                if vc != me.puuid {
                    continue;
                }
                *deaths_total.entry((r.map.clone(), sd)).or_default() += 1;
                let Some((x, y)) = loc(k, "/location") else {
                    continue;
                };
                let spot = callout(mi, x, y);
                if spot.is_empty() {
                    continue;
                }
                let t = i(k, "/time_in_round_in_ms");
                let kl = s(k, "/killer/puuid");
                let traded = ks.iter().any(|x| {
                    s(x, "/victim/puuid") == kl
                        && cx.ally(s(x, "/killer/puuid"))
                        && (0..=TRADE_MS).contains(&(i(x, "/time_in_round_in_ms") - t))
                });
                let e = spot_deaths
                    .entry((r.map.clone(), sd, spot.clone()))
                    .or_default();
                e.0 += 1;
                e.1.push(rf.clone());
                if is_first {
                    let e = first.entry((r.map.clone(), sd, spot)).or_default();
                    e.0 += 1;
                    e.1 += (!traded) as i64;
                    e.2.push(rf.clone());
                }
            }
        }
    }
    let side_name = |s: &str| if s == "atk" { "attack" } else { "defense" };
    let mut notes: Vec<(i64, Value)> = vec![];
    for ((map, sd, spot), (n, untraded, refs)) in &first {
        let total = rounds.get(&(map.clone(), *sd)).copied().unwrap_or(0);
        if *n >= 3 && total > 0 && *n * 4 >= total {
            notes.push((*n * 3, json!({ "kind": "first_death", "map": map, "side": sd,
                "text": format!("First of your team to die near {spot} in {n} of {total} {} rounds on {map}; {untraded} not traded.", side_name(sd)),
                "refs": refs.iter().take(12).collect::<Vec<_>>() })));
        }
    }
    for ((map, sd, spot), (n, refs)) in &spot_deaths {
        let total = deaths_total.get(&(map.clone(), *sd)).copied().unwrap_or(0);
        if *n >= 5
            && total > 0
            && *n * 100 >= total * 35
            && !first
                .get(&(map.clone(), *sd, spot.clone()))
                .is_some_and(|f| f.0 >= 3)
        {
            notes.push((*n * 2, json!({ "kind": "death_spot", "map": map, "side": sd,
                "text": format!("{n} of your {total} {} deaths on {map} were near {spot}.", side_name(sd)),
                "refs": refs.iter().take(12).collect::<Vec<_>>() })));
        }
    }
    let (on, ow) = opening.values().fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
    if on >= 20 {
        let overall = 100 * ow / on;
        for (map, (n, w)) in &opening {
            let p = 100 * w / n.max(&1);
            if *n >= 8 && (p + 15 <= overall || p >= overall + 15) {
                let better = p > overall;
                notes.push((*n, json!({ "kind": "opening", "map": map,
                    "text": format!("You win {p}% of your opening duels on {map} ({w} of {n}), against {overall}% everywhere else{}.", if better { "" } else { ". Worth a look at how you take first fights there" }),
                    "refs": [] })));
            }
        }
    }
    notes.sort_by_key(|(w, _)| -w);
    json!({ "matches": data.len(), "notes": notes.into_iter().take(8).map(|(_, v)| v).collect::<Vec<_>>() })
}

/// Duo synergy: results per agent pair with each person you queue with.
pub fn duos(c: &Connection, me: &Me) -> Value {
    let data = load_rows(c, me, "", "", 400);
    // friend -> (name, games, wins, pairs (mine, theirs) -> (games, wins))
    type Pairs = HashMap<(String, String), (i64, i64)>;
    let mut fr: HashMap<String, (String, i64, i64, Pairs)> = HashMap::new();
    for (r, m) in &data {
        if r.party.is_empty() {
            continue;
        }
        let won = r.won();
        for p in arr(m, "/players") {
            let pu = s(p, "/puuid");
            if pu == me.puuid || s(p, "/party_id") != r.party {
                continue;
            }
            let e = fr.entry(pu.to_string()).or_insert((
                s(p, "/name").to_string(),
                0,
                0,
                HashMap::new(),
            ));
            e.1 += 1;
            e.2 += won as i64;
            let pe =
                e.3.entry((r.agent.clone(), s(p, "/agent/name").to_string()))
                    .or_default();
            pe.0 += 1;
            pe.1 += won as i64;
        }
    }
    let mut out: Vec<Value> = fr
        .into_iter()
        .filter(|(_, v)| v.1 >= 2)
        .map(|(pu, (name, g, w, pairs))| {
            let mut ps: Vec<Value> = pairs
                .into_iter()
                .map(|((a, b), (pg, pw))| json!({ "mine": a, "theirs": b, "games": pg, "wins": pw, "win": 100 * pw / pg.max(1) }))
                .collect();
            ps.sort_by_key(|v| -v["games"].as_i64().unwrap_or(0));
            ps.truncate(6);
            json!({ "puuid": pu, "name": name, "games": g, "wins": w, "win": 100 * w / g.max(1), "pairs": ps })
        })
        .collect();
    out.sort_by_key(|v| -v["games"].as_i64().unwrap_or(0));
    json!(out)
}

/// Three plain facts about one match, plus your ability casts per round against your average on
/// that agent.
pub fn match_extras(c: &Connection, kb: &Kb, me: &Me, id: &str) -> Value {
    let Ok(raw) = load_raw(c, id) else {
        return Value::Null;
    };
    let m = unwrap_data(&raw).clone();
    let one = crate::career::compute(std::slice::from_ref(&m), &me.puuid, "");
    let o = &one["overall"];
    if o["games"].as_i64().unwrap_or(0) == 0 {
        return Value::Null;
    }
    let map = s(&m, "/metadata/map/name").to_string();
    let agent = one["agents"][0]["name"].as_str().unwrap_or("").to_string();
    let mut facts: Vec<(i64, String)> = vec![];
    // Best ACS in recent matches.
    let recent = my_rows(
        c,
        &me.puuid,
        &Filter {
            limit: 20,
            ..Default::default()
        },
    );
    let acs = o["acs"].as_f64().unwrap_or(0.0);
    let better = recent
        .iter()
        .filter(|r| r.id != id && r.rounds > 0 && (r.score as f64 / r.rounds as f64) > acs)
        .count();
    if recent.len() >= 5 && better == 0 {
        facts.push((
            9,
            format!(
                "Your best ACS ({acs}) in your last {} matches.",
                recent.len()
            ),
        ));
    } else if recent.len() >= 5 && better <= 2 {
        facts.push((
            5,
            format!("Top 3 ACS of your last {} matches.", recent.len()),
        ));
    }
    let (fb, fd) = (o["fb"].as_i64().unwrap_or(0), o["fd"].as_i64().unwrap_or(0));
    if fb + fd >= 3 {
        facts.push((
            if fb > fd { 7 } else { 6 },
            format!("You won {fb} of {} opening duels.", fb + fd),
        ));
    }
    for (k, label, w) in [("aces", "ace", 10), ("k4", "4-kill round", 8)] {
        let n = o[k].as_i64().unwrap_or(0);
        if n > 0 {
            facts.push((w, format!("{n} {label}{}.", if n > 1 { "s" } else { "" })));
        }
    }
    let cl = o["clutches"].as_i64().unwrap_or(0);
    if cl > 0 {
        facts.push((
            8,
            format!(
                "Won {cl} of {} clutches.",
                o["clutch_att"].as_i64().unwrap_or(0)
            ),
        ));
    }
    // Where you died most.
    let mi = kb.map(&map);
    let mut spots: HashMap<String, i64> = HashMap::new();
    for k in arr(&m, "/kills")
        .iter()
        .filter(|k| s(k, "/victim/puuid") == me.puuid)
    {
        if let Some((x, y)) = loc(k, "/location") {
            *spots.entry(callout(mi, x, y)).or_default() += 1;
        }
    }
    if let Some((spot, n)) = spots
        .into_iter()
        .filter(|(k, _)| !k.is_empty())
        .max_by_key(|x| x.1)
    {
        if n >= 3 {
            facts.push((7, format!("{n} deaths near {spot}.")));
        }
    }
    let d = o["d"].as_i64().unwrap_or(0);
    if d >= 5 {
        let tp = o["traded_deaths"].as_f64().unwrap_or(0.0);
        facts.push((
            if tp < 30.0 { 6 } else { 3 },
            format!("{}% of your deaths were traded.", tp.round()),
        ));
    }
    facts.sort_by_key(|f| -f.0);
    // Ability usage vs your average on this agent (from your other stored matches).
    let mine: Vec<Value> = my_rows(
        c,
        &me.puuid,
        &Filter {
            agent: agent.clone(),
            limit: 60,
            ..Default::default()
        },
    )
    .iter()
    .filter(|r| r.id != id)
    .filter_map(|r| load_raw(c, &r.id).ok().map(|v| unwrap_data(&v).clone()))
    .collect();
    let avg = crate::career::compute(&mine, &me.puuid, "");
    json!({
        "facts": facts.into_iter().take(3).map(|f| f.1).collect::<Vec<_>>(),
        "abilities": { "agent": agent, "this": o["casts_per_round"], "avg": avg["overall"]["casts_per_round"], "games": avg["overall"]["games"] },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{fixture, store_match};

    fn setup() -> (Connection, Me, Kb) {
        let mut c = crate::db::open_memory().unwrap();
        for (n, won) in [(0, true), (1, false), (2, true)] {
            store_match(
                &mut c,
                &fixture::match_v4(&format!("m{n}"), won, &format!("2026-11-11T1{n}:00:00Z")),
            )
            .unwrap();
        }
        let me = Me {
            puuid: "p0".into(),
            name: "Player0".into(),
            tag: "t".into(),
            region: "ap".into(),
            account_level: 1,
        };
        (c, me, Kb::load())
    }

    #[test]
    fn callout_names() {
        let kb = Kb::load();
        let bind = kb.map("Bind").unwrap();
        assert!(!bind.callouts.is_empty(), "maps.json has callouts");
        let (sup, reg, x, y) = bind.callouts[0].clone();
        let name = callout(Some(bind), x + 1.0, y - 1.0);
        assert!(name.contains(&reg), "{name} should name {sup} {reg}");
        assert_eq!(callout(None, 0.0, 0.0), "");
    }

    #[test]
    fn heatmap_layers_and_events() {
        let (c, me, kb) = setup();
        let map = crate::parse::s(
            &fixture::match_v4("x", true, "2026-11-11T10:00:00Z"),
            "/data/metadata/map/name",
        )
        .to_string();
        let o = |event: &str, layer: &str| HeatOpts {
            map: map.clone(),
            event: event.into(),
            side: String::new(),
            layer: layer.into(),
            agent: String::new(),
            outcome: String::new(),
            rtype: String::new(),
            match_id: String::new(),
            queue: String::new(),
            since: 0,
        };
        // p0 dies once per match; p1 (teammate) gets one kill per match.
        assert_eq!(
            heatmap(&c, &kb, &me, &o("deaths", "me"))["points"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert_eq!(
            heatmap(&c, &kb, &me, &o("kills", "me"))["points"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        assert_eq!(
            heatmap(&c, &kb, &me, &o("kills", "team"))["points"]
                .as_array()
                .unwrap()
                .len(),
            0,
            "p1 has no recorded position at its kill"
        );
        assert_eq!(
            heatmap(&c, &kb, &me, &o("kills", "enemy"))["points"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        let op = heatmap(&c, &kb, &me, &o("opening", "me"));
        assert_eq!(op["points"][0][2], false, "p0 lost the opening duel");
        let mut one = o("deaths", "me");
        one.match_id = "m1".into();
        let h = heatmap(&c, &kb, &me, &one);
        assert_eq!(h["matches"], 1, "match filter keeps one match");
        assert_eq!(h["points"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn trades_partners_and_duos() {
        let (c, me, _) = setup();
        let t = trades(&c, &me);
        assert_eq!(t["deaths"], 3);
        assert_eq!(t["traded"], 3);
        assert_eq!(t["avg_trade_ms"], 1500);
        assert_eq!(t["partners"][0]["name"], "Player1");
        let d = duos(&c, &me);
        assert_eq!(d[0]["name"], "Player1");
        assert_eq!(d[0]["games"], 3);
        assert_eq!(d[0]["pairs"][0]["mine"], "Omen");
    }

    #[test]
    fn extras_have_abilities_and_patterns_run() {
        let (c, me, kb) = setup();
        let x = match_extras(&c, &kb, &me, "m0");
        assert!(x["facts"].is_array());
        assert_eq!(x["abilities"]["agent"], "Omen");
        assert_eq!(x["abilities"]["games"], 2);
        assert!(patterns(&c, &kb, &me)["notes"].is_array());
    }
}
