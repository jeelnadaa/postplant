//! Turns a HenrikDev v4 match into rows. Field access is defensive: every field is optional,
//! so a missing field leaves a blank instead of failing the whole match (verify in docs/api-notes.md).
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};
use serde_json::Value;

pub fn s<'a>(v: &'a Value, p: &str) -> &'a str {
    v.pointer(p).and_then(|x| x.as_str()).unwrap_or("")
}
pub fn i(v: &Value, p: &str) -> i64 {
    v.pointer(p)
        .and_then(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64)))
        .unwrap_or(0)
}
pub fn f(v: &Value, p: &str) -> Option<f64> {
    v.pointer(p).and_then(|x| x.as_f64())
}
pub fn arr<'a>(v: &'a Value, p: &str) -> &'a [Value] {
    v.pointer(p)
        .and_then(|x| x.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[])
}

/// "2026-11-11T14:12:03.000Z" or epoch seconds → epoch ms.
pub fn time_ms(v: &Value) -> i64 {
    if let Some(n) = v.as_i64() {
        return if n < 10_000_000_000 { n * 1000 } else { n };
    }
    v.as_str()
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|d| d.timestamp_millis())
        .unwrap_or(0)
}

/// The match payload may be wrapped in {status, data}.
pub fn unwrap_data(v: &Value) -> &Value {
    v.get("data").unwrap_or(v)
}

/// Which side a team is on in a round (0-based). Red attacks the first half; overtime alternates.
pub fn side(team: &str, round: i64) -> &'static str {
    let red_atk = if round < 12 {
        true
    } else if round < 24 {
        false
    } else {
        (round - 24) % 2 == 0
    };
    let is_red = team.eq_ignore_ascii_case("red");
    if is_red == red_atk {
        "atk"
    } else {
        "def"
    }
}

