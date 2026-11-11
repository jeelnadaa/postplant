#!/usr/bin/env node
// Meta pipeline, Tier 1 (plan 15.3): per-map agent pick and win rates and observed 5-agent comps
// from professional matches, through the LiquipediaDB API.
//
// Why Liquipedia: its content is CC-BY-SA 3.0 (attribution required, recorded in the output) and
// it offers an API for exactly this. VLR.gg and Metabot were reviewed and excluded; see
// docs/meta-sources.md. Liquipedia's terms forbid automated access to its HTML pages, so this only
// ever calls the API, and only with an approved key (repository secret LIQUIPEDIA_API_KEY).
// Without a key it leaves data/meta-tier1.json untouched and exits cleanly.
//
// Usage: node meta-pipeline/liquipedia.mjs [--offline <matches.json>] [--days 60] [--summary <file>]
import { readFileSync, writeFileSync, appendFileSync } from "node:fs";
import { join } from "node:path";

const API = "https://api.liquipedia.net/api/v3/match";
const UA = "Postplant-meta-pipeline/1.0 (+https://github.com/jeelnadaa/postplant)";
export const MIN_MAP_GAMES = 8; // below this a map's numbers are shown as "thin" by the app
const PAGE = 200;
const MAX_PAGES = 6;

/** "KAY/O" -> "kayo", "Killjoy " -> "killjoy" */
export const norm = (s) => String(s ?? "").toLowerCase().replace(/[^a-z0-9]/g, "");

/** The ten (team, agent) picks of one game, in either shape LiquipediaDB has used. */
export function gameTeams(g) {
  const teams = [[], []];
  if (g?.participants && typeof g.participants === "object") {
    for (const [k, p] of Object.entries(g.participants)) {
      const t = Number(k.split("_")[0]) - 1;
      if ((t === 0 || t === 1) && p?.agent) teams[t].push(p.agent);
    }
  }
  if (!teams[0].length && Array.isArray(g?.opponents)) {
    g.opponents.slice(0, 2).forEach((o, t) => {
      for (const p of o?.players ?? []) if (p?.agent) teams[t].push(p.agent);
    });
  }
  return teams;
}

/**
 * Aggregate LiquipediaDB match rows into per-map stats. Only finished games on a known map with
 * five known agents on each side count. Returns { maps, games, skipped }.
 */
export function aggregate(matches, kb) {
  const agentBy = new Map(kb.agents.map((a) => [norm(a.name), a.name]));
  const mapBy = new Map(kb.maps.map((m) => [norm(m.name), m.name]));
  const maps = {};
  let games = 0;
  let skipped = 0;
  for (const m of matches) {
    const gs = Array.isArray(m.match2games) ? m.match2games : [];
    for (const g of gs) {
      const map = mapBy.get(norm(g.map));
      const winner = Number(g.winner);
      const teams = gameTeams(g).map((t) => t.map((a) => agentBy.get(norm(a))));
      const ok = map && (winner === 1 || winner === 2) && teams.every((t) => t.length === 5 && t.every(Boolean) && new Set(t).size === 5);
      if (!ok) {
        if (gs.length && g.map) skipped++;
        continue;
      }
      games++;
      const s = (maps[map] ??= { n: 0, agents: {}, comps: {} });
      s.n++;
      teams.forEach((t, i) => {
        const won = winner === i + 1 ? 1 : 0;
        for (const a of t) {
          const x = (s.agents[a] ??= { picks: 0, wins: 0 });
          x.picks++;
          x.wins += won;
        }
        const key = [...t].sort().join("|");
        const c = (s.comps[key] ??= { n: 0, wins: 0 });
        c.n++;
        c.wins += won;
      });
    }
  }
  const out = {};
  for (const [map, s] of Object.entries(maps).sort()) {
    out[map] = {
      n: s.n,
      agents: Object.entries(s.agents)
        .map(([name, x]) => ({ name, picks: x.picks, pick: Math.round((1000 * x.picks) / (2 * s.n)) / 10, win: Math.round((1000 * x.wins) / x.picks) / 10 }))
        .sort((a, b) => b.picks - a.picks || a.name.localeCompare(b.name)),
      comps: Object.entries(s.comps)
        .map(([k, c]) => ({ agents: k.split("|"), n: c.n, wins: c.wins }))
        .filter((c) => c.n >= 2)
        .sort((a, b) => b.n - a.n || b.wins - a.wins)
        .slice(0, 8),
    };
  }
  return { maps: out, games, skipped };
}

