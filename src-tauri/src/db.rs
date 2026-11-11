//! SQLite storage: schema, migrations and small helpers (plan section 6).
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub const SCHEMA_VERSION: i64 = 1;

const MIGRATION_1: &str = r#"
CREATE TABLE IF NOT EXISTS schema_info(version INTEGER NOT NULL, app_version TEXT, created_at TEXT);
CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS players(puuid TEXT PRIMARY KEY, name TEXT, tag TEXT, region TEXT, card_id TEXT,
  account_level INTEGER, last_seen_at INTEGER);
CREATE TABLE IF NOT EXISTS matches(match_id TEXT PRIMARY KEY, started_at INTEGER NOT NULL, length_ms INTEGER,
  map_id TEXT, map_name TEXT, queue TEXT, season_id TEXT, season_short TEXT, game_version TEXT, region TEXT,
  rounds_played INTEGER, red_score INTEGER, blue_score INTEGER, winning_team TEXT, raw_json_zstd BLOB);
CREATE INDEX IF NOT EXISTS matches_started ON matches(started_at DESC);
CREATE TABLE IF NOT EXISTS match_players(match_id TEXT NOT NULL, puuid TEXT NOT NULL, name TEXT, tag TEXT,
  team TEXT, party_id TEXT, agent_id TEXT, agent_name TEXT, score INTEGER, kills INTEGER, deaths INTEGER,
  assists INTEGER, headshots INTEGER, bodyshots INTEGER, legshots INTEGER, damage_made INTEGER,
  damage_received INTEGER, rank_tier INTEGER, account_level INTEGER, ability_casts_json TEXT,
  PRIMARY KEY(match_id, puuid));
CREATE INDEX IF NOT EXISTS mp_puuid ON match_players(puuid);
CREATE TABLE IF NOT EXISTS rounds(match_id TEXT NOT NULL, round_no INTEGER NOT NULL, winning_team TEXT,
  result_code TEXT, planter TEXT, defuser TEXT, plant_site TEXT, PRIMARY KEY(match_id, round_no));
CREATE TABLE IF NOT EXISTS kills(match_id TEXT NOT NULL, round_no INTEGER NOT NULL, time_in_round_ms INTEGER NOT NULL,
  time_in_match_ms INTEGER, killer TEXT, victim TEXT NOT NULL, weapon_id TEXT, weapon_name TEXT,
  killer_x REAL, killer_y REAL, victim_x REAL, victim_y REAL,
  PRIMARY KEY(match_id, round_no, time_in_round_ms, victim));
CREATE TABLE IF NOT EXISTS round_player_stats(match_id TEXT NOT NULL, round_no INTEGER NOT NULL, puuid TEXT NOT NULL,
  score INTEGER, damage INTEGER, kills INTEGER, headshots INTEGER, bodyshots INTEGER, legshots INTEGER,
  econ_spent INTEGER, loadout_value INTEGER, PRIMARY KEY(match_id, round_no, puuid));
CREATE TABLE IF NOT EXISTS mmr_snapshots(puuid TEXT NOT NULL, taken_at INTEGER NOT NULL, season_id TEXT,
  tier INTEGER, rr INTEGER, peak_tier INTEGER, elo INTEGER, PRIMARY KEY(puuid, taken_at));
CREATE TABLE IF NOT EXISTS rr_changes(match_id TEXT PRIMARY KEY, puuid TEXT, rr_change INTEGER,
  tier_after INTEGER, rr_after INTEGER);
CREATE TABLE IF NOT EXISTS player_notes(puuid TEXT PRIMARY KEY, tags_json TEXT, note TEXT, updated_at INTEGER);
CREATE TABLE IF NOT EXISTS review_items(id INTEGER PRIMARY KEY, match_id TEXT NOT NULL, round_no INTEGER,
  kind TEXT NOT NULL, note TEXT, time_in_match_ms INTEGER, created_at INTEGER NOT NULL, done_at INTEGER);
