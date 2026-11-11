//! Lifetime / career stats for any player, computed from full v4 match objects.
//! Used for you (every match stored on this PC) and for searched players (their recent matches,
//! fetched on request). Pure function: no I/O, so it is easy to test.
use crate::parse::{arr, i, s, side, unwrap_data};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

#[derive(Default, Clone)]
struct Acc {
    games: i64,
    wins: i64,
    losses: i64,
    rounds: i64,
    k: i64,
    d: i64,
    a: i64,
    score: i64,
    dmg: i64,
    dmg_recv: i64,
    hs: i64,
    body: i64,
    leg: i64,
    fb: i64,
    fd: i64,
    kast_rounds: i64,
    plants: i64,
    defuses: i64,
    atk_rounds: i64,
    def_rounds: i64,
    atk_won: i64,
    def_won: i64,
    def_planted_rounds: i64,
    post_plant_kills: i64,
    retake_kills: i64,
    clutch_att: i64,
    clutch_won: i64,
    k2: i64,
    k3: i64,
    k4: i64,
    aces: i64,
    spent: i64,
    trades: i64,
    traded_deaths: i64,
    casts: [i64; 4],
}

impl Acc {
    fn pct(a: i64, b: i64) -> Value {
        if b > 0 {
            json!(((1000 * a) as f64 / b as f64).round() / 10.0)
        } else {
            Value::Null
        }
    }
    fn ratio(a: i64, b: i64) -> f64 {
        if b > 0 {
            ((100 * a) as f64 / b as f64).round() / 100.0
        } else {
            a as f64
        }
    }
    fn json(&self) -> Value {
        let shots = self.hs + self.body + self.leg;
        let r = self.rounds.max(1) as f64;
        json!({
            "games": self.games, "wins": self.wins, "losses": self.losses, "draws": self.games - self.wins - self.losses,
            "win": Acc::pct(self.wins, self.games), "rounds": self.rounds,
            "k": self.k, "d": self.d, "a": self.a, "kd": Acc::ratio(self.k, self.d), "kda": Acc::ratio(self.k + self.a, self.d),
            "kpr": ((self.k as f64 / r) * 100.0).round() / 100.0, "dpr": ((self.d as f64 / r) * 100.0).round() / 100.0,
            "acs": (self.score as f64 / r).round(), "adr": (self.dmg as f64 / r).round(), "dd": ((self.dmg - self.dmg_recv) as f64 / r).round(),
            "hs": Acc::pct(self.hs, shots), "kast": Acc::pct(self.kast_rounds, self.rounds),
            "fb": self.fb, "fd": self.fd, "fb_win": Acc::pct(self.fb, self.fb + self.fd),
            "plants": self.plants, "defuses": self.defuses,
            "plant_rate": Acc::pct(self.plants, self.atk_rounds), "defuse_rate": Acc::pct(self.defuses, self.def_planted_rounds),
            "post_plant_kills": self.post_plant_kills, "retake_kills": self.retake_kills,
            "atk_win": Acc::pct(self.atk_won, self.atk_rounds), "def_win": Acc::pct(self.def_won, self.def_rounds),
            "clutches": self.clutch_won, "clutch_att": self.clutch_att, "clutch_win": Acc::pct(self.clutch_won, self.clutch_att),
            "k2": self.k2, "k3": self.k3, "k4": self.k4, "aces": self.aces,
            "dmg_per_1k": if self.spent > 0 { json!((self.dmg as f64 * 1000.0 / self.spent as f64).round()) } else { Value::Null },
            "trades": self.trades, "traded_deaths": Acc::pct(self.traded_deaths, self.d),
            "casts_per_round": {
                "grenade": ((self.casts[0] as f64 / r) * 100.0).round() / 100.0, "ability1": ((self.casts[1] as f64 / r) * 100.0).round() / 100.0,
                "ability2": ((self.casts[2] as f64 / r) * 100.0).round() / 100.0, "ultimate": ((self.casts[3] as f64 / r) * 100.0).round() / 100.0,
            },
        })
    }
    fn add(&mut self, o: &Acc) {
        macro_rules! sum { ($($f:ident),*) => { $( self.$f += o.$f; )* } }
        sum!(
            games,
            wins,
            losses,
            rounds,
            k,
            d,
            a,
            score,
            dmg,
            dmg_recv,
            hs,
            body,
            leg,
            fb,
            fd,
            kast_rounds,
            plants,
            defuses,
            atk_rounds,
            def_rounds,
            atk_won,
            def_won,
            def_planted_rounds,
            post_plant_kills,
            retake_kills,
            clutch_att,
            clutch_won,
            k2,
            k3,
            k4,
            aces,
            spent,
            trades,
            traded_deaths
        );
        for n in 0..4 {
            self.casts[n] += o.casts[n];
        }
    }
}

