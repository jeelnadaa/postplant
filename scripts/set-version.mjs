#!/usr/bin/env node
// Stamps one version into package.json, src-tauri/tauri.conf.json and src-tauri/Cargo.toml.
//   node scripts/set-version.mjs 1.2.3
import { readFileSync, writeFileSync } from "node:fs";

const v = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(v ?? "")) {
  console.error("usage: set-version.mjs X.Y.Z");
  process.exit(1);
}
const at = (p) => new URL(`../${p}`, import.meta.url);
for (const f of ["package.json", "src-tauri/tauri.conf.json"]) {
  const j = JSON.parse(readFileSync(at(f), "utf8"));
  j.version = v;
  writeFileSync(at(f), JSON.stringify(j, null, 2) + "\n");
}
const cargo = readFileSync(at("src-tauri/Cargo.toml"), "utf8");
writeFileSync(at("src-tauri/Cargo.toml"), cargo.replace(/^version = ".*"$/m, `version = "${v}"`));
console.log(`version set to ${v}`);
