//! Advisor data updates (plan 15.6). Every release bundles data/; between releases the app can pick
//! up newer data from the rolling `dataset` GitHub release: `dataset.json` plus `dataset.json.sig`,
//! signed in CI with the same minisign key as app updates. Nothing is loaded unless the signature
//! checks out against the public key built into the app, and only strictly newer versions replace
//! what's loaded. At most once a day, never while VALORANT runs, and switchable off in Settings.
use crate::advisor::Kb;
use crate::state::AppState;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const BASE: &str = "https://github.com/jeelnadaa/postplant/releases/download/dataset/";
const MAX_BYTES: usize = 8 * 1024 * 1024;
const FORMAT: i64 = 1;

/// The knowledge base the advisor and insights read, swappable when newer data arrives.
pub struct KbStore {
    kb: RwLock<Arc<Kb>>,
    last: RwLock<Value>,
}

impl KbStore {
    pub fn new(kb: Kb) -> Self {
        KbStore {
            kb: RwLock::new(Arc::new(kb)),
            last: RwLock::new(Value::Null),
        }
    }
    pub fn get(&self) -> Arc<Kb> {
        self.kb.read().unwrap().clone()
    }
    fn set(&self, kb: Kb) {
        *self.kb.write().unwrap() = Arc::new(kb);
    }
}

