//! Comp Advisor (plan section 14), knowledge-base mode ("Tier 3": role and utility coverage).
//! Pure functions, no I/O, no API calls.
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

#[derive(Deserialize, Clone, Debug)]
pub struct Agent {
    pub agent_id: String,
    pub name: String,
    pub role: String,
    pub utility: HashMap<String, f64>,
    #[serde(default)]
    pub flex_roles: Vec<String>,
    #[serde(default)]
    pub limited_data: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MapInfo {
    pub map_id: String,
    pub name: String,
    pub utility_needs: HashMap<String, f64>,
    pub role_mix: HashMap<String, f64>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Weights {
    pub comp_fit: f64,
    pub role_fit: f64,
    pub comfort: f64,
    pub synergy: f64,
    pub third_duelist_penalty: f64,
    pub no_smokes_bonus: f64,
    pub min_games_for_comfort: i64,
}

pub struct Kb {
    pub agents: Vec<Agent>,
    pub maps: Vec<MapInfo>,
    pub pool: Vec<String>,
    pub weights: Weights,
}

#[derive(Deserialize)]
struct AgentsFile {
    agents: Vec<Agent>,
}
#[derive(Deserialize)]
struct MapsFile {
    maps: Vec<MapInfo>,
}
#[derive(Deserialize)]
struct PoolFile {
    names: Vec<String>,
}

impl Kb {
    pub fn load() -> Kb {
        let a: AgentsFile =
            serde_json::from_str(include_str!("../../data/agents.json")).expect("agents.json");
        let m: MapsFile =
            serde_json::from_str(include_str!("../../data/maps.json")).expect("maps.json");
        let p: PoolFile =
            serde_json::from_str(include_str!("../../data/map-pool.json")).expect("map-pool.json");
        let w: Weights = serde_json::from_str(include_str!("../../data/advisor-weights.json"))
            .expect("advisor-weights.json");
        Kb {
            agents: a.agents,
            maps: m.maps,
            pool: p.names,
            weights: w,
        }
    }
    pub fn agent(&self, name: &str) -> Option<&Agent> {
        self.agents
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(name) || a.agent_id.eq_ignore_ascii_case(name))
    }
    pub fn map(&self, name: &str) -> Option<&MapInfo> {
        self.maps
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(name) || m.map_id.eq_ignore_ascii_case(name))
    }
    pub fn json(&self) -> Value {
        json!({
            "agents": self.agents.iter().map(|a| json!({ "id": a.agent_id, "name": a.name, "role": a.role, "limited": a.limited_data })).collect::<Vec<_>>(),
            "maps": self.maps.iter().map(|m| json!({ "id": m.map_id, "name": m.name, "in_pool": self.pool.contains(&m.name), "notes": m.notes })).collect::<Vec<_>>(),
            "pool": self.pool,
        })
    }
}

/// Your record on an agent: (games, wins) overall and on this map.
#[derive(Default, Clone, Copy, Debug)]
pub struct Comfort {
    pub games: i64,
    pub wins: i64,
    pub map_games: i64,
    pub map_wins: i64,
}

const TAG_LABEL: &[(&str, &str)] = &[
    ("smokes", "smokes"),
    ("flash", "flashes"),
    ("recon", "recon"),
    ("stun", "stuns"),
    ("entry", "entry"),
    ("anchor", "site anchoring"),
    ("flank", "flank watch"),
    ("heal", "heals"),
    ("wall", "walls"),
    ("postplant", "post-plant"),
    ("mobility", "mobility"),
    ("info_denial", "info denial"),
];

fn team_vec(team: &[&Agent]) -> HashMap<String, f64> {
    let mut t = HashMap::new();
    for a in team {
        for (k, v) in &a.utility {
            *t.entry(k.clone()).or_insert(0.0) += v;
        }
    }
    t
}

fn role_count(team: &[&Agent], role: &str) -> f64 {
    team.iter().filter(|a| a.role == role).count() as f64
}

