# Postplant

**A lightweight, open-source Valorant companion for Windows.** See where your games are won and lost, look up any player, keep a small overlay on screen, and plan agent picks per map. Your data stays on your PC.

[![CI](https://github.com/jeelnadaa/postplant/actions/workflows/ci.yml/badge.svg)](https://github.com/jeelnadaa/postplant/actions/workflows/ci.yml)
[![Release](https://github.com/jeelnadaa/postplant/actions/workflows/release.yml/badge.svg)](https://github.com/jeelnadaa/postplant/releases/latest)

> **Status: v0.1, first pre-release.** Everything below is built. The API field names follow HenrikDev's documentation and still need a check against live data (see [docs/api-notes.md](docs/api-notes.md)). Track progress in [PROGRESS.md](PROGRESS.md); the full design is in [plan.md](plan.md).

**Watch the 33-second launch film:** [postplant.solarquack.in](https://postplant.solarquack.in/#film) ([MP4](site/assets/media/postplant-launch.mp4)).

## Features

- **Home**: rank and RR, this session (with a gentle break nudge after a losing streak), recent form, plain-language insights.
- **Matches and match detail**: full scoreboard with every player's name, rank and stats after the match, party tags, how often you've met someone, rounds with side and loadout, and every death with who killed you, whether you were traded and how far your nearest teammate was.
- **Analytics**: by agent, by map and side, economy (eco / force / full buy), kills by weapon, untraded deaths. **Lifetime**: your whole record, not just the matches on this PC. Postplant downloads your HenrikDev match record (totals per match, up to 2,000 matches) in the background and merges it with the matches held in full; filter by mode, map, agent, act and period. Per agent, map, agent × map, act, patch, weapon and mode: win %, K/D, KDA, ACS, ADR, HS %, KAST, first duels, plants, defuses and defuse rate, post-plant and retake kills, clutches, multi-kills, attack/defense rounds, trades, damage per 1,000 credits. **When you play best**: by match number in a session, time of day, party size and after losing streaks.
- **Rank**: hero cards for current rank, peak rank and RR; rating over time, RR per match, average gain, roughly how many wins to the next rank, past acts.
- **Player lookup** by Riot ID (about 3 requests, cached 10 minutes): rank, peak, act record, a quick per-agent and per-map summary of the last 25 competitive games, and **detailed career stats** (the same full set as Lifetime) for 5–25 of their matches at 1 request per match, kept on this PC so asking again is free.
- **Match detail**: rank at match time plus each player's **peak and current rank**, a **2D replay** that steps through every kill with where all ten players stood and an arrow from killer to victim (hover any kill to preview it; filter to your own kills and deaths, or to you vs one player with a head-to-head count), and a **heatmap of that match alone**. Heatmaps and replay frames save as PNG or copy to the clipboard.
- **Heatmaps**: kills, deaths, opening duels, where rounds end, plants and defuses on each map, for you, your team or the enemy, filtered by side, agent, round type (pistol, eco, force, full buy) and won or lost rounds, with the hot spots named by callout.
- **Pattern notes and trades**: plain-language notes such as where you tend to die first on a map, each linked to the rounds it came from; how often your deaths get traded, how fast, and which teammates trade you.
- **Duo synergy**: your win rate with each friend, and which of your agent pairs work best together.
- **Post-match facts**: three short facts on the match page and the post-match overlay board, plus ability casts against your average on that agent.
- **People**: players you meet often, with your own notes and tags.
- **Comp Advisor**: pick a map, enter teammates' agents yourself, get suggestions with reasons and full comps, only from agents you own. With pro data published, it also weighs **pro pick rates on that map** and lists pro comps that fit your picks (source: Liquipedia, CC-BY-SA 3.0). Advisor data updates by itself between releases, signed like app updates. **Stack planner**: you and up to four friends, each assigned an agent from their own pool, with the top three line-ups and why. Also opens **in the overlay** (hotkey, or automatically in agent select with Live match on).
- **Overlay**: last match, session and rank panels; the scoreboard pops up by itself after each match. Click-through, movable, game-safe.
- **Live match** (opt-in, off by default, behind a warning): reads the Riot Client on your PC to know when you're in agent select or a match. Agent select only opens your Comp Advisor; nothing about other players is read there. In the match: a short round-1 panel, hold **Tab** for rank, peak and form beside the scoreboard, and a full table on a hotkey. Players who hide their name stay hidden.
- **Recordings**: point it at your ShadowPlay / OBS folder and every round and death gets a **Watch** button that opens the video at that moment. A **Review list** keeps moments to rewatch.
- **Export / import**: full `.postplant` backups, JSON export (optionally anonymised), merge or replace with a preview, weekly automatic backups.
- **Updates**: the installed app checks GitHub for a new version (never while VALORANT runs), downloads it, checks its signature and restarts. The portable build only tells you an update is out.
- **Translatable UI**: every string goes through `src/lib/i18n.svelte.ts`. English only for now; a Hindi translation is in the repo, switched off.
- **API field check** (Settings → API): checks your newest match for every field Postplant reads, so you can tell at a glance if HenrikDev changed something.
- Six themes, five fonts, colour-blind mode, text size, `Ctrl+K` command palette, tray icon, start with Windows, global hotkeys that only work while the game runs.

## Download

Get the latest build from [Releases](https://github.com/jeelnadaa/postplant/releases/latest):

| File | What it is |
|---|---|
| `Postplant_<ver>_x64-setup.exe` | Installer. Installs for your user only, no admin needed, includes an uninstaller. |
| `Postplant_<ver>_x64-offline-setup.exe` | Same installer with WebView2 bundled, for PCs without internet or without WebView2. |
| `Postplant_<ver>_x64-portable.zip` | Portable. Unzip and run; data is kept in a `data` folder next to the exe. |
| `SHA256SUMS.txt` | Checksums to verify your download. |

Builds aren't code-signed yet, so Windows SmartScreen may warn: choose **More info → Run anyway**.

## You need a HenrikDev API key

Postplant uses the community [HenrikDev API](https://docs.henrikdev.xyz), not Riot's API (Riot doesn't issue personal VALORANT keys).

1. Open [docs.henrikdev.xyz](https://docs.henrikdev.xyz) and join the HenrikDev Discord from there.
2. Create a free **Basic** key with their key bot (instant, 30 requests per minute; plenty for Postplant).
3. Paste it into Postplant during setup, or later in **Settings → API key & usage**.

The key is stored in Windows Credential Manager. It is never written to Postplant's files, backups or logs, and is only sent to `api.henrikdev.xyz`. A built-in budget keeps background sync to half of your per-minute limit.

## Privacy

No telemetry, no accounts, no servers of ours. Every outbound request is listed in [docs/privacy.md](docs/privacy.md). Postplant never reads game memory, never scouts during agent select, and never tries to reveal hidden names. The opt-in Live match module talks to the Riot Client on 127.0.0.1 and to Riot's own servers; Riot tokens stay in memory and are never sent anywhere else.

## Hotkeys

Default hotkeys (click one in **Settings → Overlay & hotkeys** and press the new combination; by default they only work while VALORANT runs):

| Action | Default |
|---|---|
| Show / hide overlay | `Ctrl+Shift+H` |
| Move overlay | `Ctrl+Shift+M` |
| Reset overlay position | `Ctrl+Shift+Home` |
| Next overlay panel | `Ctrl+Shift+PageDown` |
| Check for new matches | `Ctrl+Shift+F5` |
| Open Postplant | `Ctrl+Shift+O` |
| Comp Advisor in the overlay | `Ctrl+Shift+A` |
| Live match table | `Ctrl+Shift+L` |

## Build from source

Needs Node 20+, Rust (stable, MSVC) and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for Windows.

```bash
npm install
npm run tauri dev          # run the app with hot reload
npm run check && npm test  # type check and frontend tests
cd src-tauri && cargo test # backend tests
npm run tauri build        # build the installer
```

Stack: Tauri v2 (Rust), Svelte 5 + TypeScript, SQLite (rusqlite, bundled). See [CONTRIBUTING.md](CONTRIBUTING.md).

## Releases and versioning

Every push to `main` that changes the app builds and publishes a GitHub Release automatically ([release.yml](.github/workflows/release.yml)). The version comes from [Conventional Commits](https://www.conventionalcommits.org) since the last tag: `feat:` bumps the minor version, anything else the patch, and `!` / `BREAKING CHANGE` the major (minor while below 1.0). Docs, site and design commits never trigger a release, and `[skip release]` in a commit message leaves it out. You can also run the workflow by hand and choose the bump.

## Meta data pipeline

`meta-pipeline/` keeps `data/` current. Every Wednesday `.github/workflows/meta.yml` syncs map minimaps, callouts and the game version from valorant-api.com, pulls pro pick rates per map from the LiquipediaDB API (when the `LIQUIPEDIA_API_KEY` secret is set), validates everything and opens a pull request for review. Merging it runs `dataset.yml`, which signs the data and publishes it to the `dataset` release; apps pick it up within a day. Each app release also bundles it. Which sources are allowed and why: [docs/meta-sources.md](docs/meta-sources.md). Run it yourself with `npm run meta:sync`, `npm run meta:validate` and `npm run meta:test`.

## Website

The landing and download pages are static files in [`site/`](site/), served by Cloudflare Workers static assets via [`wrangler.jsonc`](wrangler.jsonc):

```bash
npm run site:dev      # preview at http://localhost:8787
npx wrangler login    # once
npm run site:deploy   # publish
```

Or connect the repository in the Cloudflare dashboard (Workers Builds), or add `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` as repository secrets so [site.yml](.github/workflows/site.yml) deploys on every change under `site/`.

## Design

The design system (tokens, components, every screen and the overlay) is in [`design/`](design/). Open `design/index.html` in a browser.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. Fonts and icons keep their own licences (SIL OFL, ITF Free Font License, ISC).

## Legal

Postplant isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games and all associated properties are trademarks or registered trademarks of Riot Games, Inc.
