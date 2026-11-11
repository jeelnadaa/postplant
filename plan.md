# Postplant: Plan

> **Working name: Postplant** (see section 19 for the reasoning and alternatives).
> An open-source, lightweight Valorant companion for Windows: player search, personal analytics, a movable overlay, and a per-map Comp Advisor that suggests an agent and a full team comp ("agent stack").
>
> *Postplant is not endorsed by Riot Games and does not reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games and all associated properties are trademarks or registered trademarks of Riot Games, Inc.*

---

## 0. Brief for the next planner (read this first)

This plan is a strong draft that should be **improved, not just executed**. Challenge the architecture, fix gaps, tighten estimates, and flag anything wrong or unverifiable.

### What changed in this revision (2026-11-11)
- **The project is now open source**, not personal and private. That changes the compliance picture (section 8) and adds release, licensing and contributor work (section 17).
- **Live opponent ranks are in scope (owner decision)**, shown only once the match has started, never in agent select. They ship as an **opt-in, off-by-default** toggle with a mandatory warning, using the local Riot Client. Overwolf is ruled out (section 11).
- **API key entry and a "how to get a key" guide live inside the app** (section 21).
- **Distribution**: Windows installer with uninstaller, portable build, WebView2 handling, cached CI builds on GitHub Releases, in-app auto-updater (section 20).
- **Project tracking**: `PROGRESS.md`, `README.md` and regular git commits (section 22). Static landing and download site for Cloudflare Pages (section 23).
- **Comp Advisor redesigned** (section 14): three modes (solo queue, 5-stack planner, "what should I learn for this map"), an agent knowledge base with role and utility tags, map-specific traits, and honest confidence labels.
- **Meta pipeline redesigned** (section 15) after testing the candidate sources on 2026-11-11. No source gives clean per-map ranked comp stats with sample sizes that we can legitimately fetch, so the pipeline now combines three tiers of evidence and runs **once, centrally, in the repo's CI**, instead of every install scraping websites.
- **Game facts verified** as of 2026-11-11 (section 1.1): current map pool, roster size and newest agent.
- **New features** that cost zero quota (section 16): kill/death heatmaps, ability-usage stats, economy efficiency, agent-pool tracker and more.

### Who and what
- **Users:** any Valorant player on Windows who installs it. Each user brings **their own HenrikDev API key**. The repo never ships a key.
- **Regions:** all HenrikDev regions (`ap`, `eu`, `na`, `kr`, `latam`, `br`), chosen in onboarding. The original owner plays on `ap` (India).
- **Platforms:** the main app builds on Windows, macOS and Linux (Tauri). The **overlay is Windows-only**, because Valorant is Windows-only.
- **Lightweight**: overlay movable, hideable by hotkey, no measurable game impact.

### Requirements (owner's words, condensed)
1. Search any player by Riot ID and see their data.
2. A **movable overlay** with **hotkeys** to show, hide and control it.
3. In a live match, opponents' current and peak rank, only after agent select ends, and only fields that exist. **Planned as an opt-in module: section 11.**
4. In a live match, my own kills and headshots live. **Dropped:** the local client exposes no live kill feed, and Overwolf is ruled out. Shown post-match instead (section 11).
5. All data up to date.
6. Analytics: session tracker with tilt warning; breakdowns by agent, map, side and weapon; first-blood, clutch, multi-kill and eco stats; round timeline; form vs baseline; rank view; teammate stats; repeat players; player notes and tags; friends compare.
7. **Must not use up the API quota.**
8. **Suggested agent and suggested team comp (agent stack)**, based on the map and teammates' picks, **accurate per map**, with data **pulled from the web** by an agent rather than hand-entered.
9. **Export and import** (section 16.1): full `.postplant` backup and an open JSON export; import with a preview, merge or replace; automatic local backups with one-click restore. Moving to a new PC costs zero API requests.
10. **Excluded on request:** goals, crosshair/lineup notes, CSV export (JSON export covers scripts and tools), weekly reports, Discord webhook, tray notifications, leaderboard viewer, overlay profiles.

### Decisions already made
- Stack: Tauri v2 + Svelte + SQLite; HenrikDev API for player data; valorant-api.com for assets (sections 2 and 3).
- Analytics are computed locally, behind a request-budget governor (sections 12 and 13).
- The Comp Advisor takes **manual input** and reads a **meta dataset built by a central pipeline** (sections 14 and 15).

### Still open
1. **License.** Recommended: MIT OR Apache-2.0 (section 17). Owner to confirm.
2. **HenrikDev terms for open-source distribution.** The README says the API is not meant for production use. Ask on their Discord whether a public open-source client where each user supplies their own key is acceptable, and record the answer in `docs/compliance.md` **before the first public release**.
3. **Meta source permissions** (section 15.2). Several sources are technically reachable but their terms were not read. Read each terms page before the pipeline goes live.
4. **Is the match detail endpoint rich enough?** Rounds, kill events, player locations and ability casts. Verify in M0 and adjust features.
5. **Default rank bracket** for meta data, and whether to offer non-competitive queues at all. Suggested default: competitive only.
6. **Name availability** (section 19): check GitHub, crates.io, npm and a basic trademark search before announcing.

### How to improve this plan
- Verify every endpoint and field against live docs before relying on it.
- Replace estimates (quota, RAM, CPU) with measured numbers as soon as M1 exists.
- Add a threat model for the update channel and the meta dataset download.
- Keep the quota rules in section 13 intact. They are a hard requirement.

---

## 1. Scope

### In scope
- **Search**: look up any player by Riot ID (`name#tag`): rank, peak rank, recent matches, agent and map breakdowns, headshot %, ACS, K/D, win rate.
- **My profile**: full history, trends, sessions, rank and RR history.
- **Match detail**: full 10-player scoreboard for any stored match, with a "look up this player" button on each row.
- **Overlay**: small, transparent, always on top, draggable, with hotkeys (section 4). Shows last match, session stats, rank progress and the scoreboard of the match that just ended.
- **Freshness**: "updated X ago" labels and manual refresh everywhere (section 5).
- **Analytics** (section 12) and **extra zero-quota features** (section 16).
- **Quota safety** (section 13).
- **Comp Advisor** (section 14) backed by the **meta pipeline** (section 15).

### Out of scope
Left out because Riot's policy treats them as unapproved, or because they can get accounts banned (section 8):
- Opponent stats in agent select or before the match starts (scouting).
- Reading game memory or injecting into the game. Riot has blocked game-memory access for unknown third-party apps since October 6, 2026.
- Reading the local Riot Client API **outside the opt-in live module**, or during agent select (section 11).
- Store checkers, agent-select automation, instalock, anything that automates input.
- Overlays that cover parts of the map or give real-time "go here now" guidance during rounds.

### 1.1 Verified game facts (checked 2026-11-11; the app must not hard-code these)
| Fact | Value | Source |
|---|---|---|
| Competitive map pool (V26 Act 6, since patch 13.08, Oct 13–14 2026) | Abyss, Bind, Corrode, Haven, Lotus, Summit, Sunset | Several patch summaries agree; one outlier site is out of date. Confirm against Riot patch notes. |
| Last rotation | Bind and Corrode in; Ascent and Split out | Patch 13.08 summaries |
| Next rotation | Expected with V27 Act 1 (projected early January 2027, not announced) | Third-party projection |
| Roster | 30 agents: 9 Duelists, 7 Initiators, 7 Controllers, 7 Sentinels | Agent list sites, Oct 30 2026 |
| Newest agents | **Hush** (Duelist, Oct 27 2026, patch 13.09), Miks (Controller, Mar 2026), Veto (Sentinel, Oct 2025) | Agent release coverage |
| Newest map | Summit (released June 2026, Act 4): three lanes, busy mid, retractable walls on A, B and mid | Map guides |

These facts belong in versioned data files (section 15.5), not in code. When a new act starts, a maintainer PR updates them.

---

## 2. Tech stack

