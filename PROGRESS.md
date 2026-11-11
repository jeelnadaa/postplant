# Progress

Tracks Postplant against the milestones in [plan.md](plan.md) (section 7). Updated in the same commit as the work it describes.

## Now / Next / Blocked

- **Now:** v0.1 built: full app (all screens, overlay, tray, hotkeys), CI, automatic versioned releases, Cloudflare (wrangler) site config.
- **Next:** check HenrikDev field names against live data with a real key (`docs/api-notes.md`); stack planner and compact advisor overlay panel; performance pass with the game running.
- **Blocked / waiting on owner:**
  - Ask HenrikDev whether a public open-source client with user-supplied keys is allowed.
  - Updater signing key: run `npx tauri signer generate -w postplant.key`, add `TAURI_SIGNING_PRIVATE_KEY` (and `_PASSWORD`) as repository secrets, and put the public key in `tauri.conf.json` to switch on signed auto-updates. Until then the app checks GitHub Releases.
  - Deploy the site: `npx wrangler login` then `npm run site:deploy`, or connect Workers Builds / add `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ACCOUNT_ID` secrets.
  - Code signing (SignPath Foundation or similar).

## Milestones

### Planning
- [x] Plan written and reviewed (`plan.md`)
- [x] Current map pool, roster and meta sources checked (2026-11-11)
- [x] Name chosen: **Postplant**; availability checked on GitHub, npm and crates.io
- [ ] Domain and trademark check

### Design
- [x] Brand locked: name Postplant, logo C "Pulse"
- [x] Tokens and six themes, all text 4.5:1 in every theme (`design/tokens.css`)
- [x] Components, every modal and dropdown (`design/components.html`)
- [x] Every app screen (`design/screens.html`) and the overlay (`design/overlay.html`)

### Site
- [x] Landing page (`site/index.html`)
- [x] Download page (`site/download.html`)
- [x] Brand confirmed, switcher removed; Satoshi body text; favicon
- [x] `wrangler.jsonc` for Cloudflare Workers static assets, 404 page, `site.yml` deploy workflow
- [ ] First deploy (owner)

### M0: Setup
- [x] Tauri v2 + Svelte + TypeScript project
- [x] CI (`ci.yml`) and release workflow with caching and automatic semver (`release.yml`, `scripts/next-version.mjs`)
- [x] In-app key entry, Windows Credential Manager
- [ ] Field check with a real key: rounds, kill events, locations, ability casts (`docs/api-notes.md` lists what is read)

### M1: Data layer
- [x] Budget governor
- [x] Henrik client (retries, 429 handling)
- [x] SQLite schema and migrations, cache layer
- [x] Match sync job (polls stored-matches only while VALORANT runs, backs off 3 → 6 → 12 min)

### M2: Search app
- [x] Onboarding with in-app API key entry and "how to get a key" guide, restore from backup
- [x] Search, profile, match history, match detail, look up player
- [x] Trend charts

### M3: Overlay
- [x] Window modes, hotkeys, persistence, panels, new-match detection

### M2b: Analytics
- [x] Section 12 features: by agent, map/side, economy, weapons, untraded deaths, habits

### M4: Comp Advisor v1
- [x] Knowledge base (`data/agents.json`, `data/maps.json`, `data/map-pool.json`)
- [x] Scoring engine and tests
- [x] Solo queue, learn-for-this-map
- [ ] Stack planner; compact overlay panel
- [x] Agent ownership: only suggest agents the user owns (plan 14.1a)

### M5: Meta pipeline
- [ ] Source terms reviewed (`docs/meta-sources.md`)
- [ ] Adapters, validation, PR-opening workflow
- [ ] Signed dataset release and in-app loader

### M6: Extras, polish, v1.0
- [ ] Heatmaps, opening duels, ability usage, economy efficiency, agent pool, duo synergy, patch impact, post-match summary
- [ ] Round replay, death review, trade stats, pattern notes, where rounds end
- [x] Export / import and automatic backups (plan 16.1)
- [ ] English + Hindi UI, colour-blind-safe palettes
- [x] Installer, portable zip, uninstaller (NSIS)
- [ ] Offline installer
- [ ] Auto-updater (stable + nightly channels)
- [ ] Code signing; v1.0 release

- [x] Section 25.1: Watch in recording at round/death, When you play best (Habits), Review list

### M7: Live match (opt-in)
- [ ] Toggle (off by default) with mandatory warning
- [ ] In-game phase gate, hidden-name respect, live quota budget
- [ ] Live overlay: round-1 buy-phase panel, hold-Tab rail, full table (Ctrl+Shift+L); rank, peak, last act, level, recent form, notes; Private for private profiles; hidden-name players shown by agent name
- [ ] Section 25.2: experience on current agent (1st-time tag), rematch notice, your context line

## Log

- **2026-11-11**: Plan rewritten for open source; Comp Advisor and meta pipeline redesigned; live match moved to an opt-in toggle; distribution, CI, updater and in-app API key sections added. Landing and download pages built.
- **2026-11-11**: Brand locked (Postplant, logo C). Full design system in `design/`: six themes retuned so every text colour passes 4.5:1 and light-theme cards separate from the page, components with all modals, 27 screens, overlay. Export/import added to the plan (16.1) and designs. Site switcher removed.
- **2026-11-11**: Live match detailed (plan section 11) and designed (`design/overlay.html`): buy-phase panel, hold-Tab rail beside Valorant's scoreboard, full table, and every per-player state including Private.
- **2026-11-11**: Agent portraits (valorant-api.com, monogram fallback) replace two-letter tiles; hidden-name players show their agent name live; after the match every player shows in full except private-profile data. Game-safe hover/press motion (transform/opacity only; overlay has no blur or hover). New features in plan section 25 and designed: recording sync, Habits, Review list, 1st-time-on-agent, rematch notice, context line.
- **2026-11-11**: v0.1 implementation. Rust backend (budget governor, HenrikDev client, sync, SQLite, queries, Comp Advisor, backup/export, recordings, overlay, tray, hotkeys) with 25 tests; every Svelte screen and the overlay. Plan fixes: 45-minute sessions everywhere, `rr_changes` table, hotkeys that don't clash with browsers and only work in game, stored-matches polling. Small features: `Ctrl+K` palette, copy Riot ID, region from lookup, tray, start with Windows, sync pill. CI (`ci.yml`), automatic semver releases (`release.yml`), Cloudflare wrangler site config and `site.yml`. Licence: MIT OR Apache-2.0.