pub fn store_match(conn: &mut Connection, raw: &Value) -> Result<String> {
    let m = unwrap_data(raw);
    let id = s(m, "/metadata/match_id").to_string();
    if id.is_empty() {
        return Err(anyhow!("match has no id"));
    }
    let started = time_ms(m.pointer("/metadata/started_at").unwrap_or(&Value::Null));
    let teams = arr(m, "/teams");
    let score = |t: &str| {
        teams
            .iter()
            .find(|x| s(x, "/team_id").eq_ignore_ascii_case(t))
            .map(|x| i(x, "/rounds/won"))
            .unwrap_or(0)
    };
    let winner = teams
        .iter()
        .find(|x| x.get("won").and_then(|w| w.as_bool()) == Some(true))
        .map(|x| s(x, "/team_id").to_string())
        .unwrap_or_default();
    let rounds = arr(m, "/rounds");
    let zipped = zstd::encode_all(m.to_string().as_bytes(), 9)?;
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT OR REPLACE INTO matches(match_id, started_at, length_ms, map_id, map_name, queue, season_id, season_short,
          game_version, region, rounds_played, red_score, blue_score, winning_team, raw_json_zstd)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        params![
            id, started, i(m, "/metadata/game_length_in_ms"), s(m, "/metadata/map/id"), s(m, "/metadata/map/name"),
            { let q = s(m, "/metadata/queue/id"); if q.is_empty() { s(m, "/metadata/queue/name") } else { q } },
            s(m, "/metadata/season/id"), s(m, "/metadata/season/short"), s(m, "/metadata/game_version"),
            s(m, "/metadata/region"), rounds.len() as i64, score("Red"), score("Blue"), winner, zipped
        ],
    )?;
    for p in arr(m, "/players") {
        let puuid = s(p, "/puuid");
        if puuid.is_empty() {
            continue;
        }
        tx.execute(
            "INSERT OR REPLACE INTO match_players(match_id, puuid, name, tag, team, party_id, agent_id, agent_name, score,
              kills, deaths, assists, headshots, bodyshots, legshots, damage_made, damage_received, rank_tier,
              account_level, ability_casts_json) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
            params![
                id, puuid, s(p, "/name"), s(p, "/tag"), s(p, "/team_id"), s(p, "/party_id"), s(p, "/agent/id"),
                s(p, "/agent/name"), i(p, "/stats/score"), i(p, "/stats/kills"), i(p, "/stats/deaths"),
                i(p, "/stats/assists"), i(p, "/stats/headshots"), i(p, "/stats/bodyshots"), i(p, "/stats/legshots"),
                i(p, "/stats/damage/dealt"), i(p, "/stats/damage/received"), i(p, "/tier/id"), i(p, "/account_level"),
                p.get("ability_casts").map(|a| a.to_string()).unwrap_or_default()
            ],
        )?;
        tx.execute(
            "INSERT INTO players(puuid, name, tag, region, card_id, account_level, last_seen_at) VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(puuid) DO UPDATE SET name=excluded.name, tag=excluded.tag, account_level=excluded.account_level,
               last_seen_at=MAX(COALESCE(players.last_seen_at,0), excluded.last_seen_at)",
            params![puuid, s(p, "/name"), s(p, "/tag"), s(m, "/metadata/region"), s(p, "/customization/card"), i(p, "/account_level"), started],
        )?;
    }
    for (n, r) in rounds.iter().enumerate() {
        let n = n as i64;
        tx.execute(
            "INSERT OR REPLACE INTO rounds(match_id, round_no, winning_team, result_code, planter, defuser, plant_site)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id, n, s(r, "/winning_team"), s(r, "/result"), s(r, "/plant/player/puuid"), s(r, "/defuse/player/puuid"), s(r, "/plant/site")],
        )?;
        for st in arr(r, "/stats") {
            let dmg: i64 = arr(st, "/damage_events")
                .iter()
                .map(|d| i(d, "/damage"))
                .sum();
            tx.execute(
                "INSERT OR REPLACE INTO round_player_stats(match_id, round_no, puuid, score, damage, kills, headshots,
                  bodyshots, legshots, econ_spent, loadout_value) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    id, n, s(st, "/player/puuid"), i(st, "/stats/score"), dmg, i(st, "/stats/kills"),
                    i(st, "/stats/headshots"), i(st, "/stats/bodyshots"), i(st, "/stats/legshots"),
                    i(st, "/economy/spent"), i(st, "/economy/loadout_value")
                ],
            )?;
        }
    }
    for k in arr(m, "/kills") {
        let killer = s(k, "/killer/puuid");
        let (kx, ky) = arr(k, "/player_locations")
            .iter()
            .find(|l| s(l, "/player/puuid") == killer)
            .map(|l| (f(l, "/location/x"), f(l, "/location/y")))
            .unwrap_or((None, None));
        tx.execute(
            "INSERT OR REPLACE INTO kills(match_id, round_no, time_in_round_ms, time_in_match_ms, killer, victim, weapon_id,
              weapon_name, killer_x, killer_y, victim_x, victim_y) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                id, i(k, "/round"), i(k, "/time_in_round_in_ms"), i(k, "/time_in_match_in_ms"), killer, s(k, "/victim/puuid"),
                s(k, "/weapon/id"), s(k, "/weapon/name"), kx, ky, f(k, "/location/x"), f(k, "/location/y")
            ],
        )?;
    }
    tx.commit()?;
    Ok(id)
}

