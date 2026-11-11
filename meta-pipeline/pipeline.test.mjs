// Fixture tests for the meta pipeline: run offline in CI with `node --test meta-pipeline/`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { applyMaps, missingAgents } from "./sync-assets.mjs";
import { validate, validateTier1 } from "./validate.mjs";
import { aggregate, diffSummary } from "./liquipedia.mjs";

const fx = (f) => JSON.parse(readFileSync(new URL(`./fixtures/${f}`, import.meta.url), "utf8"));
const data = (f) => JSON.parse(readFileSync(new URL(`../data/${f}`, import.meta.url), "utf8"));

test("repo data validates", () => {
  const errs = validate({ agents: data("agents.json"), maps: data("maps.json"), pool: data("map-pool.json"), weights: data("advisor-weights.json"), tier1: data("meta-tier1.json") });
  assert.deepEqual(errs, []);
});

test("validator catches bad data", () => {
  const agents = { tags: ["smokes"], agents: [{ agent_id: "nope", name: "X", role: "Tank", utility: { smokes: 9, fly: 1 } }] };
  const maps = { maps: [{ map_id: "224b0a95-48b9-f703-1bd8-67aca101a61f", name: "M", utility_needs: {}, role_mix: { Duelist: 4 } }] };
  const errs = validate({ agents, maps, pool: { maps: ["224b0a95-48b9-f703-1bd8-67aca101a61f"] }, weights: { comfort: -1 } });
  for (const want of ["not a UUID", "role \"Tank\"", "smokes=9", "\"fly\"", "sums to 4", "no minimap", "7", "comfort=-1"])
    assert.ok(errs.some((e) => e.includes(want)), `expected an error mentioning ${want}: ${errs.join(" | ")}`);
});

test("asset sync applies minimap and callouts, reports new maps and agents", () => {
  const doc = { maps: [{ map_id: "m-1", name: "Known", map_url: "/old" }] };
  const { changes, missing } = applyMaps(doc, fx("maps.json").data);
  assert.equal(doc.maps[0].minimap.x_multiplier, 0.0001);
  assert.deepEqual(doc.maps[0].callouts[0], ["A", "Site", 100, -200]);
  assert.equal(doc.maps[0].map_url, "/Game/Maps/Known");
  assert.ok(changes.some((c) => c.includes("callouts")));
  assert.deepEqual(missing, ["New playable map not in data/maps.json: Fresh (m-2)"]);
  const agents = missingAgents({ agents: [{ agent_id: "A-1" }] }, fx("agents.json").data);
  assert.deepEqual(agents, ["New agent not in data/agents.json: Newbie (a-2, Sentinel)"]);
});

test("Liquipedia adapter: both game shapes, name normalising, incomplete games skipped", () => {
  const kb = { agents: data("agents.json").agents, maps: data("maps.json").maps };
  const { maps, games, skipped } = aggregate(fx("liquipedia-matches.json").result, kb);
  assert.equal(games, 3); // 2 Lotus + 1 Haven; the empty game and the "Mystery" agent game are dropped
  assert.equal(skipped, 2);
  const lotus = maps.Lotus;
  assert.equal(lotus.n, 2);
  const by = Object.fromEntries(lotus.agents.map((a) => [a.name, a]));
  assert.equal(by.Omen.picks, 4);
  assert.equal(by.Omen.pick, 100);
  assert.equal(by["KAY/O"].picks, 3);
  assert.equal(by["KAY/O"].pick, 75);
  assert.equal(by["KAY/O"].win, 66.7); // won M1 G1 (team 1) and M2 G1 (team 2), lost M2 G1 (team 1)
  assert.deepEqual(lotus.comps[0].agents, ["Fade", "KAY/O", "Omen", "Raze", "Viper"]);
  assert.equal(lotus.comps[0].n, 3);
  const doc = { tier: 1, source: "Liquipedia", license: "CC-BY-SA 3.0", attribution: "x", maps };
  assert.deepEqual(validateTier1(doc, data("agents.json"), data("maps.json")), []);
  assert.match(diffSummary({ maps: {} }, doc), /Lotus: new, 2 games/);
});

test("tier 1 validator catches broken numbers", () => {
  const bad = { tier: 1, source: "x", license: "x", attribution: "x", maps: { Nowhere: { n: 1, agents: [{ name: "Omen", picks: 3, pick: 150, win: 50 }], comps: [{ agents: ["Omen"], n: 9, wins: 10 }] } } };
  const errs = validateTier1(bad, data("agents.json"), data("maps.json"));
  for (const want of ["\"Nowhere\"", "pick=150", "3 picks", "not 5 different", "implausible"])
    assert.ok(errs.some((e) => e.includes(want)), `expected an error mentioning ${want}: ${errs.join(" | ")}`);
});
