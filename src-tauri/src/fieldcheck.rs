//! Field check (plan M0): compare what HenrikDev actually returns with every field Postplant reads.
//! Uses your newest stored match (zero requests) plus one rank request. The result is shown in
//! Settings and can be copied into a bug report; it contains field names only, no values.
use crate::api::{paths, Prio};
use crate::parse::{arr, unwrap_data};
use crate::state::AppState;
use serde_json::{json, Value};

/// (group, base pointer into the match, fields under it)
const MATCH: &[(&str, &str, &[&str])] = &[
    (
        "Match",
        "",
        &[
            "/metadata/match_id",
            "/metadata/map/id",
            "/metadata/map/name",
            "/metadata/queue/id",
            "/metadata/game_version",
            "/metadata/started_at",
            "/metadata/game_length_in_ms",
            "/metadata/season/short",
            "/metadata/region",
        ],
    ),
    (
        "Player",
        "/players/0",
        &[
            "/puuid",
            "/name",
            "/tag",
            "/team_id",
            "/party_id",
            "/agent/id",
            "/agent/name",
            "/tier/id",
            "/account_level",
            "/stats/score",
            "/stats/kills",
            "/stats/deaths",
            "/stats/assists",
            "/stats/headshots",
            "/stats/bodyshots",
            "/stats/legshots",
            "/stats/damage/dealt",
            "/stats/damage/received",
            "/ability_casts/grenade",
            "/ability_casts/ability1",
            "/ability_casts/ability2",
            "/ability_casts/ultimate",
        ],
    ),
    ("Team", "/teams/0", &["/team_id", "/rounds/won"]),
    (
        "Round",
        "/rounds/0",
        &[
            "/winning_team",
            "/result",
            "/stats/0/player/puuid",
            "/stats/0/economy/loadout_value",
            "/stats/0/economy/spent",
        ],
    ),
    (
        "Kill",
        "/kills/0",
        &[
            "/round",
            "/time_in_round_in_ms",
            "/time_in_match_in_ms",
            "/killer/puuid",
            "/victim/puuid",
            "/weapon/name",
            "/location/x",
            "/location/y",
            "/player_locations/0/player/puuid",
            "/player_locations/0/location/x",
            "/assistants",
        ],
    ),
];
const MMR: &[&str] = &[
    "/current/tier/id",
    "/current/rr",
    "/current/elo",
    "/current/last_change",
    "/peak/tier/id",
    "/peak/season/short",
    "/seasonal/0/season/short",
    "/seasonal/0/end_tier/id",
    "/seasonal/0/wins",
    "/seasonal/0/games",
];

fn check(v: &Value, base: &str, fields: &[&str]) -> (Vec<String>, Vec<String>) {
    let (mut ok, mut missing) = (vec![], vec![]);
    for f in fields {
        if v.pointer(&format!("{base}{f}"))
            .is_some_and(|x| !x.is_null())
        {
            ok.push(f.to_string());
        } else {
            missing.push(f.to_string());
        }
    }
    (ok, missing)
}

pub async fn run(st: &AppState) -> Result<Value, String> {
    let me = st.me().ok_or("Set your Riot ID first.")?;
    let mut groups = vec![];
    let (m, plant_seen, locs) = {
        let c = st.db.lock().unwrap();
        let id: Option<String> = c
            .query_row(
                "SELECT match_id FROM matches ORDER BY started_at DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .ok();
        let Some(id) = id else {
            return Err("Sync at least one match first.".into());
        };
        let raw = crate::parse::load_raw(&c, &id).map_err(|e| e.to_string())?;
        let m = unwrap_data(&raw).clone();
        let plant = arr(&m, "/rounds")
            .iter()
            .any(|r| r.pointer("/plant/location/x").is_some_and(|x| !x.is_null()));
        let ks = arr(&m, "/kills");
        let with_locs = ks
            .iter()
            .filter(|k| !arr(k, "/player_locations").is_empty())
            .count();
        let n = ks.len();
        (m, plant, (with_locs, n))
    };
    for (g, base, fields) in MATCH {
        let (ok, missing) = check(&m, base, fields);
        groups.push(json!({ "group": g, "ok": ok.len(), "missing": missing }));
    }
    let mmr = match st
        .api
        .get(&paths::mmr(&me.region, &me.puuid), Prio::Foreground)
        .await
    {
        Ok(v) => {
            let d = unwrap_data(&v).clone();
            let (ok, missing) = check(&d, "", MMR);
            json!({ "group": "Rank (mmr v3)", "ok": ok.len(), "missing": missing })
        }
        Err(e) => {
            json!({ "group": "Rank (mmr v3)", "ok": 0, "missing": [], "error": crate::api::redact(&e.to_string()) })
        }
    };
    groups.push(mmr);
    let total_missing: usize = groups
        .iter()
        .map(|g| g["missing"].as_array().map(|a| a.len()).unwrap_or(0))
        .sum();
    Ok(json!({
        "groups": groups, "missing": total_missing,
        "kills_with_positions": locs.0, "kills": locs.1, "plant_locations": plant_seen,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixture_has_the_core_fields() {
        let raw = crate::parse::fixture::match_v4("m", true, "2026-11-11T10:00:00Z");
        let m = unwrap_data(&raw);
        let (ok, missing) = check(m, "/kills/0", &["/killer/puuid", "/nope"]);
        assert_eq!(ok, vec!["/killer/puuid"]);
        assert_eq!(missing, vec!["/nope"]);
    }
}
