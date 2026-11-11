<script lang="ts">
  import { api, type J } from "../lib/api";
  import { app, nav } from "../lib/store.svelte";
  import Agent from "../components/Agent.svelte";
  import Career from "../components/Career.svelte";

  let { params = {} }: { params?: Record<string, string> } = $props();
  let days = $state(30);
  let d = $state<J>(null);
  $effect(() => {
    void app.version;
    api.analytics(days, "competitive").then((r) => (d = r));
  });
  const o = $derived(d?.overall);
  // svelte-ignore state_referenced_locally
  let view = $state(params.view === "lifetime" ? "lifetime" : "overview");
  let cq = $state("competitive");
  let career = $state<J>(null);
  $effect(() => {
    void app.version;
    if (view === "lifetime") api.myCareer(cq).then((r) => (career = r)).catch(() => (career = null));
  });
</script>

<div class="page-head">
  <div><h1 class="h1">Analytics</h1><div class="sub">Competitive · {o?.games ?? 0} matches · all from this PC, zero API requests</div></div>
  <span class="spacer"></span>
  <div class="segmented">{#each [[7, "7 d"], [30, "30 d"], [90, "90 d"], [0, "All"]] as [v, l] (v)}<button aria-pressed={days === v} onclick={() => (days = v as number)}>{l}</button>{/each}</div>
</div>
<div class="tabs mb-4"><button role="tab" class="tab" aria-selected={view === "overview"} onclick={() => (view = "overview")}>Overview</button><button role="tab" class="tab" aria-selected={view === "lifetime"} onclick={() => (view = "lifetime")}>Lifetime</button><button role="tab" class="tab" aria-selected="false" onclick={() => nav("habits")}>When you play best</button></div>
{#if view === "lifetime"}
  <div class="segmented mb-4">{#each [["competitive", "Competitive"], ["unrated", "Unrated"], ["swiftplay", "Swiftplay"], ["", "All modes"]] as [v, l] (v)}<button aria-pressed={cq === v} onclick={() => (cq = v)}>{l}</button>{/each}</div>
  {#if career}<Career d={career} />{:else}<div class="card" style="height:200px"><div class="skeleton" style="height:100%"></div></div>{/if}
{:else if d && o.games === 0}
  <div class="card empty-inline">No competitive matches in this period.</div>
{:else if d}
  <div class="grid-12">
    <div class="col-12 stat-grid" style="grid-template-columns:repeat(6,1fr)">
      {#each [[`${o.win}%`, "Win rate"], [o.acs, "ACS"], [o.kd, "K/D"], [`${o.hs}%`, "Headshot"], [o.adr, "ADR"], [d.untraded !== null ? `${d.untraded}%` : "—", "Untraded deaths"]] as [v, l] (l)}
        <div class="stat"><div class="v">{v}</div><div class="l">{l}</div></div>
      {/each}
    </div>
    <div class="card col-6"><div class="card-head"><span class="h3">By agent</span></div>
      <table class="table"><thead><tr><th>Agent</th><th class="num">Games</th><th class="num">Win</th><th class="num">ACS</th><th class="num">K/D</th></tr></thead><tbody>
        {#each d.agents.slice(0, 8) as a (a.name)}<tr><td><div class="player"><Agent name={a.name} id={a.agent_id} sm /><b>{a.name}</b></div></td><td class="num">{a.games}</td><td class="num">{a.win}%</td><td class="num">{a.acs}</td><td class="num">{a.kd}</td></tr>{/each}
      </tbody></table></div>
    <div class="card col-6"><div class="card-head"><span class="h3">By map &amp; side</span><span class="spacer"></span><span class="faint" style="font-size:12px">round win %</span></div>
      <table class="table"><thead><tr><th>Map</th><th class="num">Games</th><th class="num atk">ATK</th><th class="num def">DEF</th><th class="num">Match win</th></tr></thead><tbody>
        {#each d.maps as m (m.name)}<tr><td><b>{m.name}</b></td><td class="num">{m.games}</td><td class="num">{m.atk ?? "—"}{m.atk !== null ? "%" : ""}</td><td class="num">{m.def ?? "—"}{m.def !== null ? "%" : ""}</td><td class="num">{m.win}%</td></tr>{/each}
      </tbody></table></div>
    <div class="card col-6"><div class="card-head"><span class="h3">Economy</span><span class="spacer"></span><span class="faint" style="font-size:12px">round win % by your loadout</span></div>
      <div class="stack" style="gap:10px">{#each d.economy as e (e.name)}
        <div class="bar-h"><span>{e.name}</span><span class="b"><i style="width:{e.win ?? 0}%;background:var(--chart-2)"></i></span><span class="mono" style="text-align:right">{e.win ?? "—"}{e.win !== null ? "%" : ""}</span></div>{/each}</div>
      <p class="faint" style="font-size:12px;margin:10px 0 0">Eco under 2,000 · force under 3,900 · pistol = rounds 1 and 13</p></div>
    <div class="card col-6"><div class="card-head"><span class="h3">Your kills by weapon</span></div>
      <div class="stack" style="gap:10px">{#each d.weapons as w (w.name)}
        <div class="bar-h"><span>{w.name}</span><span class="b"><i style="width:{w.share}%"></i></span><span class="mono" style="text-align:right">{w.share}%</span></div>{:else}<p class="faint">No kill data yet.</p>{/each}</div></div>
  </div>
{/if}
