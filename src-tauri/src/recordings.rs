//! "Jump to the round in your recording" (plan 25.1). Read-only on the user's folder:
//! files are listed and matched by time; nothing is uploaded, moved or modified.
use chrono::TimeZone;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const EXT: &[&str] = &["mp4", "mkv", "mov", "webm"];

/// Find "2026-11-11 19-42-03" (OBS), "2026.11.11 - 19.42.03" (ShadowPlay) or similar in a file name: local time → epoch ms.
pub fn time_from_name(name: &str) -> Option<i64> {
    let groups: Vec<&str> = name
        .split(|c: char| !c.is_ascii_digit())
        .filter(|g| !g.is_empty())
        .collect();
    for w in groups.windows(6) {
        if w[0].len() == 4 && w[0].starts_with("20") && w[1..].iter().all(|g| g.len() == 2) {
            let n: Vec<u32> = w.iter().map(|g| g.parse().unwrap_or(0)).collect();
            if let chrono::LocalResult::Single(t) =
                chrono::Local.with_ymd_and_hms(n[0] as i32, n[1], n[2], n[3], n[4], n[5])
            {
                return Some(t.timestamp_millis());
            }
        }
    }
    None
}

fn files(dir: &Path, depth: u8, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() && depth > 0 {
            files(&p, depth - 1, out);
        } else if p
            .extension()
            .and_then(|x| x.to_str())
            .map(|x| EXT.contains(&x.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
        {
            out.push(p);
        }
    }
}

pub fn scan(c: &Connection, dir: &Path) -> anyhow::Result<Value> {
    let mut list = vec![];
    files(dir, 1, &mut list);
    c.execute("DELETE FROM recordings", [])?;
    for p in &list {
        let meta = std::fs::metadata(p)?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64);
        let created = meta
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64);
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let start = time_from_name(name).or(created);
        let (Some(start), Some(end)) = (start, modified) else {
            continue;
        };
        if end <= start {
            continue;
        }
        c.execute("INSERT OR REPLACE INTO recordings(path, started_at, duration_ms, match_id) VALUES (?1,?2,?3,NULL)", params![p.to_string_lossy(), start, end - start])?;
    }
    let matched = rematch(c)?;
    Ok(json!({ "files": list.len(), "matched": matched }))
}

/// Link each recording to the match it overlaps most.
pub fn rematch(c: &Connection) -> anyhow::Result<i64> {
    let recs: Vec<(String, i64, i64)> = c
        .prepare("SELECT path, started_at, duration_ms FROM recordings")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .flatten()
        .collect();
    let mut n = 0;
    for (path, start, dur) in recs {
        let end = start + dur;
        let best: Option<String> = c
            .query_row(
                "SELECT match_id FROM matches WHERE started_at < ?2 AND started_at + COALESCE(length_ms, 0) > ?1
                 ORDER BY MIN(?2, started_at + COALESCE(length_ms,0)) - MAX(?1, started_at) DESC LIMIT 1",
                params![start, end],
                |r| r.get(0),
            )
            .ok();
        if best.is_some() {
            n += 1;
        }
        c.execute(
            "UPDATE recordings SET match_id=?2 WHERE path=?1",
            params![path, best],
        )?;
    }
    Ok(n)
}

/// Recording for a match, with the offset of the match start inside the video.
pub fn for_match(c: &Connection, id: &str) -> Value {
    c.query_row(
        "SELECT r.path, r.started_at, r.duration_ms, m.started_at FROM recordings r JOIN matches m ON m.match_id=r.match_id WHERE r.match_id=?1 LIMIT 1",
        [id],
        |r| {
            let path: String = r.get(0)?;
            let rs: i64 = r.get(1)?;
            let ms: i64 = r.get(3)?;
            Ok(json!({ "path": path, "name": Path::new(&path).file_name().map(|f| f.to_string_lossy().to_string()), "offset_ms": ms - rs, "duration_ms": r.get::<_, i64>(2)? }))
        },
    )
    .unwrap_or(Value::Null)
}

/// Open a video at `seconds`. VLC and mpv accept a start time; otherwise the default player opens it.
pub fn open_at(path: &str, seconds: i64) -> Value {
    let secs = seconds.max(0);
    let candidates: Vec<PathBuf> = vec![
        PathBuf::from(r"C:\Program Files\VideoLAN\VLC\vlc.exe"),
        PathBuf::from(r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe"),
    ];
    for vlc in candidates.iter().filter(|p| p.exists()) {
        if std::process::Command::new(vlc)
            .arg(format!("--start-time={secs}"))
            .arg(path)
            .spawn()
            .is_ok()
        {
            return json!({ "how": "vlc" });
        }
    }
    if std::process::Command::new("mpv")
        .arg(format!("--start={secs}"))
        .arg(path)
        .spawn()
        .is_ok()
    {
        return json!({ "how": "mpv" });
    }
    json!({ "how": "default" })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_common_recorder_names() {
        let obs = time_from_name("2026-11-11 19-42-03.mp4").unwrap();
        let sp = time_from_name("Valorant 2026.11.11 - 19.42.03.02.DVR.mp4").unwrap();
        assert_eq!(obs, sp);
        assert!(time_from_name("clip.mp4").is_none());
    }
    #[test]
    fn links_overlapping_recording() {
        let mut c = crate::db::open_memory().unwrap();
        crate::parse::store_match(
            &mut c,
            &crate::parse::fixture::match_v4("m1", true, "2026-11-11T10:00:00Z"),
        )
        .unwrap();
        let ms = chrono::DateTime::parse_from_rfc3339("2026-11-11T10:00:00Z")
            .unwrap()
            .timestamp_millis();
        c.execute(
            "INSERT INTO recordings VALUES ('a.mp4', ?1, 3000000, NULL)",
            [ms - 60_000],
        )
        .unwrap();
        assert_eq!(rematch(&c).unwrap(), 1);
        assert_eq!(for_match(&c, "m1")["offset_ms"], 60_000);
    }
}
