//! Export, import and backups (plan section 16.1). Import files are untrusted input.
use crate::db::{self, now_ms, DATA_TABLES, SCHEMA_VERSION};
use crate::state::{AppState, Me};
use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OpenFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAX_BYTES: u64 = 500 * 1024 * 1024;
const DB_ENTRY: &str = "postplant.db";
const MANIFEST: &str = "manifest.json";
const SETTINGS_WHITELIST: &[&str] = &["prefs"];

fn sha256_file(p: &Path) -> Result<String> {
    let mut f = File::open(p)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

fn stamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

pub fn backups_dir(st: &AppState) -> PathBuf {
    st.data_dir.join("backups")
}

/// Write a `.postplant` file: a consistent snapshot (VACUUM INTO) plus a manifest.
pub fn write_backup(
    conn: &Connection,
    me: Option<&Me>,
    tmp_dir: &Path,
    out: &Path,
) -> Result<Value> {
    std::fs::create_dir_all(tmp_dir)?;
    let tmp = tmp_dir.join(format!("snapshot-{}.db", now_ms()));
    let _ = std::fs::remove_file(&tmp);
    conn.execute("VACUUM INTO ?1", [tmp.to_string_lossy().to_string()])?;
    let sha = sha256_file(&tmp)?;
    let counts = db::counts(conn);
    let manifest = json!({
        "format": "postplant-backup", "format_version": 1, "schema_version": SCHEMA_VERSION, "app_version": env!("CARGO_PKG_VERSION"),
        "created_at": chrono::Utc::now().to_rfc3339(), "owner": me.map(|m| json!({ "puuid": m.puuid, "name": m.name, "tag": m.tag })),
        "counts": counts, "sha256": sha,
    });
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut zw = zip::ZipWriter::new(File::create(out)?);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zw.start_file(MANIFEST, opts)?;
    zw.write_all(serde_json::to_string_pretty(&manifest)?.as_bytes())?;
    zw.start_file(DB_ENTRY, opts)?;
    let mut f = File::open(&tmp)?;
    std::io::copy(&mut f, &mut zw)?;
    zw.finish()?;
    drop(f);
    let _ = std::fs::remove_file(&tmp);
    Ok(manifest)
}

pub fn make_backup(st: &AppState, kind: &str) -> Result<PathBuf> {
    let me = st.me();
    let prefs = st.prefs();
    let path = backups_dir(st).join(format!("postplant-{kind}-{}.postplant", stamp()));
    let c = st.db.lock().unwrap();
    let manifest = write_backup(&c, me.as_ref(), &st.data_dir.join("tmp"), &path)?;
    let size = std::fs::metadata(&path)
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    c.execute(
        "INSERT INTO backups(path, kind, created_at, size_bytes, sha256, counts_json) VALUES (?1,?2,?3,?4,?5,?6)",
        params![path.to_string_lossy(), kind, now_ms(), size, manifest["sha256"].as_str(), manifest["counts"].to_string()],
    )?;
    prune(&c, &prefs)?;
    Ok(path)
}

fn prune(c: &Connection, prefs: &Value) -> Result<()> {
    let keep = prefs["backups_keep"].as_i64().unwrap_or(4).max(1);
    let old: Vec<(i64, String)> = c
        .prepare("SELECT id, path FROM backups WHERE kind='weekly' ORDER BY created_at DESC LIMIT -1 OFFSET ?1")?
        .query_map([keep], |r| Ok((r.get(0)?, r.get(1)?)))?
        .flatten()
        .collect();
    for (id, p) in old {
        let _ = std::fs::remove_file(&p);
        c.execute("DELETE FROM backups WHERE id=?1", [id])?;
    }
    Ok(())
}

pub fn weekly_backup_due(st: &AppState) -> Result<bool> {
    if !st.prefs()["auto_backup"].as_bool().unwrap_or(true) {
        return Ok(false);
    }
    let c = st.db.lock().unwrap();
    let last: i64 = c.query_row(
        "SELECT COALESCE(MAX(created_at),0) FROM backups WHERE kind='weekly'",
        [],
        |r| r.get(0),
    )?;
    Ok(now_ms() - last > 7 * 24 * 3_600_000)
}

pub fn list_backups(st: &AppState) -> Value {
    let c = st.db.lock().unwrap();
    let rows: Vec<Value> = c
        .prepare("SELECT id, path, kind, created_at, size_bytes, counts_json FROM backups ORDER BY created_at DESC")
        .and_then(|mut s| {
            s.query_map([], |r| {
                Ok(json!({ "id": r.get::<_, i64>(0)?, "path": r.get::<_, String>(1)?, "kind": r.get::<_, String>(2)?, "created_at": r.get::<_, i64>(3)?,
                    "size": r.get::<_, Option<i64>>(4)?, "counts": r.get::<_, Option<String>>(5)?.and_then(|c| serde_json::from_str::<Value>(&c).ok()) }))
            })
            .map(|x| x.flatten().filter(|v| Path::new(v["path"].as_str().unwrap_or("")).exists()).collect())
        })
        .unwrap_or_default();
    json!(rows)
}

/// Extract and verify a `.postplant` file into a temp db. Returns (manifest, temp db path).
fn open_backup(path: &Path, tmp_dir: &Path) -> Result<(Value, PathBuf)> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX_BYTES {
        bail!("File is larger than 500 MB.");
    }
    let mut z = zip::ZipArchive::new(File::open(path)?)
        .map_err(|_| anyhow!("Not a Postplant backup (not a zip)."))?;
    for n in 0..z.len() {
        let name = z.by_index(n)?.name().to_string();
        if name != DB_ENTRY && name != MANIFEST {
            bail!("Unexpected entry in backup: {name}");
        }
    }
    let mut ms = String::new();
    z.by_name(MANIFEST)
        .map_err(|_| anyhow!("Backup has no manifest."))?
        .take(1 << 20)
        .read_to_string(&mut ms)?;
    let manifest: Value = serde_json::from_str(&ms)?;
    if manifest["format"] != "postplant-backup" {
        bail!("Not a Postplant backup.");
    }
    if manifest["schema_version"].as_i64().unwrap_or(0) > SCHEMA_VERSION {
        bail!("This backup is from a newer Postplant. Update Postplant to import this file.");
    }
    std::fs::create_dir_all(tmp_dir)?;
    let tmp = tmp_dir.join(format!("import-{}.db", now_ms()));
    {
        let mut e = z
            .by_name(DB_ENTRY)
            .map_err(|_| anyhow!("Backup has no database."))?;
        if e.size() > MAX_BYTES * 4 {
            bail!("Database inside the backup is too large.");
        }
        let mut out = File::create(&tmp)?;
        std::io::copy(&mut e.by_ref().take(MAX_BYTES * 4), &mut out)?;
    }
    if let Some(want) = manifest["sha256"].as_str() {
        if sha256_file(&tmp)? != want {
            let _ = std::fs::remove_file(&tmp);
            bail!("Checksum mismatch: the backup is damaged.");
        }
    }
    {
        let ro = Connection::open_with_flags(&tmp, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let ok: String = ro.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        if ok != "ok" {
            bail!("Integrity check failed: {ok}");
        }
        let v = db::schema_version(&ro);
        if v > SCHEMA_VERSION {
            bail!("This backup is from a newer Postplant. Update Postplant to import this file.");
        }
    }
    // Older schema: migrate the imported copy forward first.
    {
        let rw = Connection::open(&tmp)?;
        db::migrate(&rw)?;
    }
    Ok((manifest, tmp))
}

pub fn inspect(st: &AppState, path: &Path) -> Result<Value> {
    let (manifest, tmp) = open_backup(path, &st.data_dir.join("tmp"))?;
    let me = st.me();
    let c = st.db.lock().unwrap();
    c.execute(
        "ATTACH DATABASE ?1 AS imp",
        [tmp.to_string_lossy().to_string()],
    )?;
    let q = |sql: &str| -> i64 { c.query_row(sql, [], |r| r.get(0)).unwrap_or(0) };
    let r = json!({
        "manifest": manifest,
        "new_matches": q("SELECT COUNT(*) FROM imp.matches WHERE match_id NOT IN (SELECT match_id FROM main.matches)"),
        "present_matches": q("SELECT COUNT(*) FROM imp.matches WHERE match_id IN (SELECT match_id FROM main.matches)"),
        "new_notes": q("SELECT COUNT(*) FROM imp.player_notes WHERE puuid NOT IN (SELECT puuid FROM main.player_notes)"),
        "note_conflicts": q("SELECT COUNT(*) FROM imp.player_notes i JOIN main.player_notes m ON m.puuid=i.puuid WHERE COALESCE(m.note,'')<>COALESCE(i.note,'')"),
        "same_owner": me.map(|m| Some(m.puuid.as_str()) == manifest.pointer("/owner/puuid").and_then(|v| v.as_str())).unwrap_or(true),
    });
    c.execute("DETACH DATABASE imp", [])?;
    let _ = std::fs::remove_file(&tmp);
    Ok(r)
}

/// mode: "merge" | "replace". notes: "newer" | "mine" | "both".
pub fn apply(
    st: &AppState,
    path: &Path,
    mode: &str,
    notes: &str,
    include_settings: bool,
) -> Result<Value> {
    let (manifest, tmp) = open_backup(path, &st.data_dir.join("tmp"))?;
    let owner = manifest
        .pointer("/owner/puuid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let same = st
        .me()
        .map(|m| m.puuid == owner || owner.is_empty())
        .unwrap_or(true);
    if mode == "merge" && !same {
        bail!("This backup belongs to a different account. Merging would mix two players' stats; use Replace instead.");
    }
    if mode == "replace" {
        make_backup(st, "before_import")?;
    }
    let mut c = st.db.lock().unwrap();
    c.execute(
        "ATTACH DATABASE ?1 AS imp",
        [tmp.to_string_lossy().to_string()],
    )?;
    let res = (|| -> Result<Value> {
        let tx = c.transaction()?;
        let before = db::counts(&tx);
        if mode == "replace" {
            for t in DATA_TABLES.iter().rev() {
                tx.execute(&format!("DELETE FROM main.{t}"), [])?;
            }
        }
        for t in DATA_TABLES {
            match *t {
                "player_notes" if mode == "merge" => {
                    let sql = match notes {
                        "mine" => "INSERT OR IGNORE INTO main.player_notes SELECT * FROM imp.player_notes",
                        "both" => "INSERT INTO main.player_notes SELECT * FROM imp.player_notes WHERE true ON CONFLICT(puuid) DO UPDATE SET
                                   note = CASE WHEN COALESCE(player_notes.note,'')=COALESCE(excluded.note,'') THEN player_notes.note ELSE COALESCE(player_notes.note,'') || ' / ' || COALESCE(excluded.note,'') END,
                                   updated_at = MAX(player_notes.updated_at, excluded.updated_at)",
                        _ => "INSERT INTO main.player_notes SELECT * FROM imp.player_notes WHERE true ON CONFLICT(puuid) DO UPDATE SET
                              tags_json=excluded.tags_json, note=excluded.note, updated_at=excluded.updated_at WHERE excluded.updated_at > player_notes.updated_at",
                    };
                    tx.execute(sql, [])?;
                }
                "review_items" => {
                    tx.execute("INSERT INTO main.review_items(match_id, round_no, kind, note, time_in_match_ms, created_at, done_at)
                                SELECT match_id, round_no, kind, note, time_in_match_ms, created_at, done_at FROM imp.review_items i
                                WHERE NOT EXISTS (SELECT 1 FROM main.review_items m WHERE m.match_id=i.match_id AND COALESCE(m.round_no,-1)=COALESCE(i.round_no,-1) AND m.kind=i.kind AND m.created_at=i.created_at)", [])?;
                }
                _ => {
                    tx.execute(
                        &format!("INSERT OR IGNORE INTO main.{t} SELECT * FROM imp.{t}"),
                        [],
                    )?;
                }
            }
        }
        if include_settings || mode == "replace" {
            for k in SETTINGS_WHITELIST {
                tx.execute("INSERT OR REPLACE INTO main.settings SELECT key, value FROM imp.settings WHERE key=?1", [k])?;
            }
        }
        if mode == "replace" {
            tx.execute("INSERT OR REPLACE INTO main.settings SELECT key, value FROM imp.settings WHERE key='me'", [])?;
        }
        let after = db::counts(&tx);
        let mut added = serde_json::Map::new();
        for t in DATA_TABLES {
            added.insert(
                (*t).into(),
                json!(after[*t].as_i64().unwrap_or(0) - before[*t].as_i64().unwrap_or(0)),
            );
        }
        tx.execute(
            "INSERT INTO import_log(file_name, format, mode, source_puuid, imported_at, counts_json, conflicts_json) VALUES (?1,'backup',?2,?3,?4,?5,?6)",
            params![path.file_name().map(|f| f.to_string_lossy().to_string()), mode, owner, now_ms(), Value::Object(added.clone()).to_string(), json!({ "notes": notes }).to_string()],
        )?;
        tx.commit()?;
        Ok(Value::Object(added))
    })();
    let _ = c.execute("DETACH DATABASE imp", []);
    let _ = std::fs::remove_file(&tmp);
    res
}

/// Open JSON export (docs/export-format.md). `anonymise` replaces every Riot ID but yours.
pub fn export_json(
    conn: &Connection,
    me: Option<&Me>,
    out: &Path,
    anonymise: bool,
    since: i64,
) -> Result<Value> {
    let mut matches = vec![];
    let mut st =
        conn.prepare("SELECT match_id FROM matches WHERE started_at>=?1 ORDER BY started_at")?;
    let ids: Vec<String> = st.query_map([since], |r| r.get(0))?.flatten().collect();
    let my = me.map(|m| m.puuid.clone()).unwrap_or_default();
    let mut names: HashMap<String, String> = HashMap::new();
    for id in &ids {
        let mut raw = crate::parse::load_raw(conn, id)?;
        if anonymise {
            anon(&mut raw, &my, &mut names);
        }
        matches.push(raw);
    }
    let table = |sql: &str| -> Vec<Value> {
        conn.prepare(sql)
            .and_then(|mut s| {
                let n = s.column_count();
                let cols: Vec<String> = (0..n)
                    .map(|i| s.column_name(i).unwrap_or("").to_string())
                    .collect();
                s.query_map([], |r| {
                    let mut o = serde_json::Map::new();
                    for (i, c) in cols.iter().enumerate() {
                        let v: rusqlite::types::Value = r.get(i)?;
                        o.insert(
                            c.clone(),
                            match v {
                                rusqlite::types::Value::Null => Value::Null,
                                rusqlite::types::Value::Integer(x) => json!(x),
                                rusqlite::types::Value::Real(x) => json!(x),
                                rusqlite::types::Value::Text(x) => json!(x),
                                rusqlite::types::Value::Blob(_) => Value::Null,
                            },
                        );
                    }
                    Ok(Value::Object(o))
                })
                .map(|x| x.flatten().collect())
            })
            .unwrap_or_default()
    };
    let doc = json!({
        "format": "postplant-export", "version": 1, "app_version": env!("CARGO_PKG_VERSION"), "created_at": chrono::Utc::now().to_rfc3339(),
        "owner": me.map(|m| json!({ "puuid": m.puuid, "name": m.name, "tag": m.tag })), "anonymised": anonymise,
        "matches": matches,
        "rank_history": table("SELECT * FROM mmr_snapshots ORDER BY taken_at"),
        "rr_changes": table("SELECT * FROM rr_changes"),
        "notes": if anonymise { json!([]) } else { json!(table("SELECT * FROM player_notes")) },
        "agent_pool": table("SELECT * FROM agent_pool"),
        "review_items": table("SELECT * FROM review_items"),
    });
    std::fs::write(out, serde_json::to_vec(&doc)?)?;
    Ok(json!({ "matches": ids.len(), "path": out.to_string_lossy() }))
}

fn anon(v: &mut Value, my: &str, names: &mut HashMap<String, String>) {
    match v {
        Value::Object(o) => {
            if let Some(p) = o
                .get("puuid")
                .and_then(|p| p.as_str())
                .map(|s| s.to_string())
            {
                if p != my && o.contains_key("name") {
                    let n = names.len() + 1;
                    let label = names
                        .entry(p)
                        .or_insert_with(|| format!("Player {n}"))
                        .clone();
                    o.insert("name".into(), json!(label));
                    o.insert("tag".into(), json!("0000"));
                }
            }
            for (_, x) in o.iter_mut() {
                anon(x, my, names);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| anon(x, my, names)),
        _ => {}
    }
}

/// Import an open JSON export (matches are re-parsed, so nothing in the file is trusted blindly).
pub fn import_json(conn: &mut Connection, path: &Path) -> Result<Value> {
    if std::fs::metadata(path)?.len() > MAX_BYTES {
        bail!("File is larger than 500 MB.");
    }
    let doc: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    if doc["format"] != "postplant-export" || doc["version"].as_i64().unwrap_or(0) > 1 {
        bail!("Not a Postplant JSON export, or from a newer version.");
    }
    let mut n = 0;
    for m in doc["matches"].as_array().cloned().unwrap_or_default() {
        if crate::parse::store_match(conn, &m).is_ok() {
            n += 1;
        }
    }
    for r in doc["rank_history"].as_array().cloned().unwrap_or_default() {
        conn.execute("INSERT OR IGNORE INTO mmr_snapshots(puuid, taken_at, season_id, tier, rr, peak_tier, elo) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![r["puuid"].as_str(), r["taken_at"].as_i64(), r["season_id"].as_str(), r["tier"].as_i64(), r["rr"].as_i64(), r["peak_tier"].as_i64(), r["elo"].as_i64()])?;
    }
    for r in doc["rr_changes"].as_array().cloned().unwrap_or_default() {
        conn.execute("INSERT OR IGNORE INTO rr_changes(match_id, puuid, rr_change, tier_after, rr_after) VALUES (?1,?2,?3,?4,?5)",
            params![r["match_id"].as_str(), r["puuid"].as_str(), r["rr_change"].as_i64(), r["tier_after"].as_i64(), r["rr_after"].as_i64()])?;
    }
    for r in doc["notes"].as_array().cloned().unwrap_or_default() {
        conn.execute("INSERT OR IGNORE INTO player_notes(puuid, tags_json, note, updated_at) VALUES (?1,?2,?3,?4)",
            params![r["puuid"].as_str(), r["tags_json"].as_str(), r["note"].as_str(), r["updated_at"].as_i64()])?;
    }
    for r in doc["agent_pool"].as_array().cloned().unwrap_or_default() {
        conn.execute("INSERT OR IGNORE INTO agent_pool(puuid, agent_id, comfort, manual) VALUES (?1,?2,?3,?4)",
            params![r["puuid"].as_str(), r["agent_id"].as_str(), r["comfort"].as_i64().unwrap_or(2), r["manual"].as_i64().unwrap_or(0)])?;
    }
    Ok(json!({ "matches": n }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{fixture, store_match};
    fn dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("pp-test-{}", now_ms()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }
    #[test]
    fn backup_roundtrip_and_tamper_detection() {
        let d = dir();
        let mut c = db::open_memory().unwrap();
        store_match(
            &mut c,
            &fixture::match_v4("m1", true, "2026-11-11T10:00:00Z"),
        )
        .unwrap();
        let out = d.join("a.postplant");
        let m = write_backup(&c, None, &d, &out).unwrap();
        assert_eq!(m["counts"]["matches"], 1);
        let (_, tmp) = open_backup(&out, &d).unwrap();
        let other = Connection::open(&tmp).unwrap();
        let n: i64 = other
            .query_row("SELECT COUNT(*) FROM matches", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
        // A zip with an unexpected entry is refused.
        let bad = d.join("bad.postplant");
        let mut zw = zip::ZipWriter::new(File::create(&bad).unwrap());
        zw.start_file("../evil.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        zw.write_all(b"x").unwrap();
        zw.finish().unwrap();
        assert!(open_backup(&bad, &d).is_err());
    }
    #[test]
    fn newer_schema_is_refused() {
        let d = dir();
        let c = db::open_memory().unwrap();
        let out = d.join("n.postplant");
        write_backup(&c, None, &d, &out).unwrap();
        // Rewrite the manifest with a future schema version.
        let mut z = zip::ZipArchive::new(File::open(&out).unwrap()).unwrap();
        let mut dbb = vec![];
        z.by_name(DB_ENTRY).unwrap().read_to_end(&mut dbb).unwrap();
        let fut = d.join("f.postplant");
        let mut zw = zip::ZipWriter::new(File::create(&fut).unwrap());
        zw.start_file(MANIFEST, zip::write::SimpleFileOptions::default())
            .unwrap();
        zw.write_all(br#"{"format":"postplant-backup","schema_version":99}"#)
            .unwrap();
        zw.start_file(DB_ENTRY, zip::write::SimpleFileOptions::default())
            .unwrap();
        zw.write_all(&dbb).unwrap();
        zw.finish().unwrap();
        let e = open_backup(&fut, &d).unwrap_err().to_string();
        assert!(e.contains("newer"));
    }
    #[test]
    fn json_export_anonymises_others() {
        let d = dir();
        let mut c = db::open_memory().unwrap();
        store_match(
            &mut c,
            &fixture::match_v4("m1", true, "2026-11-11T10:00:00Z"),
        )
        .unwrap();
        let me = Me {
            puuid: "p0".into(),
            name: "Player0".into(),
            tag: "t".into(),
            region: "ap".into(),
            account_level: 1,
        };
        let out = d.join("e.json");
        export_json(&c, Some(&me), &out, true, 0).unwrap();
        let s = std::fs::read_to_string(&out).unwrap();
        assert!(s.contains("Player0"));
        assert!(!s.contains("\"Player3\""));
        let mut c2 = db::open_memory().unwrap();
        assert_eq!(import_json(&mut c2, &out).unwrap()["matches"], 1);
    }
}
