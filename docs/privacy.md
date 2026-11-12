# Privacy: every request Postplant makes

Postplant has no telemetry, no analytics, no accounts and no servers of its own. Everything it stores lives in one SQLite database on your PC (`%APPDATA%\app.postplant.desktop`, or `data\` next to the exe for the portable build).

## Outbound requests

| Host | When | What is sent |
|---|---|---|
| `api.henrikdev.xyz` | Setup; match sync (every 75 s while VALORANT runs, every 30 s for 10 min after it closes, then backing off to 12 min); your lifetime record (up to 80 requests once, then 1 every 6 hours, never while VALORANT runs); player lookups you start | Your API key (header), Riot IDs or PUUIDs, region, match IDs. Nothing else. |
| `media.valorant-api.com` | When a screen shows an agent portrait | A plain image request. Cached by the webview. |
| `github.com`, `objects.githubusercontent.com` | Update check: 20 s after start and every 6 hours (if enabled), and when you press **Check now**; never while VALORANT runs | Downloads `latest.json` from the latest release; if there is a newer version and you press **Update and restart**, downloads the signed installer. Nothing about you is sent. |
| `github.com` (advisor data) | Once a day if **Update advisor data automatically** is on (default), and when you press **Check advisor data**; never while VALORANT runs | Downloads `dataset.json.sig` and, if needed, `dataset.json` from the `dataset` release. Nothing about you is sent. |
| `api.github.com` | Fallback update check (portable build, or a release without an update manifest) | One unauthenticated request for the latest release. |
| `127.0.0.1` (Riot Client) | Only with **Live match** turned on: every 2 s while the Riot Client runs | Reads the lockfile password from `%LOCALAPPDATA%\Riot Games\Riot Client\Config\lockfile` and asks the local client for your session tokens, region and your own presence (game phase and map). |
| `glz-*.a.pvp.net`, `pd.*.a.pvp.net` (Riot) | Only with **Live match** on, once per match once it is **in game**; once per session for your owned agents | Your Riot tokens (held in memory only), the match ID and the PUUIDs in it, to get agents, teams, levels and the names of players who don't hide them. Nothing during agent select. |

`valorant-api.com` and the LiquipediaDB API are queried by the weekly meta pipeline in GitHub Actions, never by the app on your PC.

Opening links (HenrikDev docs, GitHub, release pages) happens in your own browser.

## What Postplant never does

- Read game memory or inject into the game.
- Read anything about other players during agent select (no scouting). With Live match on, agent select only opens your own Comp Advisor with the map filled in.
- Store or send Riot session tokens anywhere: they stay in memory and go only to Riot's own hosts.
- Look up players whose names are hidden in a live match, or try to reveal hidden names.
- Write your API key anywhere except Windows Credential Manager. Logs and errors redact anything shaped like a key; backups and exports never contain it.
- Upload or modify your recordings. It reads only file names and times to match videos to matches.

## Deleting your data

**Settings → Data & privacy → Delete all data** removes matches, notes, settings and cache, and optionally the API key. Uninstalling also offers to remove everything.
