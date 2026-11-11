<script lang="ts">
  // 2D round replay: step through each kill and see where all ten players stood at that moment.
  import { api, errText, type J } from "../lib/api";
  import { mmss, toMinimap } from "../lib/format";
  import { app } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";

  let { id, myTeam }: { id: string; myTeam: string } = $props();
  let d = $state<J>(null);
  let error = $state("");
  let round = $state(0);
  let step = $state(0);

  $effect(() => {
    api.replay(id).then((r) => (d = r)).catch((e) => (error = errText(e)));
  });
  const mm = $derived(app.kb?.maps?.find((m: J) => m.name === d?.map)?.minimap);
  const r = $derived(d?.rounds?.[round]);
  const kills = $derived(r?.kills ?? []);
  const k = $derived(kills[Math.min(step, kills.length - 1)]);
  const pmap = $derived(new Map<string, J>((d?.players ?? []).map((p: J) => [p.puuid, p])));
  const dead = $derived(new Set<string>(kills.slice(0, step).map((x: J) => x.victim)));
  const pt = (x: number, y: number) => {
    const v = toMinimap(mm, x, y);
    return v ? { x: v[0] * 1000, y: v[1] * 1000 } : null;
  };
  function setRound(n: number) {
    round = Math.max(0, Math.min((d?.rounds?.length ?? 1) - 1, n));
    step = 0;
  }
</script>

{#if error}<div class="banner danger"><Icon name="alert" />{error}</div>
{:else if d && !mm}<div class="card empty-inline">No minimap data for {d.map}.</div>
{:else if d}
  <div class="grid-12">
    <div class="card col-8" style="padding:8px">
      <svg viewBox="0 0 1000 1000" class="replay" role="img" aria-label="Positions in round {round + 1}">
        <image href={mm.icon} x="0" y="0" width="1000" height="1000" opacity="0.55" />
        {#if r?.plant?.at?.[0] !== null && r?.plant && (!k || r.plant.t <= k.t)}
          {@const p = pt(r.plant.at[0], r.plant.at[1])}
          {#if p}<rect x={p.x - 9} y={p.y - 9} width="18" height="18" transform="rotate(45 {p.x} {p.y})" class="spike" />{/if}
        {/if}
        {#if k}
          {#each k.pos as [pu, x, y] (pu)}
            {@const p = pt(x, y)}
            {@const pl = pmap.get(pu)}
            {#if p && pl}<g class="dot {pl.team === myTeam ? 'mine' : 'theirs'}"><circle cx={p.x} cy={p.y} r="13" /><text x={p.x} y={p.y + 30} text-anchor="middle">{pl.agent}</text></g>{/if}
          {/each}
          {@const v = pt(k.at[0], k.at[1])}
          {#if v}<g class="victim"><line x1={v.x - 11} y1={v.y - 11} x2={v.x + 11} y2={v.y + 11} /><line x1={v.x + 11} y1={v.y - 11} x2={v.x - 11} y2={v.y + 11} /></g>{/if}
        {/if}
      </svg>
    </div>
    <div class="col-4 stack" style="gap:10px">
      <div class="card">
        <div class="row" style="gap:8px">
          <button class="btn btn-icon btn-sm" aria-label="Previous round" onclick={() => setRound(round - 1)}><Icon name="chevron-left" /></button>
          <b style="flex:1;text-align:center">Round {round + 1} · {r?.winner === myTeam ? "won" : "lost"}</b>
          <button class="btn btn-icon btn-sm" aria-label="Next round" onclick={() => setRound(round + 1)}><Icon name="chevron-right" /></button>
        </div>
        {#if kills.length}<input type="range" min="0" max={kills.length - 1} bind:value={step} style="width:100%;margin-top:12px" />{/if}
        <div class="faint" style="font-size:12px">{r?.result}{r?.plant ? ` · planted ${r.plant.site} at ${mmss(r.plant.t)}` : ""}</div>
      </div>
      <div class="card" style="padding:8px">
        {#each kills as x, i (i)}
          {@const kl = pmap.get(x.killer)}
          {@const vc = pmap.get(x.victim)}
          <button class="list-row {i === step ? 'on' : ''}" onclick={() => (step = i)}>
            <span class="mono faint">{mmss(x.t)}</span>
            <span class={kl?.team === myTeam ? "def" : "atk"}>{kl?.name || kl?.agent || "?"}</span><span class="faint">›</span>
            <span class={vc?.team === myTeam ? "def" : "atk"}>{vc?.name || vc?.agent}</span>
            <span class="spacer"></span><span class="faint" style="font-size:11px">{x.weapon || "ability"}</span>
          </button>
        {:else}<p class="faint">No kills this round.</p>{/each}
        {#if dead.size}<div class="faint" style="font-size:11px;margin-top:6px">{dead.size} down before this kill</div>{/if}
      </div>
    </div>
  </div>
{:else}<div class="card" style="height:300px"><div class="skeleton" style="height:100%"></div></div>{/if}

<style>
  .replay { width: 100%; height: auto; display: block; }
  .dot circle { stroke: var(--bg); stroke-width: 3; transition: cx 0.25s ease, cy 0.25s ease; }
  .dot.mine circle { fill: var(--def, #4fb8b0); }
  .dot.theirs circle { fill: var(--atk, #ff5a5f); }
  .dot text { font-size: 18px; fill: var(--text); paint-order: stroke; stroke: var(--bg); stroke-width: 4px; }
  .victim line { stroke: var(--text); stroke-width: 4; }
  .spike { fill: var(--warn, #f5b041); }
  .list-row.on { background: var(--accent-soft); }
</style>
