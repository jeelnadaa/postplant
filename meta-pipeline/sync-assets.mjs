#!/usr/bin/env node
// Meta pipeline, asset step (plan 15.4): sync game facts from valorant-api.com into data/.
//   * minimap transforms and callouts for every map in data/maps.json
//   * reports agents and maps that exist in the game but not in our knowledge base
//   * data/game-version.json (the pipeline's "game version changed" trigger)
// It never invents knowledge-base values: new agents and maps are listed for a human to fill in.
//
// Usage: node meta-pipeline/sync-assets.mjs [--offline <fixture-dir>] [--summary <file>]
// With --offline it reads maps.json / agents.json / version.json from the fixture dir (CI tests).
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const args = process.argv.slice(2);
const opt = (k) => (args.includes(k) ? args[args.indexOf(k) + 1] : null);
const offline = opt("--offline");
const summaryFile = opt("--summary");
const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const UA = "Postplant-meta-pipeline (+https://github.com/jeelnadaa/postplant)";

async function get(name, url) {
  if (offline) return JSON.parse(readFileSync(join(offline, `${name}.json`), "utf8"));
  for (let i = 0; i < 3; i++) {
    const r = await fetch(url, { headers: { "User-Agent": UA } });
    if (r.status === 403 || r.status === 429) throw new Error(`${url}: ${r.status}, stopping until the next run`);
    if (r.ok) return r.json();
    await new Promise((res) => setTimeout(res, 2000 * 2 ** i));
  }
  throw new Error(`${url}: failed after retries`);
}

const read = (f) => JSON.parse(readFileSync(join(root, "data", f), "utf8"));
const write = (f, v) => writeFileSync(join(root, "data", f), JSON.stringify(v, null, 1) + "\n");

export function applyMaps(doc, apiMaps) {
  const changes = [];
  for (const m of doc.maps) {
    const a = apiMaps.find((x) => x.uuid === m.map_id);
    if (!a) {
      changes.push(`${m.name}: not found in valorant-api.com (removed from the game?)`);
      continue;
    }
    if (a.xMultiplier != null) {
      const mm = { x_multiplier: a.xMultiplier, y_multiplier: a.yMultiplier, x_scalar: a.xScalarToAdd, y_scalar: a.yScalarToAdd, icon: a.displayIcon };
      if (JSON.stringify(mm) !== JSON.stringify(m.minimap)) changes.push(`${m.name}: minimap transform updated`);
      m.minimap = mm;
    }
    const callouts = (a.callouts ?? [])
      .filter((c) => c.location && c.regionName)
      .map((c) => [c.superRegionName ?? "", c.regionName, Math.round(c.location.x), Math.round(c.location.y)]);
    if (callouts.length) {
      if (JSON.stringify(callouts) !== JSON.stringify(m.callouts)) changes.push(`${m.name}: ${callouts.length} callouts`);
      m.callouts = callouts;
    }
    if (a.mapUrl && a.mapUrl !== m.map_url) {
      changes.push(`${m.name}: map_url ${m.map_url ?? "(none)"} → ${a.mapUrl}`);
      m.map_url = a.mapUrl;
    }
  }
  const known = new Set(doc.maps.map((m) => m.map_id));
  const missing = apiMaps
    .filter((x) => !known.has(x.uuid) && x.tacticalDescription && x.callouts?.length)
    .map((x) => `New playable map not in data/maps.json: ${x.displayName} (${x.uuid})`);
  return { changes, missing };
}

export function missingAgents(kb, apiAgents) {
  const known = new Set(kb.agents.map((a) => a.agent_id.toLowerCase()));
  return apiAgents
    .filter((a) => a.isPlayableCharacter && !known.has(a.uuid.toLowerCase()))
    .map((a) => `New agent not in data/agents.json: ${a.displayName} (${a.uuid}, ${a.role?.displayName ?? "no role"})`);
}

async function main() {
  const [maps, agents, version] = await Promise.all([
    get("maps", "https://valorant-api.com/v1/maps"),
    get("agents", "https://valorant-api.com/v1/agents?isPlayableCharacter=true"),
    get("version", "https://valorant-api.com/v1/version"),
  ]);
  const doc = read("maps.json");
  const { changes, missing } = applyMaps(doc, maps.data);
  const newAgents = missingAgents(read("agents.json"), agents.data);
  write("maps.json", doc);
  const gv = { version: version.data.version, riot_client_version: version.data.riotClientVersion, build_date: version.data.buildDate };
  let prev = {};
  try {
    prev = read("game-version.json");
  } catch {
    /* first run */
  }
  write("game-version.json", gv);
  const lines = [
    `Game version: ${gv.version}${prev.version && prev.version !== gv.version ? ` (was ${prev.version})` : ""}`,
    ...changes.map((c) => `- ${c}`),
    ...missing.map((c) => `- **${c}**`),
    ...newAgents.map((c) => `- **${c}**`),
  ];
  console.log(lines.join("\n"));
  if (summaryFile) writeFileSync(summaryFile, lines.join("\n") + "\n");
}

if (process.argv[1] && import.meta.url.endsWith(process.argv[1].replace(/\\/g, "/").split("/").pop())) {
  main().catch((e) => {
    console.error(e.message);
    process.exit(1);
  });
}
