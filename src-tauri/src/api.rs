//! HenrikDev client behind the budget governor (plan section 13).
//! Every request goes through `Henrik::get`, so there is one limiter for the whole app.
use serde_json::Value;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const BASE: &str = "https://api.henrikdev.xyz";
pub const DEFAULT_LIMIT: u32 = 30;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prio {
    /// The user clicked something and is waiting.
    Foreground,
    /// Sync and polling.
    Background,
}

#[derive(Debug)]
pub struct Governor {
    pub limit: u32,
    minute_start: Instant,
    pub used_minute: u32,
    pub remaining_hdr: Option<u32>,
    pub paused_until: Option<Instant>,
    pub day: String,
    pub used_today: u32,
    pub daily_cap: u32,
}

#[derive(Debug, PartialEq)]
pub enum Decision {
    Go,
    Wait(Duration),
    Deny(&'static str),
}

impl Governor {
    pub fn new(daily_cap: u32) -> Self {
        Self {
            limit: DEFAULT_LIMIT,
            minute_start: Instant::now(),
            used_minute: 0,
            remaining_hdr: None,
            paused_until: None,
            day: today(),
            used_today: 0,
            daily_cap,
        }
    }

    fn roll(&mut self, now: Instant) {
        if now.duration_since(self.minute_start) >= Duration::from_secs(60) {
            self.minute_start = now;
            self.used_minute = 0;
            self.remaining_hdr = None;
        }
        let d = today();
        if d != self.day {
            self.day = d;
            self.used_today = 0;
        }
    }

    /// Decide whether a request of this priority may go now.
    pub fn decide(&mut self, prio: Prio, now: Instant) -> Decision {
        self.roll(now);
        let until_reset =
            Duration::from_secs(60).saturating_sub(now.duration_since(self.minute_start));
        if let Some(p) = self.paused_until {
            if now < p {
                return match prio {
                    Prio::Background => Decision::Deny("rate limited"),
                    Prio::Foreground => Decision::Wait(p - now),
                };
            }
            self.paused_until = None;
        }
        let hard = (self.limit * 8 / 10).max(1);
        let bg = (self.limit / 2).max(1);
        if prio == Prio::Background {
            if self.used_today >= self.daily_cap {
                return Decision::Deny("daily cap reached");
            }
            if let Some(r) = self.remaining_hdr {
                if r * 5 < self.limit {
                    return Decision::Deny("less than 20% of the minute left");
                }
            }
            if self.used_minute >= bg {
                return Decision::Deny("background share used");
            }
        }
        if self.used_minute >= hard {
            return Decision::Wait(until_reset);
        }
        Decision::Go
    }

    pub fn record(&mut self) {
        self.used_minute += 1;
        self.used_today += 1;
    }

    pub fn on_headers(&mut self, limit: Option<u32>, remaining: Option<u32>) {
        if let Some(l) = limit {
            if l > 0 {
                self.limit = l;
            }
        }
        if remaining.is_some() {
            self.remaining_hdr = remaining;
        }
    }

