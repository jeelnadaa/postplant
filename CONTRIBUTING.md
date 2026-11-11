# Contributing to Postplant

Thanks for helping. Small, focused pull requests are easiest to review.

## Setup

Node 20+, Rust stable (MSVC) and the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) for Windows.

```bash
npm install
npm run tauri dev
```

## Before you open a PR

```bash
npm run check
npm test
cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
```

CI runs the same checks on every pull request.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org): `feat:`, `fix:`, `perf:`, `refactor:`, `docs:`, `ci:`, `chore:`. Merging to `main` releases automatically, and the commit types decide the version (`feat` → minor, others → patch, `!` → breaking). Put `[skip release]` in the message to leave a commit out of releases.

## Where things live

| Path | What |
|---|---|
| `src/` | Svelte 5 frontend: `screens/`, `components/`, `lib/` (API wrappers, store, formatting) |
| `src/Overlay.svelte` | The in-game overlay window |
| `src-tauri/src/` | Rust backend: `api.rs` (HenrikDev client and budget governor), `sync.rs`, `parse.rs`, `db.rs`, `queries.rs`, `advisor.rs`, `transfer.rs` (backup/export), `recordings.rs`, `overlay.rs`, `commands.rs` |
| `data/` | Comp Advisor knowledge base (agents, maps, map pool, weights). No Rust needed to improve it. |
| `design/` | Design system and every screen as static HTML |
| `site/` | Website (Cloudflare, `wrangler.jsonc`) |
| `plan.md` | The full plan; `PROGRESS.md` tracks it |

## Ground rules

- No scouting: nothing may show other players' data during agent select.
- No memory reading, no game file changes.
- Never look up players whose names are hidden, and never try to reveal them.
- The API key never reaches the webview, logs, backups or exports.
- Background API use stays within the budget governor (`api.rs`).
- Keep the overlay cheap: no blur, no looping animations, transform/opacity only.
