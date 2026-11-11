# Export formats

## `.postplant` backup

A zip file with exactly two entries:

| Entry | Content |
|---|---|
| `manifest.json` | `format` (`"postplant-backup"`), `format_version`, `app_version`, `schema_version`, `created_at` (RFC 3339), `owner` (`puuid`, `name`, `tag`), `counts` per table, `sha256` of `data.sqlite` |
| `data.sqlite` | A `VACUUM INTO` copy of the database. Never contains the API key. |

On import Postplant checks: size (max 500 MB), that only these two entries exist, the SHA-256, `PRAGMA integrity_check`, and that the schema version is not newer than the app. Then it shows a preview (new matches, matches already present, new notes, notes that differ) before anything is written.

- **Merge** adds what's missing. Notes that differ follow your choice: keep the newer, keep mine, or keep both joined.
- **Replace** makes a safety backup first, then replaces all data. You type `replace` to confirm.
- Settings are only imported if you tick the box, and only known keys (theme, font, overlay, hotkeys and similar).

## JSON export

One JSON document:

| Key | Content |
|---|---|
| `format`, `version` | `"postplant-export"`, `1` |
| `app_version`, `created_at` | App version and RFC 3339 time |
| `owner` | Your `puuid`, `name`, `tag` |
| `anonymised` | Whether other players were anonymised |
| `matches` | Every match as the raw HenrikDev v4 match object Postplant stored |
| `rank_history` | Rank snapshots (`mmr_snapshots` rows) |
| `rr_changes` | RR change per match |
| `notes` | Your player notes (empty when anonymised) |
| `agent_pool` | Agents you own and comfort |
| `review_items` | Your review list |

With **anonymised** on, every other player's `name` becomes a stable `Player N` and `tag` becomes `0000`, and your notes are left out. Importing a JSON export re-parses the matches (nothing in the file is trusted as-is) and adds the ones you don't already have; it never changes existing matches or notes.