    pub fn on_429(&mut self, retry_after: Duration, now: Instant) {
        self.paused_until = Some(now + retry_after);
    }
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[derive(Debug)]
pub enum ApiError {
    NoKey,
    Budget(String),
    Status(u16, String),
    Net(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NoKey => write!(
                f,
                "No HenrikDev API key yet. Add one in Settings → API key."
            ),
            ApiError::Budget(r) => write!(f, "Paused to protect your API quota ({r})."),
            ApiError::Status(401 | 403, _) => write!(
                f,
                "The API key was rejected. Check it in Settings → API key."
            ),
            ApiError::Status(404, _) => write!(f, "Not found. Check the Riot ID and region."),
            ApiError::Status(429, _) => write!(
                f,
                "Rate limited by HenrikDev. Postplant will retry after the limit resets."
            ),
            ApiError::Status(s, m) => write!(f, "HenrikDev error {s}: {m}"),
            ApiError::Net(e) => write!(f, "Network error: {e}"),
        }
    }
}

pub struct Henrik {
    http: reqwest::Client,
    pub gov: Mutex<Governor>,
    pub key: Mutex<Option<String>>,
}

impl Henrik {
    pub fn new(key: Option<String>) -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!(
                    "Postplant/",
                    env!("CARGO_PKG_VERSION"),
                    " (+https://github.com/jeelnadaa/postplant)"
                ))
                .timeout(Duration::from_secs(25))
                .build()
                .expect("http client"),
            gov: Mutex::new(Governor::new(1500)),
            key: Mutex::new(key),
        }
    }

    pub fn has_key(&self) -> bool {
        self.key.lock().unwrap().is_some()
    }

    /// GET a HenrikDev path ("/valorant/v2/account/..."), returning the JSON body.
    pub async fn get(&self, path: &str, prio: Prio) -> Result<Value, ApiError> {
        let key = self.key.lock().unwrap().clone().ok_or(ApiError::NoKey)?;
        loop {
            let d = self.gov.lock().unwrap().decide(prio, Instant::now());
            match d {
                Decision::Go => break,
                Decision::Deny(r) => return Err(ApiError::Budget(r.into())),
                Decision::Wait(w) if w <= Duration::from_secs(20) => {
                    tokio::time::sleep(w + Duration::from_millis(50)).await
                }
                Decision::Wait(w) => {
                    return Err(ApiError::Budget(format!("retry in {} s", w.as_secs() + 1)))
                }
            }
        }
        self.gov.lock().unwrap().record();
        let resp = self
            .http
            .get(format!("{BASE}{path}"))
            .header("Authorization", key)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| ApiError::Net(redact(&e.to_string())))?;
        let h = resp.headers();
        let num = |n: &str| {
            h.get(n)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<u32>().ok())
        };
        let (limit, remaining, reset) = (
            num("x-ratelimit-limit"),
            num("x-ratelimit-remaining"),
            num("x-ratelimit-reset"),
        );
        let retry = num("retry-after").or(reset).unwrap_or(30);
        self.gov.lock().unwrap().on_headers(limit, remaining);
        let status = resp.status().as_u16();
        if status == 429 {
            self.gov
                .lock()
                .unwrap()
                .on_429(Duration::from_secs(retry as u64), Instant::now());
            return Err(ApiError::Status(429, String::new()));
        }
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        if status >= 400 {
            let msg = body
                .pointer("/errors/0/message")
                .or_else(|| body.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return Err(ApiError::Status(status, msg));
        }
        Ok(body)
    }

    pub fn usage(&self) -> Value {
        let g = self.gov.lock().unwrap();
        serde_json::json!({ "limit": g.limit, "minute": g.used_minute, "today": g.used_today, "daily_cap": g.daily_cap,
            "paused_s": g.paused_until.map(|p| p.saturating_duration_since(Instant::now()).as_secs()).unwrap_or(0) })
    }
}

/// Logs and errors never contain anything shaped like a key.
pub fn redact(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            if w.starts_with("HDEV-")
                || (w.len() > 30 && w.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
            {
                "[redacted]"
            } else {
                w
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub mod paths {
    use super::enc;
    pub fn account(name: &str, tag: &str) -> String {
        format!("/valorant/v2/account/{}/{}", enc(name), enc(tag))
    }
    pub fn mmr(region: &str, puuid: &str) -> String {
        format!("/valorant/v3/by-puuid/mmr/{region}/pc/{puuid}")
    }
    pub fn mmr_history(region: &str, puuid: &str) -> String {
        format!("/valorant/v1/by-puuid/mmr-history/{region}/{puuid}")
    }
    pub fn stored_matches(region: &str, puuid: &str, size: u32, mode: Option<&str>) -> String {
        let m = mode.map(|m| format!("&mode={m}")).unwrap_or_default();
        format!("/valorant/v1/by-puuid/stored-matches/{region}/{puuid}?size={size}{m}")
    }
    pub fn match_v4(region: &str, id: &str) -> String {
        format!("/valorant/v4/match/{region}/{id}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn background_capped_at_half() {
        let mut g = Governor::new(1500);
        let now = Instant::now();
        for _ in 0..15 {
            assert_eq!(g.decide(Prio::Background, now), Decision::Go);
            g.record();
        }
        assert!(matches!(g.decide(Prio::Background, now), Decision::Deny(_)));
        assert_eq!(g.decide(Prio::Foreground, now), Decision::Go);
    }
    #[test]
    fn hard_ceiling_makes_foreground_wait() {
        let mut g = Governor::new(1500);
        let now = Instant::now();
        for _ in 0..24 {
            g.record();
        }
        assert!(matches!(g.decide(Prio::Foreground, now), Decision::Wait(_)));
    }
    #[test]
    fn low_remaining_header_pauses_background() {
        let mut g = Governor::new(1500);
        g.on_headers(Some(30), Some(5));
        assert!(matches!(
            g.decide(Prio::Background, Instant::now()),
            Decision::Deny(_)
        ));
    }
    #[test]
    fn rate_limit_pauses_background_and_delays_foreground() {
        let mut g = Governor::new(1500);
        let now = Instant::now();
        g.on_429(Duration::from_secs(10), now);
        assert!(matches!(g.decide(Prio::Background, now), Decision::Deny(_)));
        assert!(matches!(g.decide(Prio::Foreground, now), Decision::Wait(_)));
    }
    #[test]
    fn redacts_keys() {
        assert_eq!(redact("bad HDEV-1234abcd key"), "bad [redacted] key");
    }
    #[test]
    fn encodes_riot_ids() {
        assert_eq!(enc("Nova Star"), "Nova%20Star");
    }
}
