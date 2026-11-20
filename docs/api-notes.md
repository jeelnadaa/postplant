# HenrikDev API notes

Postplant talks to the HenrikDev API (`https://api.henrikdev.xyz`) with the user's own key in the `Authorization` header. All calls go through the budget governor in `src-tauri/src/api.rs`.

> **To verify with a real key (M0):** the parser was written against HenrikDev's documented response shapes. Run a sync and a lookup with a real account, and correct any field below that comes back empty. Parsing is defensive: a missing field becomes 0 or empty instead of an error, so wrong names show up as blank columns, not crashes.

## Endpoints used

| Path | Used for | Cost |
|---|---|---|
| `/valorant/v1/status/eu` | Testing a key (reads rate-limit headers) | 1 |
| `/valorant/v2/account/{name}/{tag}` | Setup, player lookup | 1 |
| `/valorant/v3/by-puuid/mmr/{region}/pc/{puuid}` | Current rank, peak, act history | 1 |
| `/valorant/v1/by-puuid/mmr-history/{region}/{puuid}` | RR change per match | 1 |
| `/valorant/v4/by-puuid/matches/{region}/pc/{puuid}?size=N` (N ≤ 10) | Polling for my new matches: fresh from Riot, full match objects in one request | 1 |
| `/valorant/v1/by-puuid/stored-matches/{region}/{puuid}?size=N[&mode=]` | Fallback when the fresh list fails, backfills over 10, player lookup (lags behind new matches) | 1 |
| `/valorant/v4/match/{region}/{match_id}` | Full match detail (stored compressed) | 1 per new match |

Rate-limit headers read: `x-ratelimit-limit`, `x-ratelimit-remaining`, `retry-after` (on 429).

## Fields read

**Account (v2):** `puuid`, `name`, `tag`, `region`, `account_level`, `card`.

**MMR (v3):** `current.tier.id`, `current.rr`, `current.elo`, `current.last_change`, `peak.tier.id`, `peak.season.short`, `seasonal[].season.{id,short}`, `seasonal[].end_tier.id`, `seasonal[].wins`, `seasonal[].games`.

**MMR history (v1):** `match_id`, `mmr_change_to_last_game`, `currenttier`, `ranking_in_tier`.

**Stored matches (v1):** `meta.id`, `meta.map.name`, `meta.started_at`, `stats.team`, `stats.character.{id,name}`, `stats.tier`, `stats.kills|deaths|assists|score`, `stats.shots.{head,body,leg}`, `teams.red|blue`.

**Match (v4):**
- `metadata.match_id`, `metadata.map.{id,name}`, `metadata.queue.{id,name}`, `metadata.started_at`, `metadata.game_length_in_ms`, `metadata.game_version`, `metadata.season.{id,short}`, `metadata.region`
- `players[]`: `puuid`, `name`, `tag`, `team_id`, `party_id`, `agent.{id,name}`, `tier.id`, `account_level`, `customization.card`, `stats.{score,kills,deaths,assists,headshots,bodyshots,legshots}`, `stats.damage.{dealt,received}`
- `teams[]`: `team_id`, `rounds.won`, `won`
- `rounds[]`: `result`, `winning_team`, `plant.site`, `plant.player.puuid`, `defuse.player.puuid`, `stats[]` with `player.puuid`, `economy.{loadout_value,spent}`, `damage`, `kills`
- `kills[]`: `round`, `time_in_round_in_ms`, `time_in_match_in_ms`, `killer.puuid`, `victim.puuid`, `weapon.{id,name}`, `location.{x,y}`, `player_locations[]` (for "nearest teammate")

## Riot match history (live mode only, `src-tauri/src/riot_history.rs`)

HenrikDev's stored matches only hold matches HenrikDev has seen, often just the last few dozen. With live mode on and VALORANT running, Postplant reads available records for your profile or a searched profile on the connected shard from Riot's own services through the Riot Client (same tokens as live mode, no HenrikDev quota), about one request a second:

| Path (host `pd.{shard}.a.pvp.net`) | Used for |
|---|---|
| `/match-history/v1/history/{puuid}?startIndex=N&endIndex=N+20` | Match ids, newest first: `History[].MatchID`, `Total` |
| `/match-details/v1/matches/{match_id}` | One match, reduced to a record row (totals only) for matches we don't hold |

**Fields read (unverified, parsed defensively):** `matchInfo.{matchId,mapId,gameStartMillis,gameLengthMillis,queueID,seasonId,isCompleted,gameVersion}`, `players[].{subject,teamId,characterId,competitiveTier,stats.{score,kills,deaths,assists}}`, `teams[].{teamId,roundsWon}`, `roundResults[].playerStats[].{subject,damage[].{receiver,damage,headshots,bodyshots,legshots}}`. Customs (empty `queueID`) and modes without two teams (deathmatch) are skipped. Map names come from the knowledge base `map_url`, agent names from `data/agents.json`, act names from `seasonal[].season.{id,short}` in the cached v3 MMR. Rows are stored in the `history` table with `"source": "riot"`.

**Ranked record by act:** the Lifetime tab also shows `seasonal[].{wins,games,end_tier}` from the cached v3 MMR, which is Riot's own count for every act, with or without match data.

## Derived rules

- **Side:** Red attacks rounds 1–12, Blue attacks 13–24; overtime alternates every round.
- **Sessions:** matches less than 45 minutes apart (end of one to start of the next).
- **Traded death:** a teammate killed your killer within 5 seconds.
- **Economy buckets** (your loadout value): eco < 2,000, force < 3,900, full buy otherwise; rounds 1 and 13 are pistol rounds.


The Lifetime kill table uses Riot account game counts and scoreboards. Kill records are keyed by player and match so two searched players can share a match. Attack and defense figures require identifiable sides and round kill counts that reconcile with the scoreboard. Missing round detail stays unavailable. Existing Riot-imported history rows can supply totals until a sync adds the side breakdown. HenrikDev records are available in Match archive and recent views.
