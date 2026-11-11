# Postplant

**A lightweight, open-source Valorant companion for Windows.** Look up any player, see where your games are won and lost, keep a movable overlay on screen, and get per-map agent and team-comp suggestions.

> **Status: planning and website.** The app is not built yet. See [PROGRESS.md](PROGRESS.md) for where things stand and [plan.md](plan.md) for the full design.

## Planned features

- **Player search** by Riot ID: rank, peak rank, recent matches, agents, maps, headshot %, ACS, K/D.
- **Overlay** that you can move, resize and hide with hotkeys. Shows your last match, session and rank, and the full scoreboard when a match ends.
- **Analytics** from your own match history: sessions with a tilt warning, breakdowns by agent, map, side and weapon, first bloods, clutches, round timelines, teammate and duo stats, notes on players.
- **Comp Advisor**: pick the map and your teammates' agents, and get suggested agents and full team comps, with the reason, sample size and data date for each suggestion.
- **Heatmaps** of where you get kills and die, ability usage, economy efficiency, and patch impact on your agents.
- **Live match (experimental, off by default)**: opponents' rank and peak rank once the match has started. Never during agent select. See the warning below.
- **Quota-safe**: a built-in request budget keeps you well inside your API limit.
- **Private by design**: no telemetry. Your data stays on your PC.

## Download

Releases will be published on the [Releases page](../../releases): installer, offline installer and portable zip, all with auto-update.

## You need a HenrikDev API key

Postplant uses the community [HenrikDev API](https://docs.henrikdev.xyz), not Riot's API (Riot does not issue personal VALORANT keys).

1. Open [docs.henrikdev.xyz](https://docs.henrikdev.xyz) and join the HenrikDev Discord from there.
2. Use their key bot to create a free **Basic** key (instant, 30 requests per minute; enough for Postplant).
3. Paste it into Postplant (onboarding, or **Settings → API key**) and press **Test key**.

The key is stored in Windows Credential Manager and never leaves your PC except in requests to HenrikDev.

## About the live match feature

Live match reads the Riot Client running on your PC. **Riot does not support this**, it may stop working after any game update, and Riot may treat tools that show other players' information as a rules violation, which could put your account at risk. It is **off by default**, shows a warning before you can turn it on, never reads game memory, and never runs during agent select.

## Build from source

Not available yet. The stack will be Tauri v2 (Rust), Svelte 5 + TypeScript and SQLite.

## Website

The landing and download pages are static files in [`site/`](site/). To deploy on Cloudflare Pages: no build command, output directory `site`.

## License

To be decided (likely MIT OR Apache-2.0).

## Legal

Postplant is not endorsed by Riot Games and does not reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games and all associated properties are trademarks or registered trademarks of Riot Games, Inc.
