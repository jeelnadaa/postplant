#!/usr/bin/env node
// Meta pipeline, validation step (plan 15.4 step 5). Runs in CI on every change to data/ and before
// the pipeline opens a PR. Exits 1 with a list of problems.
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROLES = ["Duelist", "Initiator", "Controller", "Sentinel"];
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export function validate({ agents, maps, pool, weights }) {
  const errs = [];
  const tags = new Set(agents.tags ?? []);
  if (!tags.size) errs.push("agents.json: no tags");
  const names = new Set();
  for (const a of agents.agents ?? []) {
    const w = `agents.json ${a.name ?? a.agent_id}`;
    if (!UUID.test(a.agent_id ?? "")) errs.push(`${w}: agent_id is not a UUID`);
    if (names.has(a.name)) errs.push(`${w}: duplicate name`);
    names.add(a.name);
    if (!ROLES.includes(a.role)) errs.push(`${w}: role "${a.role}" is not one of ${ROLES.join(", ")}`);
    for (const r of a.flex_roles ?? []) if (!ROLES.includes(r)) errs.push(`${w}: flex role "${r}" unknown`);
    for (const [t, v] of Object.entries(a.utility ?? {})) {
      if (!tags.has(t)) errs.push(`${w}: utility tag "${t}" not in tags`);
      if (typeof v !== "number" || v < 0 || v > 3) errs.push(`${w}: utility ${t}=${v} outside 0–3`);
    }
  }
  const mapIds = new Set();
  for (const m of maps.maps ?? []) {
    const w = `maps.json ${m.name ?? m.map_id}`;
    if (!UUID.test(m.map_id ?? "")) errs.push(`${w}: map_id is not a UUID`);
    mapIds.add(m.map_id);
    for (const [t, v] of Object.entries(m.utility_needs ?? {})) {
      if (!tags.has(t)) errs.push(`${w}: need "${t}" not in tags`);
      if (typeof v !== "number" || v < 0 || v > 5) errs.push(`${w}: need ${t}=${v} outside 0–5`);
    }
    const mix = m.role_mix ?? {};
    for (const r of Object.keys(mix)) if (!ROLES.includes(r)) errs.push(`${w}: role_mix "${r}" unknown`);
    const total = Object.values(mix).reduce((s, x) => s + x, 0);
    if (Math.abs(total - 5) > 0.51) errs.push(`${w}: role_mix sums to ${total}, expected 5`);
    if (m.minimap) {
      for (const k of ["x_multiplier", "y_multiplier", "x_scalar", "y_scalar"])
        if (typeof m.minimap[k] !== "number") errs.push(`${w}: minimap.${k} missing`);
    }
    for (const c of m.callouts ?? []) if (!Array.isArray(c) || c.length !== 4) errs.push(`${w}: malformed callout ${JSON.stringify(c)}`);
  }
  for (const id of pool.maps ?? []) {
    if (!mapIds.has(id)) errs.push(`map-pool.json: ${id} is not in maps.json`);
    const m = (maps.maps ?? []).find((x) => x.map_id === id);
    if (m && !m.minimap) errs.push(`map-pool.json: ${m.name} has no minimap transform`);
  }
  if ((pool.maps ?? []).length !== 7) errs.push(`map-pool.json: ${pool.maps?.length ?? 0} maps, a competitive pool has 7`);
  for (const [k, v] of Object.entries(weights)) if (typeof v !== "number" || v < 0 || v > 50) errs.push(`advisor-weights.json: ${k}=${v} implausible`);
  return errs;
}

if (process.argv[1]?.replace(/\\/g, "/").endsWith("meta-pipeline/validate.mjs")) {
  const dir = process.argv[2] ?? join(new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1"), "data");
  const r = (f) => JSON.parse(readFileSync(join(dir, f), "utf8"));
  const errs = validate({ agents: r("agents.json"), maps: r("maps.json"), pool: r("map-pool.json"), weights: r("advisor-weights.json") });
  if (errs.length) {
    console.error(errs.map((e) => `✗ ${e}`).join("\n"));
    process.exit(1);
  }
  console.log("data/ is valid");
}