| Part | Choice | Why |
|---|---|---|
| Shell | **Tauri v2** (Rust) | System webview, small installer, transparent always-on-top click-through windows, global shortcuts, signed auto-updater. |
| UI | **Svelte 5** + TypeScript | Tiny bundles, no virtual-DOM cost. Chosen over Solid so contributors find it easier to start. |
| Storage | **SQLite** via `rusqlite` with embedded migrations | Single local file, no server, testable from Rust. |
| Charts | **uPlot** + hand-written SVG | Much lighter than Chart.js. |
| Player data | **HenrikDev Unofficial Valorant API (v4)** | Riot's official VALORANT API does not issue personal keys and needs production approval. |
| Assets | **valorant-api.com** | Agent, map, rank icons and minimap transform values. Cached on disk, refreshed when the game version changes. |
| Meta data | **Postplant meta dataset** (section 15) | Built in CI, reviewed by a person, published as a signed release file. |
| Design system | `design/tokens.css` + `design/components.css` (section 24) | One token file drives the app, overlay and site; six themes. |
| Type and icons | **Satoshi** (UI default; alternatives General Sans, Geist, Inter, Atkinson Hyperlegible), **Big Shoulders Display** (numbers and headings), **JetBrains Mono**; **Lucide** icons | Fonts bundled locally in the app (licences allow it); Lucide is ISC licensed and tree-shakeable. |
| Tests | `cargo test`, Vitest, Playwright for UI smoke tests | Run in GitHub Actions on every PR. |

---

## 3. Architecture

```
┌────────────────────────────────────┐
│ Postplant (one Tauri process)      │
│                                    │
│  Main window    (search, analytics,│
│                  Comp Advisor)     │
│  Overlay window (transparent)      │
│                                    │
│  Rust core                         │
│   ├─ api/      Henrik client       │  budget governor + retry
│   ├─ cache/    SQLite, TTLs        │  stale-while-revalidate
│   ├─ advisor/  scoring engine      │  pure functions, no I/O
│   ├─ meta/     dataset loader      │  verifies hash + signature
│   ├─ hotkeys/                      │
│   └─ settings/ + OS keychain       │
└──────────────┬─────────────────────┘
               │ HTTPS
   HenrikDev API · valorant-api.com · GitHub Releases (meta dataset, app updates)

Separately, in the repo's CI (never on users' machines):
   meta-pipeline/  → fetch sources → normalize → validate → open PR → human review → release
```

All network calls go through the Rust core, so there is one rate limiter and one cache, and the API key never reaches the webview.

### Repository layout
```
postplant/
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ api/         henrik.rs, assets.rs, governor.rs
│  │  ├─ db/          migrations/, queries.rs
│  │  ├─ advisor/     scoring.rs, stack_planner.rs, explain.rs
│  │  ├─ meta/        loader.rs, verify.rs
│  │  ├─ overlay.rs
│  │  ├─ hotkeys.rs
│  │  └─ settings.rs
│  └─ tauri.conf.json
├─ src/
│  ├─ app/            search, profile, match, analytics, advisor, settings
│  ├─ overlay/
│  └─ lib/            shared types, formatters, i18n
├─ data/              agents.json, maps.json, map-pool.json (reviewed by people)
├─ meta-pipeline/     source adapters, fixtures, validation, CLI
├─ docs/              compliance.md, meta-sources.md, privacy.md, architecture.md
├─ site/              static landing + download pages (Cloudflare Pages)
├─ PROGRESS.md        milestone checklist, updated with every meaningful commit
├─ README.md
├─ .github/           workflows, issue templates, CODEOWNERS
├─ LICENSE-MIT, LICENSE-APACHE, CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md
└─ plan.md
```

---

## 4. Overlay and hotkeys

### Window setup
- Separate Tauri window: `transparent`, `decorations: false`, `alwaysOnTop`, `skipTaskbar`, `shadow: false`, no focus on show.
- **Valorant must run in Windowed Fullscreen (borderless).** Overlays do not draw over exclusive fullscreen. Onboarding says so in one line, with a screenshot.
- Two modes:
  - **Locked (default)**: click-through (`set_ignore_cursor_events(true)`), so clicks reach the game.
  - **Edit**: drag to move, edge handles to resize, opacity slider.
- Position, size, opacity, scale and visibility persist and are restored on launch. A saved position that is off-screen (monitor change) resets to the default corner.
- **Respect the policy line:** the overlay shows post-match and session information. It never covers the minimap, the kill feed or the crosshair area by default, and it never gives in-round instructions.

### Default hotkeys (all rebindable, registered globally)
| Action | Default |
|---|---|
| Show / hide overlay | `Ctrl+Shift+H` |
| Toggle edit mode | `Ctrl+Shift+M` |
| Reset overlay position | `Ctrl+Shift+R` |
| Cycle overlay panel | `Ctrl+Shift+Tab` |
| Force refresh | `Ctrl+Shift+F5` |
| Open main app | `Ctrl+Shift+O` |
| Open Comp Advisor panel | `Ctrl+Shift+A` |

If a shortcut fails to register, Settings shows a warning and offers a rebind. Test plan: register, trigger and unregister every hotkey with Valorant focused in borderless mode on 60 Hz and 240 Hz monitors, and with Discord and NVIDIA overlays running.

### Overlay panels (about 260×140 px)
1. **Last match**: result, map, agent, ACS, K/D/A, headshot %, RR change.
2. **Session**: today's W/L, average ACS, headshot %, K/D, net RR, tilt warning.
3. **Rank**: current rank, RR, peak rank, RR sparkline.
4. **Post-match scoreboard** (auto-shows when a new match is detected, hides after N seconds): all 10 players with rank, agent and score, plus your notes and "played before" badges.
5. **Comp Advisor (compact)**: section 14.

---

## 5. Data freshness and caching