pub struct Scored {
    pub score: f64,
    pub reasons: Vec<String>,
}

/// Score one candidate joining `team` on `map`.
pub fn score(
    kb: &Kb,
    map: &MapInfo,
    team: &[&Agent],
    cand: &Agent,
    comfort: Option<Comfort>,
) -> Scored {
    let w = &kb.weights;
    let tv = team_vec(team);
    let total_need: f64 = map.utility_needs.values().sum::<f64>().max(1.0);
    let mut gain = 0.0;
    let mut best_tag = ("", 0.0);
    for (tag, need) in &map.utility_needs {
        let have = tv.get(tag).copied().unwrap_or(0.0);
        let add = cand.utility.get(tag).copied().unwrap_or(0.0);
        let g = (have + add).min(*need) - have.min(*need);
        gain += g;
        if g > best_tag.1 {
            best_tag = (tag.as_str(), g);
        }
    }
    let fit = gain / total_need;
    let mut s = w.comp_fit * fit * 3.0;
    let mut reasons = vec![];
    if best_tag.1 > 0.0 {
        let label = TAG_LABEL
            .iter()
            .find(|(k, _)| *k == best_tag.0)
            .map(|x| x.1)
            .unwrap_or(best_tag.0);
        let need = map.utility_needs.get(best_tag.0).copied().unwrap_or(0.0);
        let have = tv.get(best_tag.0).copied().unwrap_or(0.0);
        reasons.push(format!(
            "Adds {label}: {} wants about {need:.0} points, the team has {have:.0}",
            map.name
        ));
    }
    let target = map.role_mix.get(&cand.role).copied().unwrap_or(1.0);
    let have_role = role_count(team, &cand.role);
    let flex_ok = cand
        .flex_roles
        .iter()
        .any(|r| role_count(team, r) < map.role_mix.get(r).copied().unwrap_or(1.0));
    if have_role < target {
        s += w.role_fit;
        reasons.push(if have_role == 0.0 {
            format!(
                "No {} yet; {} usually runs {target:.0}",
                cand.role.to_lowercase(),
                map.name
            )
        } else {
            format!(
                "{} usually runs {target:.0} {}s",
                map.name,
                cand.role.to_lowercase()
            )
        });
    } else if flex_ok {
        s += w.role_fit * 0.5;
        reasons.push(format!(
            "Can flex into a missing {} role",
            cand.flex_roles.join("/").to_lowercase()
        ));
    } else {
        s -= w.role_fit * (have_role - target + 1.0) * 0.6;
        if cand.role == "Duelist" && have_role >= 2.0 {
            s -= w.third_duelist_penalty;
        }
    }
    if team.len() >= 2
        && tv.get("smokes").copied().unwrap_or(0.0) == 0.0
        && cand.utility.get("smokes").copied().unwrap_or(0.0) >= 2.0
    {
        s += w.no_smokes_bonus;
        reasons.push("The team has no smokes yet".into());
    }
    if let Some(c) = comfort {
        let (g, wn, scope) = if c.map_games >= w.min_games_for_comfort {
            (c.map_games, c.map_wins, format!(" on {}", map.name))
        } else {
            (c.games, c.wins, String::new())
        };
        if g >= w.min_games_for_comfort {
            let shrunk = (wn as f64 + 5.0) / (g as f64 + 10.0);
            s += w.comfort * ((shrunk - 0.5) * 4.0 + (g as f64).ln() / 6.0);
            reasons.push(format!(
                "Your record{scope}: {g} games, {:.0}% win",
                100.0 * wn as f64 / g as f64
            ));
        } else if g > 0 {
            s += w.comfort * 0.1 * g as f64 / w.min_games_for_comfort as f64;
        }
    }
    Scored { score: s, reasons }
}

pub struct Input<'a> {
    pub map: &'a str,
    pub team: Vec<String>,
    pub owned: HashSet<String>,
    pub comfort: HashMap<String, Comfort>,
}

