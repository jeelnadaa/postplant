<script lang="ts">
  // Career stats for anyone: overall plus per agent, map, agent × map, patch, weapon and queue.
  // `d` is the output of career::compute (or from_stored for the quick summary, which lacks round data).
  import type { J } from "../lib/api";
  import { queueLabel } from "../lib/format";
  import Agent from "./Agent.svelte";

  let { d, quick = false }: { d: J; quick?: boolean } = $props();

  type Col = [key: string, label: string, fmt?: "pct" | "num"];
  const BASIC: Col[] = [["games", "Games"], ["win", "Win", "pct"], ["kd", "K/D"], ["kda", "KDA"], ["acs", "ACS"], ["adr", "ADR"], ["hs", "HS", "pct"]];
  const DEEP: Col[] = [
    ["kast", "KAST", "pct"], ["fb", "FB"], ["fd", "FD"], ["plants", "Plants"], ["defuses", "Defuses"], ["defuse_rate", "Defuse %", "pct"],
    ["post_plant_kills", "Post-plant K"], ["retake_kills", "Retake K"], ["clutches", "Clutches"], ["k3", "3K"], ["k4", "4K"], ["aces", "Aces"],
    ["atk_win", "ATK", "pct"], ["def_win", "DEF", "pct"],
  ];
  const cols = $derived(quick ? BASIC : [...BASIC, ...DEEP]);
  const VIEWS = $derived(quick ? [["agents", "Agents"], ["maps", "Maps"]] : [["agents", "Agents"], ["maps", "Maps"], ["combos", "Agent × map"], ["patches", "Patches"], ["weapons", "Weapons"], ["queues", "Modes"]]);
  let view = $state("agents");
  let sortKey = $state("games");
  let desc = $state(true);

  const o = $derived(d?.overall ?? {});
  const fmt = (v: J, f?: string) => (v === null || v === undefined ? "—" : f === "pct" ? `${v}%` : v);
  const rows = $derived.by(() => {
    const list = [...(d?.[view] ?? [])];
    if (view === "weapons" || view === "queues") return list;
    return list.sort((a: J, b: J) => ((desc ? -1 : 1) * ((a[sortKey] ?? -1) - (b[sortKey] ?? -1))));
  });
  function sortBy(k: string) {
    if (sortKey === k) desc = !desc;
    else { sortKey = k; desc = true; }
  }
  const tiles = $derived<[string, string, string?][]>(quick
    ? [[`${o.win}%`, "Win rate", `${o.wins}W ${o.losses}L`], [o.kd, "K/D"], [o.acs, "ACS"], [o.adr, "ADR"], [`${o.hs}%`, "Headshot"], [o.games, "Matches"]]
    : [
        [`${o.win}%`, "Win rate", `${o.wins}W ${o.losses}L${o.draws ? ` ${o.draws}D` : ""}`], [o.kd, "K/D", `KDA ${o.kda}`], [o.acs, "ACS"], [o.adr, "ADR", `ΔDmg ${o.dd > 0 ? "+" : ""}${o.dd}`],
        [`${o.hs}%`, "Headshot"], [`${o.kast}%`, "KAST"], [`${o.fb_win}%`, "First duels won", `${o.fb} FB · ${o.fd} FD`],
        [`${o.plant_rate}%`, "Plant rate", `${o.plants} plants`], [`${o.defuse_rate}%`, "Defuse rate", `${o.defuses} defuses on planted D rounds`],
        [o.post_plant_kills, "Post-plant kills", `${o.retake_kills} retake kills`], [`${o.clutch_win}%`, "Clutches", `${o.clutches} of ${o.clutch_att}`],
        [o.aces, "Aces", `${o.k3} 3K · ${o.k4} 4K`], [`${o.atk_win}%`, "Attack rounds"], [`${o.def_win}%`, "Defense rounds"],
        [`${o.traded_deaths}%`, "Deaths traded", `${o.trades} trades made`], [o.dmg_per_1k ?? "—", "Damage per 1k credits"],
      ]);
</script>

{#if !o.games}
  <div class="card empty-inline">No matches to show yet.</div>
{:else}
  <div class="stat-grid mb-4" style="grid-template-columns:repeat(auto-fill,minmax(150px,1fr))">
    {#each tiles as [v, l, sub] (l)}
      <div class="stat"><div class="v">{v}</div><div class="l">{l}</div>{#if sub}<div class="faint" style="font-size:11px;margin-top:2px">{sub}</div>{/if}</div>
    {/each}
  </div>
  {#if d.form?.length}
    <div class="row mb-4" style="gap:4px;flex-wrap:wrap" aria-label="Recent results">
      {#each d.form as f, i (i)}<span class="badge {f.result === 'W' ? 'win' : f.result === 'L' ? 'loss' : ''}" title="{f.agent} · {f.acs} ACS">{f.result}</span>{/each}
    </div>
  {/if}
  <div class="tabs mb-2">
    {#each VIEWS as [k, l] (k)}<button role="tab" class="tab" aria-selected={view === k} onclick={() => (view = k)}>{l}{#if d[k]?.length}<span class="count">{d[k].length}</span>{/if}</button>{/each}
  </div>
  <div class="card" style="overflow-x:auto;padding:0">
    {#if view === "weapons"}
      <div class="stack" style="gap:10px;padding:16px">{#each d.weapons as w (w.name)}
        <div class="bar-h"><span>{w.name}</span><span class="b"><i style="width:{w.share}%"></i></span><span class="mono" style="text-align:right">{w.kills} · {w.share}%</span></div>{:else}<p class="faint">No kill data.</p>{/each}</div>
    {:else if view === "queues"}
      <table class="table"><thead><tr><th>Mode</th><th class="num">Games</th><th class="num">Win</th></tr></thead><tbody>
        {#each d.queues as q (q.name)}<tr><td>{queueLabel(q.name)}</td><td class="num">{q.games}</td><td class="num">{q.win}%</td></tr>{/each}
      </tbody></table>
    {:else}
      <table class="table career-t"><thead><tr>
        <th>{view === "agents" ? "Agent" : view === "maps" ? "Map" : view === "patches" ? "Patch" : "Agent · map"}</th>
        {#each cols as [k, l] (k)}<th class="num sortable" aria-sort={sortKey === k ? (desc ? "descending" : "ascending") : undefined} onclick={() => sortBy(k)}>{l}</th>{/each}
      </tr></thead><tbody>
        {#each rows as r, i (i)}
          <tr>
            <td>{#if view === "agents"}<div class="player"><Agent name={r.name} id={r.id} sm /><b>{r.name}</b></div>
              {:else if view === "combos"}<div class="player"><Agent name={r.agent} sm /><b>{r.agent}</b><span class="faint">· {r.map}</span></div>
              {:else}<b>{r.name}</b>{/if}</td>
            {#each cols as [k, , f] (k)}<td class="num mono">{fmt(r[k], f)}</td>{/each}
          </tr>
        {/each}
      </tbody></table>
    {/if}
  </div>
  {#if quick}<p class="faint mt-2" style="font-size:12px">Quick summary from the last {o.games} matches. Load detailed stats for defuses, clutches, KAST, trades and more.</p>{/if}
{/if}

<style>
  .career-t th.sortable { cursor: pointer; user-select: none; white-space: nowrap; }
  .career-t th.sortable:hover { color: var(--text); }
  .career-t td, .career-t th { white-space: nowrap; }
</style>