| Data | Cache policy |
|---|---|
| Finished match details | Immutable. Stored forever, never refetched. |
| My match list | Driven by the adaptive polling in section 13. Fetch only matches not already stored. |
| Player rank / MMR | TTL 2 min for me, 10 min for searched players. Manual refresh bypasses it. |
| Account (name#tag → PUUID) | Re-resolve on every search because names change. Always key data by PUUID. |
| Rank history | Snapshot after every detected match and on season change, tagged with season ID. |
| Agent, map and rank assets | On disk, revalidated when the game version changes. |
| Meta dataset | Checked at most once a day; downloaded only when a newer signed version exists (section 15.6). |

Behaviour: stale-while-revalidate; "Updated 2m ago" and a refresh button on every card; `X-RateLimit-Remaining` and `Retry-After` respected; priority queue (mine, then user-initiated, then background); offline banner when the API is down; no bulk lookups. Confirm endpoint versions in `docs.henrikdev.xyz` at the start of M1.

---

## 6. Data model (SQLite)

```sql
players(puuid PK, name, tag, region, card_id, last_seen_at)

matches(match_id PK, started_at, map_id, queue, season_id, game_version,
        rounds_played, winning_team, raw_json_zstd)

match_players(match_id, puuid, team, agent_id, score, kills, deaths, assists,
              headshots, bodyshots, legshots, damage_made, damage_received,
              rank_tier, ability_casts_json, PRIMARY KEY(match_id, puuid))

mmr_snapshots(puuid, taken_at, season_id, tier, rr, peak_tier, elo)

rounds(match_id, round_no, winning_team, result_code, planter, defuser,
       plant_site, PRIMARY KEY(match_id, round_no))

kills(match_id, round_no, time_in_round_ms, killer, victim, weapon_id,
      killer_x, killer_y, victim_x, victim_y,           -- for heatmaps (section 16)
      PRIMARY KEY(match_id, round_no, time_in_round_ms, victim))

round_player_stats(match_id, round_no, puuid, score, damage, kills,
                   headshots, bodyshots, legshots, econ_spent, loadout_value,
                   PRIMARY KEY(match_id, round_no, puuid))

player_notes(puuid PK, tags_json, note, updated_at)     -- local only, never uploaded

agent_pool(puuid, agent_id, comfort INTEGER,            -- 0 = won't play .. 3 = main
           PRIMARY KEY(puuid, agent_id))                -- for me and for stack friends

-- Meta data (section 15). Loaded from the signed dataset; never written by the player API.
meta_snapshots(snapshot_id PK, dataset_version, built_at, game_patch, act_id,
               signature_ok, notes)
meta_agent_map(snapshot_id, tier, source_id, map_id, agent_id, rank_bracket,
               pick_rate, win_rate, atk_win_rate, def_win_rate, n,
               PRIMARY KEY(snapshot_id, tier, source_id, map_id, agent_id, rank_bracket))
meta_comps(snapshot_id, tier, source_id, map_id, comp_key, agents_json,
           pick_rate, win_rate, n, PRIMARY KEY(snapshot_id, tier, source_id, map_id, comp_key))
meta_map_profile(snapshot_id, map_id, role_mix_json, utility_needs_json,
                 confidence, PRIMARY KEY(snapshot_id, map_id))

settings(key PK, value)
fetch_log(endpoint, key, fetched_at, status)
api_usage(minute_bucket, units_used, source)

-- Export / import (section 16.1)
schema_info(version INTEGER, app_version, created_at)   -- checked on import and before migrations
backups(id PK, path, kind, created_at, size_bytes,     -- kind: weekly | before_import | before_migration | manual
        sha256, counts_json)
import_log(id PK, file_name, format, mode, source_puuid, -- format: backup | json; mode: merge | replace
           imported_at, counts_json, conflicts_json)
```

Headshot % is derived: `headshots / (headshots + bodyshots + legshots)`. A session is computed, not stored: consecutive matches with gaps under 2 hours. Store raw match JSON compressed (zstd) so new features can be computed later from existing data without refetching.

---

## 7. Milestones and cut line

### Week-one cut (first public pre-release, `v0.1`)
M0, M1, M2 and M3 below, plus the Comp Advisor running on the **knowledge base only** (section 14, "Tier 3" mode). That is a useful, honest product before any meta scraping exists.

### M0: Setup (half a day)
- [ ] Tauri v2 + Svelte + TypeScript project, CI running lint and tests on Windows.
- [ ] Release workflow skeleton with caching (section 20), producing an unsigned installer and portable zip from day one.
- [ ] Get a HenrikDev key; store it in the OS keychain.
- [ ] Call account, MMR, match-list and match-detail endpoints with a real Riot ID. Record field names in `docs/api-notes.md`, and confirm whether rounds, kill events, **player locations** and **ability casts** are present.

### M1: Data layer (1–2 days)
- [ ] Budget governor (section 13), built before any feature that calls the API.
- [ ] Henrik client with retries and 429 handling. Deduplicate in-flight requests.
- [ ] SQLite schema and migrations; cache layer with TTLs.
- [ ] Sync job for my recent matches.

### M2: Search app (2–3 days)
- [ ] Onboarding: region, Riot ID, API key entry with the in-app "how to get a key" guide (section 21), agent pool.
- [ ] Search with recent history; profile page; match history; match detail; "look up this player".
- [ ] Trend charts.

### M3: Overlay (2 days)
- [ ] Window modes, hotkeys with rebinding, persistence, panels, new-match detection.

### M2b: Analytics (2–3 days)
- [ ] Everything in section 12, trimmed to what M0 showed the data supports.

### M4: Comp Advisor v1 (2–3 days)
- [ ] `data/agents.json` and `data/maps.json` knowledge base, reviewed by people (section 14.2).
- [ ] Scoring engine as pure Rust functions with fixture tests.
- [ ] Solo-queue mode and stack planner UI, plus the compact overlay panel.

### M5: Meta pipeline (3–5 days, can start after M0)
- [ ] Source review recorded in `docs/meta-sources.md` (section 15.2).
- [ ] Adapters, normalization, validation, CI workflow that opens a PR.
- [ ] Signed dataset release and app-side loader (section 15.6).
- [ ] Advisor upgraded to use Tiers 1 and 2.

### M6: Extras (section 16), polish and v1.0
- [ ] Every feature in section 16, each trimmed to what the M0 field check showed:
  - [ ] Kill/death heatmaps per map (by side, filter by agent)
  - [ ] Opening duel map and win rate
  - [ ] Ability usage per round vs your own average
  - [ ] Economy efficiency (damage per 1,000 credits; eco/force/full-buy results)
  - [ ] Agent pool tracker (results per agent and map; feeds Comp Advisor comfort)
  - [ ] Duo synergy (per agent pair with each friend)
  - [ ] Patch impact (before/after a patch that changed your agent)
  - [ ] Post-match summary card
  - [ ] English + Hindi UI (`lib/i18n`, every string translatable from day one)
  - [ ] Colour-blind-safe palettes and scalable overlay text
  - [ ] Export / import and automatic backups (section 16.1): `.postplant` backup + JSON export, preview, merge/replace, conflict rules, anonymised export, weekly + safety backups, restore from onboarding
- [ ] Settings, error states, tray icon, start with Windows, performance pass (section 9).
- [ ] Code signing, auto-updater (section 20), first stable release.

### Site (can run any time)
- [ ] Landing and download pages in `site/` (section 23); deploy to Cloudflare Pages.

### M7: Live match module (opt-in, 2–3 days, after v1.0 is stable)
- [ ] Re-read Riot's VALORANT policy and the current state of the local client endpoints.
- [ ] Phase detection: activate only when the match phase is in-game; do nothing during pre-game (agent select).
- [ ] Read players and agents for the current match; fetch ranks through the governor's live budget (section 13).
- [ ] Live overlay panel with the "show only what's available" rules: buy-phase compact panel, hold-Tab full panel, Ctrl+Shift+L toggle; per-player states (loading, queued, private profile, hidden name, unranked, error); hidden-name players never looked up.
- [ ] Opt-in screen with the risk notice; module disabled by default and removable at build time (`--no-default-features`).

---

## 8. Compliance and safety (open-source edition)

Making the project public changes the risk. A private tool affected one account; a public tool affects every user who installs it, and Riot can act on the tool as a whole.

### Riot policy, as it applies here
- Riot's VALORANT developer policy lists **scouting** (opponent stats before a match starts) as unapproved, and says that showing a player's stats is an approved use only **with player opt-in**. It also lists apps "designed for personal use only" as unapproved, which no longer applies to us.
- Riot does not issue personal VALORANT keys, and its developer FAQ says it does not accept GitHub repositories in place of a working product. We therefore use HenrikDev, not Riot's API, and do not apply for a Riot production key until there is a working release.
- Since October 6, 2026, Riot blocks game-memory access for unknown third-party apps. Memory reading is out of scope permanently.
- Riot's published policy rejects overlays that give real-time performance guidance or block parts of the map. Our overlay stays post-match and session-focused.

### Rules for this codebase
1. **No scouting in agent select.** Opponent data is shown only for a **finished** match (on click) or, in the opt-in live module, once the match phase is in-game. Never during agent select, and the code checks the phase, not a timer.
2. **No bundled keys.** Each user enters their own HenrikDev key. CI uses a secret key only for tests, with recorded fixtures by default.
3. **Never** ask for or store a Riot password, read game memory or inject into the game. Only the opt-in live module may read the local client's lockfile and session tokens; it keeps them in memory, sends them only to Riot's own endpoints, and never writes them to disk or logs.
4. **No bulk scraping** of players. HenrikDev's rules disallow large analytics projects, and section 13 enforces this in code.
5. **No telemetry.** Nothing leaves the machine except the API calls listed in `docs/privacy.md`. Notes and tags never leave the machine.
6. **Trademarks.** The name, logo and icons must not use "Valorant", "Riot" or Riot logos. Agent and map art comes from valorant-api.com at runtime and is not committed to the repo. Include Riot's "not endorsed" notice in the README and the About screen.
7. **Before each release**, re-read the Riot VALORANT policy, the HenrikDev terms and each meta source's terms, and update `docs/compliance.md` with the date.

---

## 9. Performance targets

| Metric | Target |
|---|---|
| Overlay idle CPU | under 1% |
| Total RAM (app + overlay) | under about 150 MB |
| Overlay render | no animation loops while idle; update only on data change |
| Polling | pauses when the overlay is hidden and the game is not running |
| Installer size | under about 15 MB |
| Cold start | under 2 s |
| Advisor response | under 50 ms per input change (all local) |

Measure these on a low-end machine (4 cores, 8 GB RAM, integrated GPU) as well as a gaming PC, and publish the numbers in the README.

---

## 10. Definition of done (v1.0)

- A new user can install, enter a key and a Riot ID, and see their stats in under 2 minutes.
- Searching any Riot ID shows rank, peak and recent matches in a couple of seconds.
- After a match, the overlay shows the result and scoreboard automatically.
- The overlay can be hidden, shown, moved and reset by hotkey without alt-tabbing.
- Every number shows when it was updated and can be refreshed.
- No request storms; quota use stays inside the rules in section 13 without user effort.
- The Comp Advisor returns suggestions for every map in the current pool, each with its evidence tier, sample size, data date and confidence.
- Releases are signed, reproducible from a tag, and documented.

---

## 11. Live match module

### Status: planned, opt-in, off by default (owner decision, 2026-11-11)
Live opponent ranks (requirement 3) need data from the running game. The web API cannot tell us who is in a match until it ends.

### How Tracker.gg does it, and why we can't copy that
Tracker.gg's in-game app runs on **Overwolf**, which has a game-events agreement with Riot, and Tracker Network is an approved Riot API partner. Their live data comes through those approvals. **The owner has ruled out Overwolf (2026-11-11)**, and we don't have a Riot partnership, so that route is closed.

### Our route: the local Riot Client
Read the Riot Client's lockfile for the local port and session, then ask Riot's match service who is in the current match and which agents they play. This is how open-source rank-checker tools work.
- No extra runtime; fits the Tauri app.
- **Unsupported by Riot** and can break with any client update. Riot has warned that apps showing information the client hides can lead to penalties, and its policy treats showing other players' stats as needing their opt-in. The risk falls on each user who turns it on, which is why the warning below is mandatory.
- Gives **no live kill feed**, so requirement 4 (my own live kills and headshots) is **dropped**. Those numbers appear in the post-match panel about a minute after the match.

### The toggle and the warning
- Settings → **Live match (experimental)**, a single on/off switch. **Off by default**, and it can be turned off again at any time with immediate effect (all live polling and in-memory tokens are dropped).
- Turning it on opens a warning that must be acknowledged, in plain words:
  > Live match uses the Riot Client running on your PC. Riot does not support this, it may stop working after any game update, and Riot may treat third-party tools that show other players' information as a rules violation. This could put your account at risk. Postplant never reads game memory and never runs during agent select. Turn this on only if you accept that risk.
- The acknowledgement is stored with a date and the app version, and is shown again after any update that changes the live module.

### Guardrails
- **Off by default.** Turning it on shows a plain-language notice: unsupported by Riot, may break, may carry account risk, use at your own risk. The choice is stored with a date.
- **Phase gate:** active only when the client reports the in-game phase. During agent select the module does nothing, not even fetch PUUIDs. This is checked in code and covered by tests with recorded client responses.
- **Hidden names are respected:** if a player has hidden their name (streamer mode), the app shows the agent and "Hidden name · Private", and makes **no** lookup for that player.
- **Private profiles are labelled, not guessed:** fields HenrikDev can't return show "Private".
- **Read only:** no requests that change anything, no agent-select calls, no automation.
- **Quota:** ranks come from HenrikDev using PUUIDs, fetched through the governor's live budget (section 13), cached for the match. Calling Riot's rank endpoints with the user's session tokens is avoided, to keep token use to the minimum.
- **Compile-time switch:** a Cargo feature (`live`) so packagers and cautious users can build without it.
- **Kill switch:** if a Riot policy change or a breaking client update lands, a dataset flag (section 15.6) can disable the module remotely until a fixed release ships.

### What the live panel shows
Everything below appears only when Live match is on (warning accepted) and only after the match has started. Each field loads and renders on its own, so one missing field never blanks a row. Nothing is guessed.

| Field | Source | Cost |
|---|---|---|
| Agent, team, name#tag | Local Riot Client (current match) | Free |
| Account level | Local Riot Client, unless the player hides it | Free |
| Current rank + RR | HenrikDev MMR by PUUID | 1 request per player |
| Peak rank (with the act it was reached in) and last act's final rank | Same MMR response | Included |
| Recent form: win %, K/D, HS %, ACS and most-played agents over their last 10 competitive matches | HenrikDev stored matches by PUUID | 1 request per player; can be turned off |
| Your notes and tags, "played with / against N×", last result together | Local database | Free |
| "New account" tag | Account level under 30 | Free |

**Per-player states (each designed in `design/overlay.html`):**
- **Loaded:** all fields shown.
- **Loading:** skeleton bars per field; ranks load before recent form.
- **Queued:** waiting for quota (requests are spread over at least 60 s); shows a clock instead of a value.
- **Private profile:** HenrikDev returns nothing for that player. Name, agent and level still show (from the client); rank, peak, last act and form show **Private** with a lock.
- **Hidden name (streamer mode):** the agent shows, the name reads **Hidden name**, every other field reads **Private**, and **no lookup is made**.
- **Unranked:** no competitive rank this act; peak still shows if it exists.
- **Error / paused:** a dash with a retry hint; the panel never shows stale data from another player.

**Where it shows:**
- **Buy phase of round 1:** a compact panel (agent, name, current rank) auto-shows on the left for 25 s, clear of the minimap.
- **Hold Tab:** while Tab is held in game, the full panel appears docked on the right of the screen, beside Valorant's own scoreboard. Valorant's scoreboard already has the live K/D/A, score and economy; Postplant adds what it lacks: current and peak rank, last act, level, recent form and your notes. Rows can't line up with the game's rows because the game re-sorts by score during the match and Postplant cannot see that order, so each team is sorted by rank and every row starts with the agent icon to match by eye.
- **Hotkey:** `Ctrl+Shift+L` toggles the same full panel, for people who turn off the Tab trigger.

**Tab detection:** a passive check of the Tab key state, only while Valorant is the focused window and Live match is on. It never intercepts, blocks or remaps the key, and nothing is sent to the game. It's a separate setting ("Show while Tab is held", on by default when Live match is on), so anyone who prefers not to have it can use the hotkey instead.

**Not available, by design:** other players' live K/D/A, score, credits or loadouts (the local client doesn't expose them; Valorant's own scoreboard shows them), and party detection (would need reading friends' presence data; left out).