/// Check a base64 minisign signature (the format `tauri signer sign` writes) against the base64
/// public key from tauri.conf.json.
pub fn verify(pubkey_b64: &str, data: &[u8], sig_b64: &str) -> Result<(), String> {
    let text = |b: &str| -> Result<String, String> {
        String::from_utf8(B64.decode(b.trim()).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    };
    let pk = minisign_verify::PublicKey::decode(&text(pubkey_b64)?)
        .map_err(|e| format!("public key: {e}"))?;
    let sig = minisign_verify::Signature::decode(&text(sig_b64)?)
        .map_err(|e| format!("signature: {e}"))?;
    pk.verify(data, &sig, true)
        .map_err(|_| "signature doesn't match".to_string())
}

/// Parse a verified dataset file into a knowledge base.
pub fn parse(data: &[u8]) -> Result<Kb, String> {
    let v: Value = serde_json::from_slice(data).map_err(|e| e.to_string())?;
    if v["format"].as_i64() != Some(FORMAT) {
        return Err(format!(
            "dataset format {} isn't supported by this version",
            v["format"]
        ));
    }
    let version = v["version"].as_i64().ok_or("dataset has no version")?;
    Kb::from_files(&v["files"], version, "downloaded")
}

fn pubkey(app: &AppHandle) -> Option<String> {
    app.config()
        .plugins
        .0
        .get("updater")?
        .get("pubkey")?
        .as_str()
        .map(str::to_string)
}

fn paths(dir: &Path) -> (PathBuf, PathBuf) {
    (dir.join("dataset.json"), dir.join("dataset.json.sig"))
}

/// At start: the bundled data, or a previously downloaded newer dataset if its signature still checks out.
pub fn initial(app: &AppHandle, dir: &Path) -> Kb {
    let bundled = Kb::load();
    let (d, s) = paths(dir);
    let (Ok(data), Ok(sig), Some(pk)) =
        (std::fs::read(&d), std::fs::read_to_string(&s), pubkey(app))
    else {
        return bundled;
    };
    match verify(&pk, &data, &sig).and_then(|_| parse(&data)) {
        Ok(kb) if kb.version > bundled.version => kb,
        Ok(_) => {
            // The app update brought data at least as new: drop the old download.
            let _ = std::fs::remove_file(&d);
            let _ = std::fs::remove_file(&s);
            bundled
        }
        Err(e) => {
            eprintln!("ignoring stored dataset: {e}");
            bundled
        }
    }
}

async fn fetch(client: &reqwest::Client, name: &str) -> Result<Option<Vec<u8>>, String> {
    let r = client
        .get(format!("{BASE}{name}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if r.status().as_u16() == 404 {
        return Ok(None);
    }
    if !r.status().is_success() {
        return Err(format!("GitHub returned {}", r.status()));
    }
    if r.content_length().unwrap_or(0) as usize > MAX_BYTES {
        return Err("dataset is too large".into());
    }
    let b = r.bytes().await.map_err(|e| e.to_string())?;
    if b.len() > MAX_BYTES {
        return Err("dataset is too large".into());
    }
    Ok(Some(b.to_vec()))
}

/// Look for newer advisor data. { updated, version, source, message }
pub async fn check(app: &AppHandle) -> Result<Value, String> {
    let st = app.state::<AppState>();
    if st.game_running.load(Ordering::SeqCst) {
        return Ok(json!({ "updated": false, "message": "Skipped while VALORANT is running." }));
    }
    let store = app.state::<KbStore>();
    let cur = store.get().version;
    let pk = pubkey(app).ok_or("no public key built into this app")?;
    let client = reqwest::Client::builder()
        .user_agent(concat!("Postplant/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let result = async {
        let Some(sig) = fetch(&client, "dataset.json.sig").await? else {
            return Ok(json!({ "updated": false, "version": cur, "message": "No data published yet." }));
        };
        let sig = String::from_utf8(sig).map_err(|_| "signature isn't text".to_string())?;
        let data = fetch(&client, "dataset.json")
            .await?
            .ok_or("dataset file missing from the release")?;
        verify(&pk, &data, &sig)?;
        let kb = parse(&data)?;
        if kb.version <= cur {
            return Ok(json!({ "updated": false, "version": cur, "message": "Advisor data is up to date." }));
        }
        let (d, s) = paths(&st.data_dir);
        let tmp = d.with_extension("json.tmp");
        std::fs::write(&tmp, &data).map_err(|e| e.to_string())?;
        std::fs::write(&s, &sig).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &d).map_err(|e| e.to_string())?;
        let v = kb.version;
        store.set(kb);
        let _ = app.emit("kb-updated", v);
        Ok::<Value, String>(json!({ "updated": true, "version": v, "message": "New advisor data loaded." }))
    }
    .await;
    *store.last.write().unwrap() = json!({
        "at": crate::db::now_ms(),
        "ok": result.is_ok(),
        "message": match &result { Ok(v) => v["message"].clone(), Err(e) => json!(e) },
    });
    result
}

/// What's loaded, for Settings.
pub fn status(app: &AppHandle) -> Value {
    let store = app.state::<KbStore>();
    let kb = store.get();
    let t = &kb.tier1;
    let mut maps: Vec<Value> = t
        .maps
        .iter()
        .map(|(k, m)| json!({ "map": k, "n": m.n, "enough": m.n >= crate::advisor::MIN_PRO_GAMES }))
        .collect();
    maps.sort_by(|a, b| a["map"].as_str().cmp(&b["map"].as_str()));
    json!({
        "version": kb.version,
        "bundled_version": crate::advisor::bundled_version(),
        "source": kb.source,
        "auto": app.state::<AppState>().prefs()["data_updates"].as_bool().unwrap_or(true),
        "last_check": *store.last.read().unwrap(),
        "tier1": { "games": t.games, "window": t.window, "fetched_at": t.fetched_at, "source": t.source,
                   "license": t.license, "attribution": t.attribution, "maps": maps },
    })
}

/// Daily background check, first one a couple of minutes after start.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(120)).await;
        loop {
            let st = app.state::<AppState>();
            let on = st.prefs()["data_updates"].as_bool().unwrap_or(true);
            if on && !st.game_running.load(Ordering::SeqCst) {
                if let Err(e) = check(&app).await {
                    eprintln!("advisor data check failed: {e}");
                }
            }
            tokio::time::sleep(Duration::from_secs(24 * 3600)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // Signed with the release key (`tauri signer sign`) over fixtures/dataset-test.json.
    const DATA: &[u8] = include_bytes!("../fixtures/dataset-test.json");
    const SIG: &str = include_str!("../fixtures/dataset-test.json.sig");

    fn pk() -> String {
        let conf: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        conf["plugins"]["updater"]["pubkey"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn signed_dataset_verifies_and_parses() {
        verify(&pk(), DATA, SIG).expect("valid signature");
        let kb = parse(DATA).expect("parses");
        assert!(kb.version > 0);
        assert_eq!(kb.source, "downloaded");
        assert_eq!(kb.pool.len(), 7);
    }

    #[test]
    fn tampered_dataset_is_rejected() {
        let mut bad = DATA.to_vec();
        let i = bad.iter().position(|&b| b == b'7').unwrap_or(10);
        bad[i] = b'8';
        assert!(verify(&pk(), &bad, SIG).is_err());
        assert!(verify(&pk(), DATA, "bm90IGEgc2lnbmF0dXJl").is_err());
    }

    #[test]
    fn rejects_unknown_format_and_thin_data() {
        assert!(parse(br#"{"format":2,"version":5,"files":{}}"#).is_err());
        assert!(parse(br#"{"format":1,"version":5,"files":{}}"#).is_err());
    }
}