const TRADE_MS: i64 = 5000;

/// One match from the player's point of view. None if they weren't in it.
#[allow(clippy::type_complexity)]
fn one(
    m: &Value,
    me: &str,
) -> Option<(
    Acc,
    String,
    String,
    String,
    String,
    Vec<(String, i64, i64)>,
    String,
    char,
)> {
    let m = unwrap_data(m);
    let players = arr(m, "/players");
    let p = players.iter().find(|p| s(p, "/puuid") == me)?;
    let team = s(p, "/team_id").to_string();
    let team_of: HashMap<&str, &str> = players
        .iter()
        .map(|p| (s(p, "/puuid"), s(p, "/team_id")))
        .collect();
    let rounds = arr(m, "/rounds");
    let kills = arr(m, "/kills");
    let mut a = Acc {
        games: 1,
        ..Default::default()
    };
    let teams = arr(m, "/teams");
    let won_rounds = |t: &str| {
        teams
            .iter()
            .find(|x| s(x, "/team_id").eq_ignore_ascii_case(t))
            .map(|x| i(x, "/rounds/won"))
            .unwrap_or(0)
    };
    let mine = won_rounds(&team);
    let theirs: i64 = teams
        .iter()
        .filter(|x| !s(x, "/team_id").eq_ignore_ascii_case(&team))
        .map(|x| i(x, "/rounds/won"))
        .max()
        .unwrap_or(0);
    let result = if mine > theirs {
        'W'
    } else if mine < theirs {
        'L'
    } else {
        'D'
    };
    if result == 'W' {
        a.wins = 1;
    } else if result == 'L' {
        a.losses = 1;
    }
    a.rounds = rounds.len() as i64;
    a.k = i(p, "/stats/kills");
    a.d = i(p, "/stats/deaths");
    a.a = i(p, "/stats/assists");
    a.score = i(p, "/stats/score");
    a.dmg = i(p, "/stats/damage/dealt");
    a.dmg_recv = i(p, "/stats/damage/received");
    a.hs = i(p, "/stats/headshots");
    a.body = i(p, "/stats/bodyshots");
    a.leg = i(p, "/stats/legshots");
    a.casts = [
        i(p, "/ability_casts/grenade"),
        i(p, "/ability_casts/ability1"),
        i(p, "/ability_casts/ability2"),
        i(p, "/ability_casts/ultimate"),
    ];
    let mut weapons: HashMap<String, (i64, i64)> = HashMap::new();
    let mut by_round: HashMap<i64, Vec<&Value>> = HashMap::new();
    for k in kills {
        by_round.entry(i(k, "/round")).or_default().push(k);
    }
    for (n, r) in rounds.iter().enumerate() {
        let n = n as i64;
        let sd = side(&team, n);
        let won = s(r, "/winning_team").eq_ignore_ascii_case(&team);
        let planted = !s(r, "/plant/site").is_empty()
            || r.pointer("/plant/round_time_in_ms")
                .is_some_and(|v| !v.is_null());
        let plant_t = r
            .pointer("/plant/round_time_in_ms")
            .and_then(|v| v.as_i64());
        if sd == "atk" {
            a.atk_rounds += 1;
            a.atk_won += won as i64;
        } else {
            a.def_rounds += 1;
            a.def_won += won as i64;
            if planted {
                a.def_planted_rounds += 1;
            }
        }
        if s(r, "/plant/player/puuid") == me {
            a.plants += 1;
        }
        if s(r, "/defuse/player/puuid") == me {
            a.defuses += 1;
        }
        if let Some(st) = arr(r, "/stats")
            .iter()
            .find(|x| s(x, "/player/puuid") == me)
        {
            a.spent += i(st, "/economy/spent");
        }
        let mut ks: Vec<&Value> = by_round.get(&n).cloned().unwrap_or_default();
        ks.sort_by_key(|k| i(k, "/time_in_round_in_ms"));
        // Opening duel.
        if let Some(first) = ks.first() {
            if s(first, "/killer/puuid") == me {
                a.fb += 1;
            } else if s(first, "/victim/puuid") == me {
                a.fd += 1;
            }
        }
        let mut my_kills = 0;
        let mut kast = false;
        let mut died_at: Option<(i64, String)> = None;
        for k in &ks {
            let t = i(k, "/time_in_round_in_ms");
            let killer = s(k, "/killer/puuid");
            let victim = s(k, "/victim/puuid");
            if killer == me && victim != me {
                my_kills += 1;
                kast = true;
                let w = s(k, "/weapon/name");
                let e = weapons
                    .entry(if w.is_empty() {
                        "Ability".into()
                    } else {
                        w.to_string()
                    })
                    .or_default();
                e.0 += 1;
                if let Some(pt) = plant_t {
                    if t > pt {
                        if sd == "atk" {
                            a.post_plant_kills += 1;
                        } else {
                            a.retake_kills += 1;
                        }
                    }
                }
                // Trade: my kill on someone who killed a teammate in the last 5 s.
                if ks.iter().any(|x| {
                    s(x, "/killer/puuid") == victim
                        && team_of.get(s(x, "/victim/puuid")) == Some(&team.as_str())
                        && s(x, "/victim/puuid") != me
                        && (0..=TRADE_MS).contains(&(t - i(x, "/time_in_round_in_ms")))
                }) {
                    a.trades += 1;
                }
            }
            if arr(k, "/assistants")
                .iter()
                .any(|x| s(x, "/puuid") == me || s(x, "/assistant/puuid") == me)
            {
                kast = true;
            }
            if victim == me {
                died_at = Some((t, killer.to_string()));
            }
        }
        match &died_at {
            None => kast = true,
            Some((t, killer)) => {
                let traded = ks.iter().any(|x| {
                    s(x, "/victim/puuid") == killer
                        && team_of.get(s(x, "/killer/puuid")) == Some(&team.as_str())
                        && (0..=TRADE_MS).contains(&(i(x, "/time_in_round_in_ms") - t))
                });
                if traded {
                    kast = true;
                    a.traded_deaths += 1;
                }
            }
        }
        a.kast_rounds += kast as i64;
        match my_kills {
            2 => a.k2 += 1,
            3 => a.k3 += 1,
            4 => a.k4 += 1,
            k if k >= 5 => a.aces += 1,
            _ => {}
        }
        // Clutch: I'm the last one alive on my team with at least one enemy alive.
        let mut alive: HashSet<&str> = players.iter().map(|p| s(p, "/puuid")).collect();
        let mut clutch = false;
        let check = |alive: &HashSet<&str>| {
            let mates = alive
                .iter()
                .filter(|p| team_of.get(*p) == Some(&team.as_str()))
                .count();
            let foes = alive
                .iter()
                .filter(|p| team_of.get(*p).is_some_and(|t| *t != team))
                .count();
            mates == 1 && alive.contains(me) && foes >= 1
        };
        for k in &ks {
            alive.remove(s(k, "/victim/puuid"));
            if !clutch && check(&alive) {
                clutch = true;
            }
        }
        if clutch {
            a.clutch_att += 1;
            a.clutch_won += won as i64;
        }
    }
    let wv: Vec<(String, i64, i64)> = weapons.into_iter().map(|(k, v)| (k, v.0, v.1)).collect();
    Some((
        a,
        s(p, "/agent/name").to_string(),
        s(p, "/agent/id").to_string(),
        s(m, "/metadata/map/name").to_string(),
        {
            let q = s(m, "/metadata/queue/id");
            if q.is_empty() {
                s(m, "/metadata/queue/name").to_lowercase()
            } else {
                q.to_lowercase()
            }
        },
        wv,
        s(m, "/metadata/game_version").to_string(),
        result,
    ))
}