---

## 12. Analytics (all computed from stored data)

None of these call the API; they read SQLite only, so they cost **zero quota**.

### Performance
- **Session tracker**: W/L, net RR, ACS, K/D, headshot %. **Tilt warning** after N losses in a row (default 3).
- **Breakdowns** by agent, map, side and weapon. Per-weapon headshot % only where kill events and damage data can be matched (confirm in M0).
- **First-blood rate, clutches, multi-kills**, eco vs full-buy performance.
- **Round timeline** per match: kills, plants, defuses, trades.
- **Form vs baseline**: rolling ACS and K/D against the season average; best time of day and day of week.
- **Rank view**: RR graph, estimated games to next rank, peak vs current, act-by-act comparison.

### People
- **Teammate stats**: win rate with each duo, best agent pairs together.
- **Repeat players**: "you've played this person before", matched by PUUID locally.
- **Notes and tags** (good duo, toxic, smurf?), local only.
- **Friends compare**: side by side from shared matches (free); anything else fetched only on open and cached 10 minutes.

### Excluded
Goals, crosshair/lineup notes, CSV export, weekly reports, Discord webhook, tray notifications, leaderboard viewer, overlay profiles.

---

## 13. Quota budget (hard requirement)

### Facts
- HenrikDev limits are per key: 30 requests/minute for a Basic key and 90 for Advanced (confirm on the dashboard).
- Since v4, a request costs **1 + one unit per background request the server makes to Riot**. Matches already stored on their server cost nothing extra.
- Data in local SQLite costs nothing.

### Rules
1. **Local first.** Every screen reads SQLite first.
2. **Match details are fetched once, ever.** Identical in-flight requests are merged.
3. **Cheap "anything new?" check**: latest match list at the smallest page size; fetch details only for new IDs.
4. **Adaptive polling**: only while Valorant is running (process-exists check, nothing read from it) or the overlay is visible. Every 3 min, backing off 3 → 6 → 12 min while nothing changes; stop 10 min after the game closes.
5. **Rank snapshots** only after a new match, plus once at start if the last one is over 30 min old.
6. **Other players on demand only**, cached 10 minutes. The one exception is the opt-in live module (section 11): once per match it may fetch rank (1 request) and, if enabled, recent form (1 request) for up to 9 players: at most 18 requests, about 18–36 units. They are spread over at least 60 seconds, ranks first, skipping hidden-name players and anyone cached in the last 10 minutes. It counts against a separate per-match cap and never pushes total use above the 80% ceiling.
7. **Backfill is manual and bounded** (default 20), resumable, with its cost shown first.
8. **Search is cached**; refresh forces a call.
9. **The Comp Advisor and the meta pipeline make zero Henrik calls.**

### Budget governor (built in M1)
| Mechanism | Setting |
|---|---|
| Foreground reserve | User-triggered requests always go first. |
| Background ceiling | At most 50% of the per-minute limit. |
| Hard ceiling | Never above 80% of the limit. |
| Header checks | Below 20% remaining, pause background work until reset. |
| 429 handling | Obey `Retry-After`, pause background, show a notice. |
| Daily soft cap | Default 1,500 units/day, configurable. |
| Visibility | Settings shows "used this minute / today". |

