// Fixture tests for the meta pipeline: run offline in CI with `node --test meta-pipeline/`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { applyMaps, missingAgents } from "./sync-assets.mjs";
import { validate } from "./validate.mjs";

const fx = (f) => JSON.parse(readFileSync(new URL(`./fixtures/${f}`, import.meta.url), "utf8"));
const data = (f) => JSON.parse(readFileSync(new URL(`../data/${f}`, import.meta.url), "utf8"));

test("repo data validates", () => {
  const errs = validate({ agents: data("agents.json"), maps: data("maps.json"), pool: data("map-pool.json"), weights: data("advisor-weights.json") });
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