/** A readable diff for the PR body: biggest pick-rate moves per map. */
export function diffSummary(before, after) {
  const lines = [];
  for (const [map, a] of Object.entries(after.maps ?? {})) {
    const b = before?.maps?.[map];
    if (!b) {
      lines.push(`- ${map}: new, ${a.n} games`);
      continue;
    }
    const moves = a.agents
      .map((x) => ({ name: x.name, d: x.pick - (b.agents.find((y) => y.name === x.name)?.pick ?? 0), now: x.pick }))
      .filter((x) => Math.abs(x.d) >= 5)
      .sort((x, y) => Math.abs(y.d) - Math.abs(x.d))
      .slice(0, 3)
      .map((x) => `${x.name} ${x.now - x.d}% → ${x.now}%`);
    lines.push(`- ${map}: ${b.n} → ${a.n} games${moves.length ? `; ${moves.join(", ")}` : ""}`);
  }
  return lines.join("\n");
}

async function fetchMatches(key, since) {
  const rows = [];
  for (let page = 0; page < MAX_PAGES; page++) {
    const q = new URLSearchParams({
      wiki: "valorant",
      conditions: `[[date::>${since}]] AND [[finished::1]]`,
      query: "match2id, date, liquipediatier, tournament, match2games",
      order: "date DESC",
      limit: String(PAGE),
      offset: String(page * PAGE),
    });
    let r;
    for (let i = 0; i < 3; i++) {
      r = await fetch(`${API}?${q}`, { headers: { Authorization: `Apikey ${key}`, "User-Agent": UA, "Accept-Encoding": "gzip" } });
      if (r.status === 403 || r.status === 429) throw new Error(`LiquipediaDB returned ${r.status}; stopping until the next run`);
      if (r.ok) break;
      await new Promise((res) => setTimeout(res, 3000 * 2 ** i));
    }
    if (!r.ok) throw new Error(`LiquipediaDB returned ${r.status}`);
    const body = await r.json();
    const batch = body.result ?? [];
    rows.push(...batch);
    if (batch.length < PAGE) break;
    await new Promise((res) => setTimeout(res, 2500)); // well inside any plan's rate limit
  }
  return rows;
}

async function main() {
  const args = process.argv.slice(2);
  const opt = (k) => (args.includes(k) ? args[args.indexOf(k) + 1] : null);
  const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
  const read = (f) => JSON.parse(readFileSync(join(root, "data", f), "utf8"));
  const summary = opt("--summary");
  const say = (s) => {
    console.log(s);
    if (summary) appendFileSync(summary, s + "\n");
  };
  const days = Number(opt("--days") ?? 60);
  const to = new Date();
  const from = new Date(to.getTime() - days * 86400e3);
  const since = from.toISOString().slice(0, 10);

  let matches;
  if (opt("--offline")) matches = JSON.parse(readFileSync(opt("--offline"), "utf8")).result;
  else {
    const key = process.env.LIQUIPEDIA_API_KEY;
    if (!key) {
      say("Tier 1 (pro play): skipped, no LIQUIPEDIA_API_KEY secret. Request a LiquipediaDB key and add it to enable pro data.");
      return;
    }
    matches = await fetchMatches(key, since);
  }
  const kb = { agents: read("agents.json").agents, maps: read("maps.json").maps };
  const { maps, games, skipped } = aggregate(matches, kb);
  if (!games) {
    // Most likely the API's shape changed: fail loudly instead of publishing an empty dataset.
    const sample = JSON.stringify(matches.find((m) => m.match2games?.length)?.match2games?.[0] ?? matches[0] ?? {}).slice(0, 600);
    throw new Error(`No usable games in ${matches.length} matches (${skipped} skipped). First game looks like: ${sample}`);
  }
  const before = read("meta-tier1.json");
  const doc = {
    tier: 1,
    label: "Pro play",
    source: "Liquipedia",
    source_url: "https://liquipedia.net/valorant/",
    license: "CC-BY-SA 3.0",
    attribution: "Pro match data from Liquipedia (liquipedia.net), licensed CC-BY-SA 3.0.",
    fetched_at: to.toISOString(),
    window: { from: since, to: to.toISOString().slice(0, 10) },
    matches: matches.length,
    games,
    maps,
  };
  writeFileSync(join(root, "data", "meta-tier1.json"), JSON.stringify(doc, null, 1) + "\n");
  say(`Tier 1 (pro play, Liquipedia): ${games} games from ${matches.length} matches since ${since}; ${skipped} games skipped (incomplete).`);
  const d = diffSummary(before, doc);
  if (d) say(d);
}

if (process.argv[1] && import.meta.url.endsWith(process.argv[1].replace(/\\/g, "/").split("/").pop())) {
  main().catch((e) => {
    console.error(e.message);
    process.exit(1);
  });
}