### Estimate (verify with real headers in M1)
A 3-hour, 6-match session: about 60 list checks, 6 detail fetches and a few rank snapshots, so **roughly 100–150 units**, with peaks of a few units per minute.

### Tests
- [ ] Log every call and its real cost over a full session and compare.
- [ ] Simulate a 429; confirm background pauses and recovers.
- [ ] Confirm zero calls while the game is closed and nothing is open.

---

## 14. Comp Advisor (suggested agent + suggested agent stack)

### 14.1 What it does
Three modes, all using the same engine.

| Mode | Input | Output |
|---|---|---|
| **Solo queue** (default) | Map, side you start on (optional), teammates' picks so far (0–4), your agent pool | Top 3 agents for you, each with reasons; top 3 full comps that include the agents already picked |
| **Stack planner** (5-stack or duo) | Map, each friend's agent pool and comfort | Best assignment of agents to people, plus 2 alternatives |
| **Learn for this map** | Map, your current pool | 1–2 agents worth adding to your pool for this map, and why |

**Input is manual by design.** You pick the map and click teammates' agents on a grid. The app does not read agent select, the local client or anything from the game, and it never selects or locks an agent. It is a reference you read and decide from.

The compact overlay panel (`Ctrl+Shift+A`) shows a 5-slot row: click a slot, click an agent, and suggestions update instantly.

### 14.1a Only agents you own
The advisor **never suggests an agent the user can't pick**. Ownership is stored in `agent_pool` (comfort 0 = owned but won't play; a missing row = not owned).

| Source | When | Notes |
|---|---|---|
| Starter agents | Always | Brimstone, Jett, Phoenix, Sage, Sova are on every account |
| Agents in stored match history | After each sync | If you've played it, you own it (or had it on a free trial; the user can untick) |
| Riot Client entitlements | Only when Live match (section 11) is on | Read from the local client's owned-items list; same opt-in, same warnings |
| Manual | Onboarding step "Agents you own" and Settings → Comp Advisor | Final say; auto-detection never removes a manual tick |

UI rules:
- Locked agents show greyscale with a lock and a tooltip ("Not on your account; tap to mark as owned"). In the teammate-pick grid they stay selectable, since teammates can own them.
- Solo suggestions and full comps only include owned agents in your slot. The stack planner uses each friend's own pool.
- A newly released agent appears locked until it shows up in your matches or you tick it.
- Learn-a-map may name an agent you don't own, clearly labelled "Not owned: unlock to try".

### 14.2 Agent knowledge base (`data/agents.json`, reviewed by people)
This is what keeps advice sensible when statistics are thin, for example for Hush right after release.

```jsonc
// Shape only; the tag values and patch below are placeholders, not reviewed data.
{
  "agent_id": "<valorant-api uuid>",
  "name": "Omen",
  "role": "Controller",
  "utility": {                    // 0–3 per tag, reviewed by maintainers
    "smokes": 3, "flash": 2, "recon": 0, "stun_or_concuss": 0,
    "entry": 0, "site_anchor": 0, "flank_watch": 1, "heal_or_res": 0,
    "wall": 0, "post_plant": 1, "mobility": 2, "info_denial": 0
  },
  "flex_roles": [],               // e.g. Viper: ["Sentinel"]
  "released": "2020-06-02",
  "last_changed_patch": "13.06"
}
```

And `data/maps.json` holds per-map traits, for example:
- **Bind**: no mid, two one-way teleporters linking the sites; rewards a second controller-style smoker or wall and strong site anchors.
- **Summit**: three lanes, busy mid, retractable walls on A, B and mid; pro teams have favoured Neon and Fade with two sentinels to hold walls (Champions 2026).
- **Abyss**: open map edges where players can fall; mobility and recon value.

Each map entry lists `utility_needs` (weights per utility tag) and a typical `role_mix`. Initial values come from Tier 1 data plus maintainer review, and each PR that changes them must cite sources.

### 14.3 Scoring
For each candidate agent *a* in your pool and not already picked:

```
score(a) = w1 * map_strength(a, map, bracket)   -- shrunk win rate on this map
         + w2 * comp_fit(a, team, map)          -- how much a closes the gap to the map's utility_needs and role_mix
         + w3 * synergy(a, team, map)           -- pair data where it exists, else utility overlap/complement
         + w4 * comfort(a, map)                 -- your own results with a on this map
         - penalties                            -- third duelist, no smokes yet, very low n
```

- **Shrinkage**: Bayesian shrinkage toward the agent's overall rate, with the prior strength set by `n`. A lower bound of a Wilson interval is a fine first version.
- **Utility coverage is the backbone.** `comp_fit` uses the gap between the team's summed utility vector and the map's `utility_needs`. This works even with zero statistics.
- **Comfort**: blended in only after 8+ games on that agent and map; otherwise overall agent results; otherwise zero.
- **Side-aware**: if Tier 1 data gives attack/defence splits, show which side the pick favours.
- **Weights** live in `data/advisor-weights.json` and are tuned offline against held-out data from the pipeline. Never hard-code them.

**Stack planner**: enumerate comps from Tier 1/2 data plus the top-scoring knowledge-base comps, then solve the assignment of agents to players (5×5, brute force or Hungarian algorithm) maximizing comp score + Σ comfort. Under a millisecond.

### 14.4 Output rules
- Show **top 3 agents** and **top 3 comps**. Every item shows a **one-line reason** ("no smokes yet; Bind needs two", "+2.1% win rate on Lotus in Diamond+, n = 4,210"), its **evidence tier**, **sample size**, **data date** and **confidence** (High / Medium / Low).
- Below the minimum sample (default 200 for agents, 100 for comps), statistics are hidden and the item falls back to knowledge-base reasons, labelled "based on role coverage, not win-rate data".
- **Pro and ranked data are never blended silently.** The UI says which one each number came from.
- New or recently reworked agents carry a "limited data" badge (currently Hush).
- A banner appears when the dataset is older than 21 days or the game patch has moved on since it was built.
- Explanations never say "you must". The panel's footer reads: "Suggestions from aggregate data. Play what you play well."

### 14.5 Accuracy ("accurate per map")
- All numbers are per map, per patch or act, per rank bracket, per mode.
- Only maps in `data/map-pool.json` are offered. If the user's own recent competitive matches include a map that isn't in the pool, the app flags the dataset as out of date. (valorant-api.com lists maps but does not say which are in rotation, so the pool cannot come from there.)
- Everything is keyed by valorant-api.com UUIDs, so renames and new agents don't break joins.
- Cross-check rule: for each map, compare the top 5 agents across at least two sources where possible. If they disagree by more than 3 percentage points in win rate, mark that map Low confidence.
- **Tests**: fixture-based tests where adding a teammate changes suggestions sensibly. For example, after two duelists, a third duelist must not rank first; with no controller on Bind, a controller must appear in the top 3; with Hush and no data, the reasons must mention role coverage only.

---

## 15. Meta pipeline (where the per-map data comes from)

### 15.1 Why it runs centrally
Per-map agent and comp statistics need large samples. Building them from your own matches gives tiny, biased numbers, and bulk-collecting other players' matches through HenrikDev breaks its rules. So the data comes from public aggregate sources.

For an open-source app it is better to **fetch once, centrally**, than have every install scrape the same websites:
- far fewer requests to each source (one weekly CI run instead of thousands of clients);
- a person reviews every data change before users see it;
- every user sees the same, versioned numbers, which makes bug reports reproducible.

"Pulled from the web by an agent" means: a search step (a person or an AI agent such as Claude, using web search) **finds and reviews** sources and records them in `docs/meta-sources.md`; the CI pipeline then **fetches only those approved pages** on a schedule. Search engines are for discovery, not a data feed.

### 15.2 Source review (tested 2026-11-11)
| Source | What it has | Access test | Decision |
|---|---|---|---|
| **VLR.gg** event agent pages | Pro play, **per-map** agent pick %, attack/defence win % per map, map sample sizes (Champions 2026: Summit n=18, Lotus 15, Sunset 12, Haven 10, Abyss 8) | Page loads; robots.txt only disallows `/search/auto` and `/rr/`. No API. | **Tier 1 candidate** once terms are read. Small samples per event, so pool several events per patch. |
| **Metabot.gg** | Ranked comps with win rate and **n** (e.g. top comp 57.1%, n = 1,428), but **global, not per map**; rank filter selectable | Page loads; robots.txt allows the comps path and disallows `/api/` | **Tier 2 candidate** (global comp strength) once terms are read. Do not call `/api/`. |
| **vstats.gg** | Act 6 comp and duo tier lists; Overwolf-collected ranked data | Pages render only with JavaScript; robots.txt disallows filter query variants (`&map=`, `&rank=`) | **Not automated.** Manual reference only, unless they offer an export or API on request. |
| **Tracker.gg Insights** | Per-map agent stats, last 14 days, ranked | Returned **403** to automated fetch | **Excluded from automation.** Do not bypass. Tracker Network's developer API is free for hobby projects; ask whether it includes insights data. |
| **ValoHub** | Per-map comps with win/pick % | Loads, but states its figures are estimates; last reviewed Patch 12.10 / Act 3 | **Excluded as data.** Not measured, and out of date. |
| **Gametree, Hotspawn, tier-list articles** | Editorial per-map picks | Articles | Useful only as a sanity check during human review. |

