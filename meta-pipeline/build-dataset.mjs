#!/usr/bin/env node
// Meta pipeline, publish step (plan 15.6): bundle data/ into one dataset file the app can download.
// CI signs it with the same minisign key as app updates (tauri signer) and uploads both files to the
// rolling "dataset" GitHub release. The app verifies the signature before loading anything.
//
// version = commit time (unix seconds) of the newest commit touching data/. The app is built with the
// same number for its bundled copy (src-tauri/build.rs), so it only downloads strictly newer data.
//
// Usage: node meta-pipeline/build-dataset.mjs [out-file]   (default out/dataset.json)
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { validate } from "./validate.mjs";

export const FORMAT = 1;
export const FILES = { agents: "agents.json", maps: "maps.json", map_pool: "map-pool.json", weights: "advisor-weights.json", tier1: "meta-tier1.json" };

export function dataVersion(root) {
  try {
    return Number(execFileSync("git", ["log", "-1", "--format=%ct", "--", "data"], { cwd: root, encoding: "utf8" }).trim()) || 0;
  } catch {
    return 0;
  }
}

const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
if (process.argv[1]?.replace(/\\/g, "/").endsWith("meta-pipeline/build-dataset.mjs")) {
  const out = process.argv[2] ?? join(root, "out", "dataset.json");
  const read = (f) => JSON.parse(readFileSync(join(root, "data", f), "utf8"));
  const files = Object.fromEntries(Object.entries(FILES).map(([k, f]) => [k, read(f)]));
  const errs = validate({ agents: files.agents, maps: files.maps, pool: files.map_pool, weights: files.weights, tier1: files.tier1 });
  if (errs.length) {
    console.error(errs.join("\n"));
    process.exit(1);
  }
  const version = dataVersion(root);
  if (!version) {
    console.error("Couldn't read the data/ commit time from git");
    process.exit(1);
  }
  const doc = { format: FORMAT, version, game_version: read("game-version.json").version ?? null, created_at: new Date().toISOString(), files };
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, JSON.stringify(doc));
  console.log(`dataset version ${version} (${new Date(version * 1000).toISOString()}), ${Object.keys(files).length} files → ${out}`);
}