pub fn suggest(kb: &Kb, inp: &Input) -> Result<Value, String> {
    let map = kb.map(inp.map).ok_or("Unknown map")?;
    let team: Vec<&Agent> = inp.team.iter().filter_map(|n| kb.agent(n)).collect();
    let taken: HashSet<&str> = team.iter().map(|a| a.name.as_str()).collect();
    let mut cands: Vec<(f64, &Agent, Vec<String>)> = kb
        .agents
        .iter()
        .filter(|a| !taken.contains(a.name.as_str()) && inp.owned.contains(&a.agent_id))
        .map(|a| {
            let sc = score(kb, map, &team, a, inp.comfort.get(&a.name).copied());
            (sc.score, a, sc.reasons)
        })
        .collect();
    cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let top: Vec<Value> = cands
        .iter()
        .take(3)
        .map(|(s, a, r)| {
            let mut reasons = r.clone();
            if reasons.is_empty() {
                reasons.push("Fits the team's role coverage".into());
            }
            json!({ "name": a.name, "id": a.agent_id, "role": a.role, "score": (s * 10.0).round() / 10.0, "reasons": reasons.iter().take(3).collect::<Vec<_>>(),
                "tier": "Role coverage", "confidence": if team.len() >= 2 { "Medium" } else { "Low" }, "limited": a.limited_data })
        })
        .collect();
    // Comps: my pick from the top candidates, then fill the open slots greedily from all agents.
    let mut comps: Vec<(f64, Vec<String>)> = vec![];
    for (_, mine, _) in cands.iter().take(6) {
        let mut cur: Vec<&Agent> = team.clone();
        cur.push(mine);
        let mut total = 0.0;
        while cur.len() < 5 {
            let used: HashSet<&str> = cur.iter().map(|a| a.name.as_str()).collect();
            let best = kb
                .agents
                .iter()
                .filter(|a| !used.contains(a.name.as_str()))
                .map(|a| (score(kb, map, &cur, a, None).score, a))
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            match best {
                Some((s, a)) => {
                    total += s;
                    cur.push(a);
                }
                None => break,
            }
        }
        let names: Vec<String> = cur.iter().map(|a| a.name.clone()).collect();
        let mut key = names.clone();
        key.sort();
        if !comps.iter().any(|(_, c)| {
            let mut k = c.clone();
            k.sort();
            k == key
        }) {
            comps.push((total, names));
        }
    }
    comps.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let comps: Vec<Value> = comps
        .iter()
        .take(3)
        .map(|(s, names)| {
            let agents: Vec<&Agent> = names.iter().filter_map(|n| kb.agent(n)).collect();
            let roles: Vec<String> = ["Duelist", "Initiator", "Controller", "Sentinel"].iter().map(|r| format!("{} {}", role_count(&agents, r), r.to_lowercase())).collect();
            json!({ "agents": names, "you": names.get(team.len()), "score": (s * 10.0).round() / 10.0, "roles": roles.join(" · "), "tier": "Role coverage" })
        })
        .collect();
    Ok(
        json!({ "map": map.name, "notes": map.notes, "agents": top, "comps": comps,
        "footer": "Suggestions from role and utility coverage, not win-rate data. Play what you play well." }),
    )
}