**Conclusion:** no source found offers per-map **ranked comp** statistics with sample sizes that we can fetch legitimately. The design must not pretend otherwise.

### 15.3 Three evidence tiers
| Tier | Content | Source | Shown as |
|---|---|---|---|
| **1. Pro per-map** | Agent pick and win rates per map, attack/defence splits, observed 5-agent comps | VLR.gg events in the current patch window (pooled) | "Pro play, n maps" |
| **2. Ranked global** | Agent and comp win rates by rank bracket, not per map | Metabot (or a better source found later) | "Ranked, all maps, n matches" |
| **3. Knowledge base** | Role and utility coverage per map | `data/agents.json` + `data/maps.json` | "Role coverage" |

The engine combines them: Tier 1 decides **what each map wants** (role mix and staple agents), Tier 2 adjusts for **ranked strength** of agents and comps, Tier 3 fills gaps and explains. If a per-map ranked source becomes available (for example, through Tracker Network's API or a data agreement), it becomes Tier 0 and outranks the others.

### 15.4 Pipeline steps (`meta-pipeline/`, runs in GitHub Actions)
1. **Trigger**: weekly cron, on game-version change (polled from valorant-api.com), or manual dispatch.
2. **Fetch** only URLs listed in `docs/meta-sources.md`. Identifiable user agent with a link to the repo; at most a few requests per source; cache; exponential backoff; stop on any 403 or 429 until the next run.
3. **Parse** with one adapter per source, tested against saved HTML fixtures.
4. **Normalize** names to valorant-api.com UUIDs; tag each row with tier, source, URL, fetched_at, patch, bracket, n.
5. **Validate**: every agent and map resolves; rates are in plausible ranges; per-map pick rates sum near the expected total; sample minimums met; cross-source agreement within tolerance; no map outside `map-pool.json`.
6. **Diff and open a PR** with a human-readable summary ("Lotus: Omen pick rate 81% → 76%; new comp Neon/Fade/Omen/Viper/Vyse n=9").
7. **Human review and merge.** Merging tags a dataset release.

### 15.5 Data files in the repo
```
data/
  agents.json        knowledge base (people edit, cite sources)
  maps.json          map traits and utility_needs
  map-pool.json      {"act_id": "...", "since_patch": "13.08", "maps": [...uuids]}
  advisor-weights.json
meta/
  2026-11-11_13.09/  tier1.json, tier2.json, manifest.json (hashes, sources, counts)
```

### 15.6 How the app gets the data
- Each app release **bundles** the latest dataset, so the advisor works offline from first launch.
- With the user's permission (on by default, can be turned off in Settings), the app checks GitHub Releases for a newer dataset **at most once a day**. It verifies the hash and a minisign signature before loading. A failed check keeps the previous dataset.
- Manual import of a dataset file (same schema) for people who want to build their own.

### 15.7 Risks
- Sites change markup and break adapters: fixture tests plus a "data is X days old" banner.
- Terms may forbid automation: decided per source and recorded; a forbidden source is dropped, never worked around.
- Pro data reflects coordinated teams, not solo queue; always labelled as pro.
- New agents and maps have thin data; say so instead of guessing.

### 15.8 Acceptance criteria
- For every map in the pool, the advisor returns top agents and comps, each with tier, n, date and confidence.
- Pipeline runs offline from fixtures in CI and passes.
- No Henrik requests from the advisor or pipeline.
- Every number in the UI can be traced to a source URL in the manifest.

---

## 16. Additional features (zero quota, from data already stored)

Everything here reads the local database only. Each one depends on what M0 finds in the match detail response.

| Feature | What it shows | Data needed |
|---|---|---|
| **Heatmaps per map** | **Layers:** Me / My team / Enemies. **Events:** kills, deaths, first bloods, round-ending kills, spike plants, spike defuses. **Filters:** side, agent, round type (pistol, eco, full buy), won/lost rounds. Only from matches you played (your teammates and opponents in those matches); no extra API calls | Kill events carry every player's location at the moment of the kill; rounds carry plant/defuse locations; valorant-api.com minimap transform values |
| **Where rounds end** | For each map and side: where the last kill of the round happens, which site rounds are decided on, and whether your team wins or loses there | Same as above |
| **Opening duel map** | Where your first engagements happen and how often you (and your team) win them | Same as above |
| **2D round replay** | Step through each round on the minimap: all 10 players at every kill, plant and defuse, with view direction, weapon and the kill feed. Positions are known only at those moments, so it steps between snapshots; it does not invent movement in between | Kill events with all players' locations and view direction; plant/defuse events |
| **Death review** | Every one of your deaths: where, killed by whom and with what, time in round, whether it was traded within 3 s, distance to the nearest teammate (isolation), your loadout value vs theirs | Kill events + round economy |
| **Trade stats** | Untraded-death %, average time to trade, for you and your team; best trading partners | Kill events |
| **Pattern notes** | Plain-language notes found in your recent matches, e.g. "first to die on B Main in 6 of 11 attack rounds on Lotus; 5 untraded". Each note links to the rounds it came from. Factual and post-match only; never in-round advice | All of the above |
| **Ability usage** | Casts per round per ability vs your own average on that agent | `ability_casts` per player |
| **Economy efficiency** | Damage per 1,000 credits, win rate by loadout value, eco/force/full-buy results | Round economy data |
| **Agent pool tracker** | Your results per agent and map, with a "comfort" score the advisor reuses | Local matches |
| **Duo synergy** | Win rate and stats per agent pair with each friend | Local matches |
| **Patch impact** | Your results on an agent before and after a patch that changed them | `game_version` on matches + `last_changed_patch` in the knowledge base |
| **Post-match summary card** | Three plain facts after each match ("best opening-duel rate this week", "4 deaths from B main") | Local matches |
| **Localization** | UI in multiple languages; English and Hindi first | `lib/i18n` |
| **Accessibility** | Colour-blind-safe palettes for win/loss and heatmaps, scalable overlay text | UI only |

**Not possible: live in-round heatmaps or positions.** The local client does not expose player positions; only game memory does, which is forbidden and blocked by Riot. Riot's policy also rejects real-time "go here now" guidance. Everything above is post-match.

### 16.1 Export and import

Lets people back up their history, move to a new PC (or between the installed and portable builds), and use their data in their own tools. Everything is file based; nothing is uploaded. **Restoring a backup costs zero API requests**, which matters because rebuilding 200+ matches from HenrikDev would take hours of quota.

**Export** (Settings → Data & privacy → Export; also in the command palette)

| Format | What it is | Use |
|---|---|---|
| **Postplant backup** `.postplant` | A zip: a consistent SQLite snapshot made with `VACUUM INTO` (safe while the app runs), plus `manifest.json` (format version, schema version, app version, created_at, owner PUUID and Riot ID, row counts, SHA-256 of the database file) | Full backup and moving PCs. Default choice |
| **Open JSON** `.json` | Matches (with rounds, kills, economy), rank history, notes, agent pool and settings in a documented, versioned format (`docs/export-format.md`, JSON Schema in `schemas/export-v1.json`) | Your own scripts and tools. Can be imported back |

Options:
- What to include: matches, rank history, notes & tags, settings/hotkeys/agent pool.
- Date range: all time, this act, last 30 days.
- **Hide other players' names** (for sharing): replaces every Riot ID except yours with "Player 1, Player 2…" consistently within the file, and drops notes and tags.

The file size is estimated before saving. **The API key is never exported**; it lives in Windows Credential Manager, not the database. CSV stays excluded as asked earlier; JSON covers that need.

**Import** (Settings → Data & privacy → Import; "Restore from a backup" on the first setup screen; or drag a file onto the window)

1. **Inspect** (read-only, nothing changes yet):
   - Verify the zip, manifest and checksum; check the format and schema version; read the counts.
   - Show a preview: owner account, created date, app version, new matches vs already present, new notes, note conflicts.
2. **Choose a mode:**
   - **Merge** (default):
     - Insert matches, rounds and kills missing by `match_id`, and rank snapshots by timestamp.
     - Agent pool becomes the union of both.
     - Note conflicts on the same player follow the chosen rule: keep the newer (default), keep this PC's, or keep both joined.
     - Settings and hotkeys are imported only if ticked.
   - **Replace**: wipes local data and restores the file. Needs typing `replace`. A **safety backup is always made first**, so it can be undone from the backups list.
3. **Different account:** if the file's owner PUUID differs from this PC's, Merge is disabled (stats would mix two people). Replace warns that Postplant will then track the other account.
4. **Apply** in one SQLite transaction on an attached copy; on any error nothing changes. Then rebuild derived caches (comfort scores, pattern notes) and write `import_log`.

**Version rules:**
- Older schema: migrate the imported copy forward first.
- Newer schema than the app: refuse with "Update Postplant to import this file".
- JSON imports are validated against the schema before any row is written.

**Automatic backups:**
- Weekly by default, keeping the last 4.
- One before every Replace import, and one before every schema migration on update.
- Stored in `<data dir>/backups/`, so the portable build keeps them in `.\data\backups`.
- Listed in Settings with size and reason. Each has **Restore**, which runs the Replace flow, including its own safety backup.
- The toggle and the number kept are settings.

**Safety (import files are untrusted input):**
- Size cap of 500 MB.
- Zip entries limited to the expected names: no path traversal, no nested archives.
- The database is opened read-only and checked with `PRAGMA integrity_check` before attaching.
- Only known tables and columns are copied, with parameterised SQL.
- Settings are restricted to a whitelist of keys.
- Nothing in the file is ever executed.

**Implementation:**
- Rust module `src-tauri/src/transfer/`.
- Commands: `export_backup`, `export_json`, `inspect_import`, `apply_import`, `list_backups`, `restore_backup`.
- Unit tests use fixture files, including corrupt, oversized, newer-schema and other-account cases.

**Designs:**
- Settings → Data & privacy: the Backup & transfer card and the backup list.
- Setup welcome screen: "Restore from a backup".
- `design/components.html`: the Export, Import preview and Replace confirmation modals.

---

## 17. Open-source project setup

- **License:** MIT OR Apache-2.0 (dual, the common Rust choice). The data files under `data/` and `meta/` are factual compilations; keep source attribution in each manifest.
- **Docs:** README (what it does, screenshots, install, how to get a HenrikDev key, Riot notice), `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `docs/privacy.md` (every outbound request listed), `docs/export-format.md` (export/import format), `docs/compliance.md`, `docs/meta-sources.md`.
- **CI:** fmt, clippy, cargo test, Vitest, Playwright smoke test, meta-pipeline fixture tests, dependency audit (`cargo audit`, `npm audit`), on Windows and Linux runners.
- **Releases:** tag-driven GitHub Actions build; Tauri updater with signed update manifests; Windows code signing (look at free signing for open-source projects such as SignPath Foundation) so SmartScreen does not scare users.
- **Contribution paths that don't need Rust:** `data/agents.json` and `data/maps.json` edits, translations, adapter fixtures. Label these "good first issue".
- **CODEOWNERS:** maintainer review required for `data/`, `meta/`, `meta-pipeline/` and anything touching the network.
- **Issue templates:** bug (with app version, dataset version, region), data correction (with a source link), feature request (with a check that it isn't in the excluded list).

---

## 18. Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| HenrikDev disallows public clients or changes v4 | Medium | High | Ask before release; isolate the client behind a trait so another provider can replace it |
| Riot policy tightens on third-party overlays | Low–Medium | High | No memory, no local client, no scouting; re-read policy each release |
| Meta sources block automation | Medium | Medium | Tiered design; knowledge base works alone; manual dataset import |
| Dataset tampering via update channel | Low | Medium | Hash + signature checks; previous dataset kept |
| Riot penalizes users of the opt-in live module | Low–Medium | High (per user) | Off by default, clear notice, phase gate, read-only, remote kill switch, `live` feature can be compiled out |
| Users leak their own API key in bug reports | Medium | Low | Logs redact the key; issue template warns |
| Trademark complaint about name or art | Low | Medium | No Riot marks in name or logo; art loaded at runtime |

---

## 19. Name

**Recommended: Postplant.**
- It is a term every Valorant player knows (the phase after the spike is planted), and it fits a tool whose core is **after-the-fact** analysis: post-match scoreboard, sessions, breakdowns.
- It is an ordinary gaming word, not a Riot trademark, so it fits the rule in section 8.
- Short, easy to say, works as a repo name, binary name and domain stem (`postplant`).

**Owner approved Postplant on 2026-11-11.**

**Logo locked 2026-11-11: concept C, "Pulse"**: an open ring with a dot at the gap (a spike timer about to complete) around a solid diamond (the planted spike). Geometry, clear space, minimum sizes (16 px with a heavier stroke), app and tray icons and don'ts are in `design/index.html`. The site and design files use the same `#logo` symbol; `site/favicon.svg` is the Ink tile version.

### Availability check (2026-11-11)
Checked for an exact GitHub repository name, an npm package and a crates.io crate. "Free" means none exist; domains and trademarks were **not** checked.

| Name | GitHub (exact name) | npm | crates.io | Notes |
|---|---|---|---|---|
| **Postplant** | 1 repo, 0 stars | free | free | Chosen |
| **Afterspike** | none | free | free | "After the spike": fits post-match analysis |
| **Plantside** | none | free | free | Calm, readable; "the side of the plant" |
| **Sitehold** | none | free | free | Anchoring a site; steady, reliable tool feel |
| **Tradekill** | none | free | free | Punchy; teamwork connotation |
| **Ultpoint** | none | free | free | Short; game-flavoured |
| **Rotatecall** | none | free | free | Strategy and comp-calling feel |
| Sitecall | 3 repos, 0 stars | free | free | Usable, slightly crowded |
| Ecoround | 1 repo, 0 stars | free | free | Usable |
| Halftime | 48 repos | taken | taken | Avoid |
| Clutchlog | 1 repo, 17 stars | free | free | Avoid (active project) |

Before announcing: check domains (`.gg`, `.app`, `.dev`) and a basic trademark search (WIPO Global Brand Database, USPTO, Indian IP office).

---

## 20. Distribution, CI and auto-updates (Windows)

### 20.1 What users download (every release)
| File | What it is | Notes |
|---|---|---|
| `Postplant_<ver>_x64-setup.exe` | **Installer** (NSIS) | Per-user install, no admin needed. Start-menu entry, optional desktop shortcut, optional "start with Windows". Registers in *Settings → Apps* with a working **uninstaller**. WebView2: `embedBootstrapper` (installs it only if missing; Windows 11 already has it). |
| `Postplant_<ver>_x64-offline-setup.exe` | **Offline installer** | Same, but with the WebView2 runtime bundled (`offlineInstaller`), for machines without internet or with WebView2 removed. About 130 MB larger. |
| `Postplant_<ver>_x64-portable.zip` | **Portable** | Unzip and run. A `portable.flag` file next to the exe makes the app keep its database and settings in `.\data\` instead of `%APPDATA%`. Needs WebView2 (present on Windows 10 21H2+ and 11); if missing, the app shows a one-click link to Microsoft's installer. |
| `latest.json`, `*.sig` | Updater manifest and signatures | Read by the in-app updater. |
| `SHA256SUMS.txt` | Checksums | Linked from the download page. |

**Dependencies:** build with a statically linked C runtime (`-C target-feature=+crt-static` in `.cargo/config.toml`) so no Visual C++ Redistributable is needed. WebView2 is the only runtime dependency and is handled as above.

**Uninstaller:** removes the app, shortcuts, the autostart entry and the registry keys. A checkbox, unchecked by default, "Also delete my match history, settings and saved API key" removes `%APPDATA%\Postplant` and the Credential Manager entry. The portable build is removed by deleting its folder; Settings has a "Remove saved API key" button for that case.

### 20.2 GitHub Actions
| Workflow | Trigger | Does |
|---|---|---|
| `ci.yml` | Every PR and push | fmt, clippy, tests, Vitest, meta-pipeline fixture tests. Path filters: changes only under `site/`, `docs/` or `*.md` skip the Rust and app jobs. |
| `release.yml` | Push to `main` that touches app paths (`src/`, `src-tauri/`, `data/`, lockfiles) | Builds and publishes a rolling **nightly** pre-release (tag `nightly`), replacing its assets. |
| `release.yml` | Tag `v*.*.*` | Builds a **stable** release with `tauri-action`: draft → upload assets, `latest.json`, checksums → publish. Release notes generated from conventional commits. |
| `site.yml` (optional) | Changes under `site/` | Not needed if Cloudflare Pages is connected to the repo; it deploys on its own. |

**No unnecessary rebuilds:**
- Path filters (above) stop docs or site commits from building the app.
- `concurrency` with `cancel-in-progress` so only the newest push to a branch keeps building.
- `Swatinem/rust-cache` for Cargo registry and `target/`; `actions/setup-node` with the npm/pnpm cache.
- **Build fingerprint:** the release job hashes app inputs (`hashFiles('src/**', 'src-tauri/**', 'data/**', '**/Cargo.lock', '**/pnpm-lock.yaml')`). Built bundles are saved with `actions/cache` under that key; if a later push has the same fingerprint (for example a README-only change that slipped past the filter, or re-running a tag on the same commit), the job restores the bundles and only re-publishes them.

**Secrets:** `TAURI_SIGNING_PRIVATE_KEY` and its password (updater signatures, required). Authenticode signing later through a free open-source signing service such as SignPath Foundation, so SmartScreen warnings fade as reputation builds. Until then, the download page explains the SmartScreen prompt.

### 20.3 In-app auto-updater
- `tauri-plugin-updater`, endpoint `https://github.com/<owner>/postplant/releases/latest/download/latest.json` (stable) or the `nightly` release's manifest (nightly channel).
- Checks at start and every 6 hours. **Never interrupts a match:** if Valorant is running, the update waits until the game closes.
- Shows version and changelog; buttons "Update now" and "On next restart". Every download is verified against the updater public key baked into the app; an invalid signature is rejected.
- Settings: channel (Stable / Nightly), "check automatically" on/off, "check now".
- **Portable build:** same check and signature verification; it downloads the new zip, renames the running exe to `.old`, extracts the new files, restarts, and deletes `.old` on the next start. The `data\` folder is never touched.

---

## 21. API key inside the app

- **Where:** onboarding step 2, and **Settings → API key** at any time.
- **Field:** paste box with show/hide, **Test key** button (one cheap request; shows OK, the key tier and the per-minute limit read from response headers), **Remove key** button.
- **Storage:** Windows Credential Manager via the `keyring` crate (portable build too). Never written to settings files, logs or crash reports; the logger redacts anything shaped like a key.
- **"How do I get a key?" panel**, shown next to the field:
  1. Open the HenrikDev API docs (`docs.henrikdev.xyz`) and join their Discord from there.
  2. Use their key bot to create a **Basic** key: free and instant, 30 requests per minute, which is enough for Postplant.
  3. Optional: apply for an **Advanced** key (90 per minute) if you share a key across heavy use; replies take 1–2 weeks.
  4. Paste the key here and press **Test key**.
  - Plus a short "Why not a Riot key?" note: Riot does not issue personal VALORANT keys.
  - Screenshots for each step, stored in the app (no web calls to show the guide).
- **Errors:** an invalid or revoked key shows a banner linking straight to this panel; rate-limit warnings link to the quota page.

---

## 22. Project tracking

- **`PROGRESS.md`**: one checklist per milestone mirroring section 7, a "Now / Next / Blocked" header, and a dated log. Updated in the same commit as the work it describes.
- **`README.md`**: what Postplant is, screenshots, download links, how to get an API key, build from source, privacy summary, Riot notice, license.
- **Commits**: conventional commits (`feat:`, `fix:`, `docs:`, `ci:`, `chore:`), small and focused; release notes are generated from them. `main` is always releasable; larger work goes on branches with PRs once there are contributors.

---

## 23. Website (static, Cloudflare Pages)

- `site/index.html` (landing) and `site/download.html` (download), shared `site/assets/`. **No build step**: plain HTML, CSS and a little JavaScript, so Cloudflare Pages serves the folder as is (build command: none, output directory: `site`).
- Typography: Satoshi for body text (Fontshare), Big Shoulders Display and JetBrains Mono (Google Fonts). The CSP in `_headers` allows exactly these two font hosts. Motion is done with CSS and small vanilla scripts (scroll-linked reveals via IntersectionObserver, a canvas hero, magnetic buttons). Respects `prefers-reduced-motion`; no motion library needed.
- The download page reads the latest release from the GitHub API (unauthenticated, cached in the browser for an hour) to show version, date, file sizes and checksums, and falls back to a link to the Releases page if the API is unavailable.
- `site/_headers` sets security headers and long caching for assets.
- Brand locked: logo C and the name Postplant everywhere; the temporary brand switcher has been removed.
- Riot "not endorsed" notice in the footer of both pages. No Riot logos or game art on the site.

---

## 24. Design system (`design/`)

The design is finished up front so implementation is translation, not invention. Open `design/index.html` in a browser; the toolbar switches theme, font and colour-blind mode on every page.

| File | What it holds |
|---|---|
| `design/tokens.css` | **Source of truth.** Type scale, spacing (4 px grid), radii, motion, layout sizes, z-layers, role and rank colours, and six themes. The app imports it unchanged. |
| `design/components.css` | The component contract: buttons, inputs, custom dropdowns and menus, switches, checkboxes, radios, segmented controls, tabs, sliders, chips, badges, ranks, agent tiles (available / selected / picked / locked), map tiles, cards, stats, tables, match rows, progress, skeletons, banners, toasts, tooltips, popovers, modals, empty states, app shell, charts and overlay panels. Svelte components reproduce these class names and states. |
| `design/index.html` | Foundations: logo and icons, themes, typography, spacing, elevation, motion, icon set, data colours, writing rules. |
| `design/components.html` | Every component in every state, including all modals (live-match warning, delete data, update ready, keybind capture, backfill cost, unsaved changes, player note, agents you own, export, import preview, replace confirmation), command palette and player card. |
| `design/screens.html` | 27 screens at 1280 × 800: setup (5 steps, including restore from backup), home, search, profile, match history, match detail (scoreboard, round replay), heatmaps, analytics, rank, Comp Advisor (solo, stack planner, learn a map), people, settings (appearance, API key, overlay & hotkeys, Comp Advisor, live match, updates, data & privacy, about) and offline / rate-limit / empty / loading states. |
| `design/overlay.html` | The overlay over an abstract game scene with the HUD zones it must avoid: last match, session with tilt warning, rank, live panel, post-match scoreboard, compact Comp Advisor, edit mode, scales, anchors, tray menu and tray icon states. |

### 24.1 Themes
Ink (default dark, lime), Midnight (navy, sky blue), Nebula (violet, lavender), Paper (default light, lime fill with olive text), Frost (cool grey-blue, cobalt), Sandstone (sand, terracotta). "Follow Windows" switches between Ink and Paper.

Rules every theme meets (checked on every text element of the design pages):
- `--text`, `--text-2`, `--text-3`, `--accent-text` and all status colours reach **4.5:1** on `--bg`, `--bg-2`, `--surface`, `--surface-2` and `--surface-3`.
- Page, sidebar and cards are clearly separate steps. Light themes also lift cards with `--card-shadow`.
- Text on solid win/loss fills uses `--win-ink` / `--loss-ink`.
- Colour-blind mode (`data-cvd="on"`) swaps win/loss to blue/orange in every theme.
- Overlay glass is 88% opaque, so even faint text passes 4.5:1 over a pure white or black game scene. The user's glass slider (75–100%) changes only the background; below 88%, secondary text switches to `--text-2`.

### 24.2 Fonts
UI default **Satoshi**. Options: General Sans, Geist, Inter, Atkinson Hyperlegible (for low vision). Satoshi and General Sans are under the ITF Free Font License (Fontshare); the others are SIL OFL. The app bundles the font files; the design pages load them from CDNs.

### 24.3 Implementation notes
- Copy `tokens.css` into the app as is; theme = `data-theme` on `<html>`, font = `data-font`, colour-blind = `data-cvd`.
- Agent, map and rank art is shown as monograms in the designs; the app uses valorant-api.com images in the same boxes.
- Never hard-code a colour in a component; add a token instead and check it against the 4.5:1 rule above.

