<script lang="ts">
  // Where you die or get kills, per map and side, from your own stored matches (zero requests).
  import { api, errText, type J } from "../lib/api";
  import { toMinimap } from "../lib/format";
  import { app } from "../lib/store.svelte";
  import Icon from "../components/Icon.svelte";

  let map = $state("");
  let event = $state("deaths");
  let side = $state("");
  let d = $state<J>(null);
  let error = $state("");

  const maps = $derived((app.kb?.maps ?? []).filter((m: J) => m.minimap));
  $effect(() => {
    if (!map && maps.length) map = (maps.find((m: J) => m.in_pool) ?? maps[0]).name;
  });
  $effect(() => {
    void app.version;
    if (!map) return;
    error = "";
    api.heatmap(map, event, side).then((r) => (d = r)).catch((e) => (error = errText(e)));
  });
  const mm = $derived(maps.find((m: J) => m.name === map)?.minimap);
  const pts = $derived(
    (d?.points ?? [])
      .map(([x, y, won]: [number, number, boolean]) => {
        const v = toMinimap(mm, x, y);
        return v ? { x: v[0] * 1000, y: v[1] * 1000, won } : null;
      })
      .filter(Boolean) as { x: number; y: number; won: boolean }[],
  );
</script>

<div class="page-head"><div><h1 class="h1">Heatmaps</h1><div class="sub">From your own matches on this PC · zero requests</div></div></div>
<div class="row mb-4" style="gap:8px;flex-wrap:wrap">
  {#each maps as m (m.name)}<button class="chip" aria-pressed={map === m.name} onclick={() => (map = m.name)}><Icon name="map" />{m.name}</button>{/each}
</div>
<div class="row mb-4" style="gap:12px;flex-wrap:wrap">
  <div class="segmented">{#each [["deaths", "Where you die"], ["kills", "Where you get kills"]] as [v, l] (v)}<button aria-pressed={event === v} onclick={() => (event = v)}>{l}</button>{/each}</div>
  <div class="segmented">{#each [["", "Both sides"], ["atk", "Attack"], ["def", "Defense"]] as [v, l] (v)}<button aria-pressed={side === v} onclick={() => (side = v)}>{l}</button>{/each}</div>
</div>
{#if error}<div class="banner danger mb-4"><Icon name="alert" />{error}</div>{/if}
{#if mm}
  <div class="grid-12">
    <div class="card col-8" style="padding:8px">
      <svg viewBox="0 0 1000 1000" style="width:100%;height:auto;display:block" role="img" aria-label="{event} on {map}">
        <defs>
          <radialGradient id="hm-dot"><stop offset="0%" stop-color={event === "deaths" ? "var(--loss)" : "var(--win)"} stop-opacity="0.55" /><stop offset="100%" stop-color={event === "deaths" ? "var(--loss)" : "var(--win)"} stop-opacity="0" /></radialGradient>
        </defs>
        <image href={mm.icon} x="0" y="0" width="1000" height="1000" opacity="0.6" />
        {#each pts as p, i (i)}<circle cx={p.x} cy={p.y} r="34" fill="url(#hm-dot)" />{/each}
        {#each pts as p, i (i)}<circle cx={p.x} cy={p.y} r="4" class={p.won ? "hm-w" : "hm-l"} />{/each}
      </svg>
    </div>
    <div class="col-4 stack" style="gap:10px">
      <div class="stat-grid" style="grid-template-columns:1fr 1fr">
        <div class="stat"><div class="v">{pts.length}</div><div class="l">{event === "deaths" ? "Deaths" : "Kills"}</div></div>
        <div class="stat"><div class="v">{d?.matches ?? 0}</div><div class="l">Matches on {map}</div></div>
      </div>
      <div class="card"><p class="faint" style="margin:0;font-size:13px">Bright areas are where it happens most. Small dots are filled for rounds your team won and hollow for rounds it lost. Use it to spot spots you keep dying in and try a different angle.</p></div>
    </div>
  </div>
{:else if !maps.length}<div class="card empty-inline">Map data is still loading.</div>{/if}

<style>
  .hm-w { fill: var(--text); }
  .hm-l { fill: none; stroke: var(--text); stroke-width: 1.5; }
</style>