/// "release-13.08-shipping-…" → "13.08".
pub fn patch_of(v: &str) -> String {
    v.split('-')
        .find(|x| x.chars().next().is_some_and(|c| c.is_ascii_digit()) && x.contains('.'))
        .unwrap_or("")
        .to_string()
}

/// Full career for `puuid` over `matches` (any order). `queue` filters ("" = all).
pub fn compute(matches: &[Value], puuid: &str, queue: &str) -> Value {
    let mut total = Acc::default();
    let mut agents: HashMap<String, (String, Acc)> = HashMap::new();
    let mut maps: HashMap<String, Acc> = HashMap::new();
    let mut combos: HashMap<(String, String), Acc> = HashMap::new();
    let mut queues: HashMap<String, Acc> = HashMap::new();
    let mut patches: HashMap<String, Acc> = HashMap::new();
    let mut weapons: HashMap<String, i64> = HashMap::new();
    let mut form: Vec<(i64, char, String, i64)> = vec![];
    for m in matches {
        let Some((a, agent, agent_id, map, q, wv, ver, result)) = one(m, puuid) else {
            continue;
        };
        queues.entry(q.clone()).or_default().add(&a);
        if !queue.is_empty() && q != queue {
            continue;
        }
        total.add(&a);
        agents
            .entry(agent.clone())
            .or_insert_with(|| (agent_id.clone(), Acc::default()))
            .1
            .add(&a);
        maps.entry(map.clone()).or_default().add(&a);
        combos
            .entry((agent.clone(), map.clone()))
            .or_default()
            .add(&a);
        let p = patch_of(&ver);
        if !p.is_empty() {
            patches.entry(p).or_default().add(&a);
        }
        for (w, k, _) in wv {
            *weapons.entry(w).or_default() += k;
        }
        let mm = unwrap_data(m);
        let t = crate::parse::time_ms(mm.pointer("/metadata/started_at").unwrap_or(&Value::Null));
        let acs = if a.rounds > 0 { a.score / a.rounds } else { 0 };
        form.push((t, result, agent.clone(), acs));
    }
    form.sort_by_key(|x| -x.0);
    let mut ag: Vec<Value> = agents
        .into_iter()
        .map(|(name, (id, a))| {
            let mut v = a.json();
            v["name"] = json!(name);
            v["id"] = json!(id);
            v
        })
        .collect();
    ag.sort_by_key(|v| -v["games"].as_i64().unwrap_or(0));
    let mut mp: Vec<Value> = maps
        .into_iter()
        .map(|(name, a)| {
            let mut v = a.json();
            v["name"] = json!(name);
            v
        })
        .collect();
    mp.sort_by_key(|v| -v["games"].as_i64().unwrap_or(0));
    let mut cb: Vec<Value> = combos
        .into_iter()
        .map(|((agent, map), a)| {
            let mut v = a.json();
            v["agent"] = json!(agent);
            v["map"] = json!(map);
            v
        })
        .collect();
    cb.sort_by_key(|v| -v["games"].as_i64().unwrap_or(0));
    let mut qs: Vec<Value> = queues
        .into_iter()
        .map(
            |(name, a)| json!({ "name": name, "games": a.games, "win": Acc::pct(a.wins, a.games) }),
        )
        .collect();
    qs.sort_by_key(|v| -v["games"].as_i64().unwrap_or(0));
    let mut pt: Vec<Value> = patches
        .into_iter()
        .map(|(name, a)| {
            let mut v = a.json();
            v["name"] = json!(name);
            v
        })
        .collect();
    pt.sort_by(|a, b| b["name"].as_str().cmp(&a["name"].as_str()));
    let wk: i64 = weapons.values().sum::<i64>().max(1);
    let mut wp: Vec<Value> = weapons
        .into_iter()
        .map(|(n, k)| json!({ "name": n, "kills": k, "share": (100 * k / wk) }))
        .collect();
    wp.sort_by_key(|v| -v["kills"].as_i64().unwrap_or(0));
    wp.truncate(12);
    json!({
        "overall": total.json(), "agents": ag, "maps": mp, "combos": cb, "queues": qs, "patches": pt, "weapons": wp,
        "form": form.iter().take(20).map(|(t, r, a, acs)| json!({ "at": t, "result": r.to_string(), "agent": a, "acs": acs })).collect::<Vec<_>>(),
    })
}

