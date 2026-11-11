# Progress

Tracks Postplant against the milestones in [plan.md](plan.md) (section 7). Updated in the same commit as the work it describes.

## Now / Next / Blocked

- **Now:** Design system done (`design/`); ready to start M0.
- **Next:** M0: Tauri v2 + Svelte scaffold, CI and release workflow skeleton, check the HenrikDev match-detail fields.
- **Blocked / waiting on owner:**
  - Choose a license (recommended: MIT OR Apache-2.0).
  - Ask HenrikDev whether a public open-source client with user-supplied keys is allowed.
  - Create the GitHub repository and set the owner name in `site/assets/config.js`.

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
- [ ] Deployed to Cloudflare Pages

### M0: Setup
- [ ] Tauri v2 + Svelte + TypeScript project
- [ ] CI (`ci.yml`) and release workflow skeleton with caching (`release.yml`)
- [ ] HenrikDev key; field check for rounds, kill events, locations, ability casts (`docs/api-notes.md`)

### M1: Data layer
- [ ] Budget governor
- [ ] Henrik client (retries, 429 handling, request merging)
- [ ] SQLite schema and migrations, cache layer
- [ ] Match sync job

### M2: Search app
- [ ] Onboarding with in-app API key entry and "how to get a key" guide
- [ ] Search, profile, match history, match detail, look up player
- [ ] Trend charts

### M3: Overlay
- [ ] Window modes, hotkeys, persistence, panels, new-match detection

### M2b: Analytics
- [ ] Section 12 features (trimmed to what M0 data supports)

### M4: Comp Advisor v1
- [ ] Knowledge base (`data/agents.json`, `data/maps.json`, `data/map-pool.json`)
- [ ] Scoring engine and tests
- [ ] Solo queue, stack planner, learn-for-this-map; compact overlay panel
- [ ] Agent ownership: only suggest agents the user owns (plan 14.1a)

### M5: Meta pipeline
- [ ] Source terms reviewed (`docs/meta-sources.md`)
- [ ] Adapters, validation, PR-opening workflow
- [ ] Signed dataset release and in-app loader

### M6: Extras, polish, v1.0
- [ ] Heatmaps, opening duels, ability usage, economy efficiency, agent pool, duo synergy, patch impact, post-match summary
- [ ] Round replay, death review, trade stats, pattern notes, where rounds end
- [ ] Export / import and automatic backups (plan 16.1)
- [ ] English + Hindi UI, colour-blind-safe palettes
- [ ] Installer, offline installer, portable zip, uninstaller
- [ ] Auto-updater (stable + nightly channels)
- [ ] Code signing; v1.0 release

### M7: Live match (opt-in)
- [ ] Toggle (off by default) with mandatory warning
- [ ] In-game phase gate, hidden-name respect, live quota budget
- [ ] Live overlay panel

## Log

- **2026-11-11**: Plan rewritten for open source; Comp Advisor and meta pipeline redesigned; live match moved to an opt-in toggle; distribution, CI, updater and in-app API key sections added. Landing and download pages built.
- **2026-11-11**: Brand locked (Postplant, logo C). Full design system in `design/`: six themes retuned so every text colour passes 4.5:1 and light-theme cards separate from the page, components with all modals, 27 screens, overlay. Export/import added to the plan (16.1) and designs. Site switcher removed.