pub fn load_raw(conn: &Connection, id: &str) -> Result<Value> {
    let blob: Vec<u8> = conn.query_row(
        "SELECT raw_json_zstd FROM matches WHERE match_id=?1",
        [id],
        |r| r.get(0),
    )?;
    let bytes = zstd::decode_all(blob.as_slice())?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
pub mod fixture {
    use serde_json::{json, Value};
    /// A small but complete v4-shaped match used by tests across modules.
    pub fn match_v4(id: &str, me_won: bool, started: &str) -> Value {
        let players: Vec<Value> = (0..10)
            .map(|n| {
                let team = if n < 5 { "Red" } else { "Blue" };
                json!({"puuid": format!("p{n}"), "name": format!("Player{n}"), "tag": "t", "team_id": team,
                  "party_id": if n < 2 { "party-a" } else { "solo" }, "agent": {"id": format!("a{n}"), "name": (["Omen","Jett","Sova","Killjoy","Raze","Viper","Reyna","Fade","Cypher","Neon"][n])},
                  "stats": {"score": 4000 + n * 100, "kills": 15 + n, "deaths": 14, "assists": 5, "headshots": 20, "bodyshots": 50, "legshots": 5, "damage": {"dealt": 3000, "received": 2800}},
                  "tier": {"id": 19, "name": "Diamond 2"}, "account_level": 100 + n, "ability_casts": {"grenade": 10}})
            })
            .collect();
        let (rw, bw) = if me_won { (13, 9) } else { (9, 13) };
        let rounds: Vec<Value> = (0..22)
            .map(|r| json!({"result": "Elimination", "winning_team": if r % 2 == 0 { "Red" } else { "Blue" },
                "plant": null, "defuse": null,
                "stats": [{"player": {"puuid": "p0"}, "stats": {"score": 200, "kills": 1, "headshots": 1, "bodyshots": 1, "legshots": 0},
                           "economy": {"loadout_value": if r == 0 || r == 12 { 800 } else { 4200 }, "spent": 3000},
                           "damage_events": [{"damage": 140}]}]}))
            .collect();
        let kills = vec![
            json!({"round": 0, "time_in_round_in_ms": 20000, "time_in_match_in_ms": 60000, "killer": {"puuid": "p5"}, "victim": {"puuid": "p0"},
                   "weapon": {"id": "w", "name": "Vandal"}, "location": {"x": 10.0, "y": 20.0},
                   "player_locations": [{"player": {"puuid": "p5"}, "location": {"x": 0.0, "y": 0.0}}, {"player": {"puuid": "p1"}, "location": {"x": 10.0, "y": 2520.0}}]}),
            json!({"round": 0, "time_in_round_in_ms": 21500, "time_in_match_in_ms": 61500, "killer": {"puuid": "p1"}, "victim": {"puuid": "p5"},
                   "weapon": {"id": "w", "name": "Phantom"}, "location": {"x": 0.0, "y": 0.0}, "player_locations": []}),
        ];
        json!({"data": {"metadata": {"match_id": id, "map": {"id": "m1", "name": "Lotus"}, "game_length_in_ms": 2400000,
            "started_at": started, "queue": {"id": "competitive"}, "season": {"id": "s1", "short": "e9a3"}, "region": "ap", "game_version": "13.09"},
            "players": players, "teams": [{"team_id": "Red", "rounds": {"won": rw, "lost": bw}, "won": me_won}, {"team_id": "Blue", "rounds": {"won": bw, "lost": rw}, "won": !me_won}],
            "rounds": rounds, "kills": kills}})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stores_a_match() {
        let mut c = crate::db::open_memory().unwrap();
        let id = store_match(
            &mut c,
            &fixture::match_v4("m-1", true, "2026-11-11T10:00:00Z"),
        )
        .unwrap();
        assert_eq!(id, "m-1");
        let n: i64 = c
            .query_row("SELECT COUNT(*) FROM match_players", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 10);
        let (red, winner): (i64, String) = c
            .query_row("SELECT red_score, winning_team FROM matches", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!((red, winner.as_str()), (13, "Red"));
        assert_eq!(
            load_raw(&c, "m-1")
                .unwrap()
                .pointer("/metadata/match_id")
                .unwrap(),
            "m-1"
        );
    }
    #[test]
    fn sides_swap_at_half_and_alternate_in_overtime() {
        assert_eq!(side("Red", 0), "atk");
        assert_eq!(side("Red", 12), "def");
        assert_eq!(side("Blue", 24), "def");
        assert_eq!(side("Blue", 25), "atk");
    }
}