/// Cheap summary from HenrikDev "stored matches" rows (no rounds or kill events): per agent / map.
pub fn from_stored(rows: &[Value]) -> Value {
    let mut total = Acc::default();
    let mut agents: HashMap<String, (String, Acc)> = HashMap::new();
    let mut maps: HashMap<String, Acc> = HashMap::new();
    for m in rows {
        let team = s(m, "/stats/team").to_lowercase();
        let (mine, theirs) = if team == "red" {
            (i(m, "/teams/red"), i(m, "/teams/blue"))
        } else {
            (i(m, "/teams/blue"), i(m, "/teams/red"))
        };
        let a = Acc {
            games: 1,
            wins: (mine > theirs) as i64,
            losses: (mine < theirs) as i64,
            rounds: (mine + theirs).max(1),
            k: i(m, "/stats/kills"),
            d: i(m, "/stats/deaths"),
            a: i(m, "/stats/assists"),
            score: i(m, "/stats/score"),
            dmg: i(m, "/stats/damage/made"),
            dmg_recv: i(m, "/stats/damage/received"),
            hs: i(m, "/stats/shots/head"),
            body: i(m, "/stats/shots/body"),
            leg: i(m, "/stats/shots/leg"),
            ..Default::default()
        };
        total.add(&a);
        agents
            .entry(s(m, "/stats/character/name").to_string())
            .or_insert_with(|| (s(m, "/stats/character/id").to_string(), Acc::default()))
            .1
            .add(&a);
        maps.entry(s(m, "/meta/map/name").to_string())
            .or_default()
            .add(&a);
    }
    let mut ag: Vec<Value> = agents
        .into_iter()
        .map(|(n, (id, a))| {
            let mut v = a.json();
            v["name"] = json!(n);
            v["id"] = json!(id);
            v
        })
        .collect();
    ag.sort_by_key(|v| -v["games"].as_i64().unwrap_or(0));
    let mut mp: Vec<Value> = maps
        .into_iter()
        .map(|(n, a)| {
            let mut v = a.json();
            v["name"] = json!(n);
            v
        })
        .collect();
    mp.sort_by_key(|v| -v["games"].as_i64().unwrap_or(0));
    json!({ "overall": total.json(), "agents": ag, "maps": mp })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::fixture::match_v4;

    #[test]
    fn career_from_fixture() {
        let m = match_v4("m-1", true, "2026-11-11T10:00:00Z");
        let c = compute(std::slice::from_ref(&m), "p1", "");
        assert_eq!(c["overall"]["games"], 1);
        assert_eq!(c["overall"]["wins"], 1);
        assert_eq!(c["agents"].as_array().unwrap().len(), 1);
        assert!(c["overall"]["kast"].as_f64().is_some());
        // p1 traded p0's death 1.5 s later.
        assert_eq!(c["overall"]["trades"], 1);
        let p0 = compute(std::slice::from_ref(&m), "p0", "");
        assert!(p0["overall"]["fd"].as_i64().unwrap() >= 1);
        // One of p0's deaths in the fixture was traded.
        assert!(p0["overall"]["traded_deaths"].as_f64().unwrap() > 0.0);
        let none = compute(&[m], "nobody", "");
        assert_eq!(none["overall"]["games"], 0);
    }

    #[test]
    fn patch_parse() {
        assert_eq!(patch_of("release-13.08-shipping-12-123456"), "13.08");
        assert_eq!(patch_of(""), "");
    }
}