/// "Learn for this map": 1–2 agents outside your regular pool that fit the map.
pub fn learn(
    kb: &Kb,
    map_name: &str,
    owned: &HashSet<String>,
    comfort: &HashMap<String, Comfort>,
) -> Result<Value, String> {
    let map = kb.map(map_name).ok_or("Unknown map")?;
    let regular: Vec<&Agent> = kb
        .agents
        .iter()
        .filter(|a| comfort.get(&a.name).map(|c| c.games >= 5).unwrap_or(false))
        .collect();
    let mut c: Vec<(f64, &Agent, Vec<String>)> = kb
        .agents
        .iter()
        .filter(|a| !regular.iter().any(|r| r.name == a.name))
        .map(|a| {
            let empty: Vec<&Agent> = vec![];
            let sc = score(kb, map, &empty, a, None);
            let covers_gap = !regular.iter().any(|r| r.role == a.role);
            (sc.score + if covers_gap { 1.0 } else { 0.0 }, a, {
                let mut r = sc.reasons;
                if covers_gap {
                    r.insert(
                        0,
                        format!("You have no regular {} yet", a.role.to_lowercase()),
                    );
                }
                r
            })
        })
        .collect();
    c.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    Ok(
        json!({ "map": map.name, "picks": c.iter().take(2).map(|(_, a, r)| json!({ "name": a.name, "id": a.agent_id, "role": a.role, "owned": owned.contains(&a.agent_id), "reasons": r.iter().take(3).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "regular": regular.iter().map(|a| a.name.clone()).collect::<Vec<_>>() }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn all_owned(kb: &Kb) -> HashSet<String> {
        kb.agents.iter().map(|a| a.agent_id.clone()).collect()
    }
    #[test]
    fn knowledge_base_loads_every_pool_map() {
        let kb = Kb::load();
        assert_eq!(kb.agents.len(), 30);
        for m in &kb.pool {
            assert!(kb.map(m).is_some(), "{m} missing from maps.json");
        }
    }
    #[test]
    fn no_third_duelist_first() {
        let kb = Kb::load();
        let r = suggest(
            &kb,
            &Input {
                map: "Lotus",
                team: vec!["Jett".into(), "Raze".into()],
                owned: all_owned(&kb),
                comfort: HashMap::new(),
            },
        )
        .unwrap();
        let first = r["agents"][0]["name"].as_str().unwrap();
        assert_ne!(kb.agent(first).unwrap().role, "Duelist");
    }
    #[test]
    fn bind_without_controller_suggests_one() {
        let kb = Kb::load();
        let r = suggest(
            &kb,
            &Input {
                map: "Bind",
                team: vec!["Jett".into(), "Sova".into(), "Killjoy".into()],
                owned: all_owned(&kb),
                comfort: HashMap::new(),
            },
        )
        .unwrap();
        assert!(r["agents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["role"] == "Controller"));
    }
    #[test]
    fn never_suggests_unowned_agents() {
        let kb = Kb::load();
        let owned: HashSet<String> = ["Jett", "Sage", "Sova", "Brimstone", "Phoenix"]
            .iter()
            .map(|n| kb.agent(n).unwrap().agent_id.clone())
            .collect();
        let r = suggest(
            &kb,
            &Input {
                map: "Haven",
                team: vec![],
                owned: owned.clone(),
                comfort: HashMap::new(),
            },
        )
        .unwrap();
        for a in r["agents"].as_array().unwrap() {
            assert!(owned.contains(a["id"].as_str().unwrap()));
        }
    }
    #[test]
    fn limited_data_agent_explains_by_role_coverage_only() {
        let kb = Kb::load();
        let r = suggest(
            &kb,
            &Input {
                map: "Sunset",
                team: vec![],
                owned: [kb.agent("Hush").unwrap().agent_id.clone()]
                    .into_iter()
                    .collect(),
                comfort: HashMap::new(),
            },
        )
        .unwrap();
        assert_eq!(r["agents"][0]["name"], "Hush");
        assert_eq!(r["agents"][0]["tier"], "Role coverage");
        assert_eq!(r["agents"][0]["limited"], true);
    }
    #[test]
    fn comps_have_five_agents_including_picks() {
        let kb = Kb::load();
        let r = suggest(
            &kb,
            &Input {
                map: "Summit",
                team: vec!["Fade".into()],
                owned: all_owned(&kb),
                comfort: HashMap::new(),
            },
        )
        .unwrap();
        let c = r["comps"][0]["agents"].as_array().unwrap();
        assert_eq!(c.len(), 5);
        assert!(c.iter().any(|a| a == "Fade"));
    }
}