CREATE TABLE IF NOT EXISTS recordings(path TEXT PRIMARY KEY, started_at INTEGER, duration_ms INTEGER, match_id TEXT);
CREATE TABLE IF NOT EXISTS agent_pool(puuid TEXT NOT NULL, agent_id TEXT NOT NULL, comfort INTEGER NOT NULL DEFAULT 2,
  manual INTEGER NOT NULL DEFAULT 0, PRIMARY KEY(puuid, agent_id));
CREATE TABLE IF NOT EXISTS cache(key TEXT PRIMARY KEY, value TEXT NOT NULL, fetched_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS fetch_log(endpoint TEXT, key TEXT, fetched_at INTEGER, status INTEGER);
CREATE TABLE IF NOT EXISTS api_usage(minute_bucket INTEGER PRIMARY KEY, units_used INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS backups(id INTEGER PRIMARY KEY, path TEXT NOT NULL, kind TEXT NOT NULL,
  created_at INTEGER NOT NULL, size_bytes INTEGER, sha256 TEXT, counts_json TEXT);
CREATE TABLE IF NOT EXISTS import_log(id INTEGER PRIMARY KEY, file_name TEXT, format TEXT, mode TEXT,
  source_puuid TEXT, imported_at INTEGER, counts_json TEXT, conflicts_json TEXT);
"#;

/// Tables copied by export/import, in dependency order.
pub const DATA_TABLES: &[&str] = &[
    "players",
    "matches",
    "match_players",
    "rounds",
    "kills",
    "round_player_stats",
    "mmr_snapshots",
    "rr_changes",
    "player_notes",
    "review_items",
    "agent_pool",
];

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;",
    )?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn open_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn schema_version(conn: &Connection) -> i64 {
    conn.query_row("SELECT MAX(version) FROM schema_info", [], |r| {
        r.get::<_, Option<i64>>(0)
    })
    .ok()
    .flatten()
    .unwrap_or(0)
}

pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_info(version INTEGER NOT NULL, app_version TEXT, created_at TEXT);")?;
    if schema_version(conn) < 1 {
        conn.execute_batch(MIGRATION_1)?;
        conn.execute(
            "INSERT INTO schema_info(version, app_version, created_at) VALUES (1, ?1, ?2)",
            params![env!("CARGO_PKG_VERSION"), chrono::Utc::now().to_rfc3339()],
        )?;
    }
    Ok(())
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
        r.get(0)
    })
    .optional()
    .ok()
    .flatten()
}

pub fn get_json(conn: &Connection, key: &str) -> Option<serde_json::Value> {
    get_setting(conn, key).and_then(|s| serde_json::from_str(&s).ok())
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn cache_get(
    conn: &Connection,
    key: &str,
    max_age_ms: i64,
) -> Option<(serde_json::Value, i64)> {
    let row: Option<(String, i64)> = conn
        .query_row(
            "SELECT value, fetched_at FROM cache WHERE key=?1",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .ok()
        .flatten();
    let (v, at) = row?;
    if max_age_ms >= 0 && now_ms() - at > max_age_ms {
        return None;
    }
    serde_json::from_str(&v).ok().map(|v| (v, at))
}

pub fn cache_put(conn: &Connection, key: &str, value: &serde_json::Value) -> Result<()> {
    conn.execute(
        "INSERT INTO cache(key, value, fetched_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value, fetched_at=excluded.fetched_at",
        params![key, value.to_string(), now_ms()],
    )?;
    Ok(())
}

pub fn counts(conn: &Connection) -> serde_json::Value {
    let mut m = serde_json::Map::new();
    for t in DATA_TABLES {
        let n: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
            .unwrap_or(0);
        m.insert((*t).to_string(), n.into());
    }
    serde_json::Value::Object(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migrates_once() {
        let c = open_memory().unwrap();
        migrate(&c).unwrap();
        assert_eq!(schema_version(&c), SCHEMA_VERSION);
        set_setting(&c, "a", "1").unwrap();
        set_setting(&c, "a", "2").unwrap();
        assert_eq!(get_setting(&c, "a").as_deref(), Some("2"));
    }
}
