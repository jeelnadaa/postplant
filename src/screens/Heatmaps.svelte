<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  // Heatmaps (plan 16): layers me / my team / enemies; events kills, deaths, opening duels, where
  // rounds end, plants, defuses; filters for side, agent, round type and won/lost rounds.
  // From your own stored matches only: zero requests.
  import { api, errText, type J } from "../lib/api";
  import { toMinimap } from "../lib/format";
  import { app } from "../lib/store.svelte";
  import Icon from "../components/Icon.svelte";
  import Select from "../components/Select.svelte";
  import ExportImage from "../components/ExportImage.svelte";

  // With matchId and fixedMap set, this shows one match (embedded in the match page).
  let { matchId = "", fixedMap = "" }: { matchId?: string; fixedMap?: string } = $props();
  let map = $state("");
  let event = $state("deaths");
  let side = $state("");
  let layer = $state("me");
  let agent = $state("");
  let outcome = $state("");
  let rtype = $state("");
  let queue = $state("");
  let days = $state(0);
  let svgEl = $state<SVGSVGElement>();
  let d = $state<J>(null);
  let error = $state("");

  const EVENTS: [string, string][] = [["deaths", "Deaths"], ["kills", "Kills"], ["opening", "Opening duels"], ["round_end", "Where rounds end"], ["plants", "Spike plants"], ["defuses", "Defuses"]];
  const maps = $derived((app.kb?.maps ?? []).filter((m: J) => m.minimap));
  $effect(() => {
    if (fixedMap) map = fixedMap;
    else if (!map && maps.length) map = (maps.find((m: J) => m.in_pool) ?? maps[0]).name;
  });
  $effect(() => {
    void app.version;
    if (!map) return;
    error = "";
    api.heatmap({ map, event, side, layer, agent, outcome, rtype, matchId, queue: matchId ? "" : queue, days: matchId ? 0 : days }).then((r) => (d = r)).catch((e) => (error = errText(e)));
  });
  const mm = $derived(maps.find((m: J) => m.name === map)?.minimap);
  const pts = $derived(
    (d?.points ?? [])
      .map(([x, y, good]: [number, number, boolean]) => {
        const v = toMinimap(mm, x, y);
        return v ? { x: v[0] * 1000, y: v[1] * 1000, good } : null;
      })
      .filter(Boolean) as { x: number; y: number; good: boolean }[],
  );
  const goodLabel = $derived(event === "opening" ? "duel won" : "round won");
  const color = $derived(event === "deaths" ? "var(--loss)" : event === "kills" || event === "opening" ? "var(--win)" : "var(--accent)");
</script>

{#if !matchId}
<div class="page-head"><div><h1 class="h1">{t("Heatmaps")}</h1><div class="sub">{t("From matches you played, stored on this PC · zero requests")}</div></div></div>
<div class="row mb-4" style="gap:8px;flex-wrap:wrap">
  {#each maps as m (m.name)}<button class="chip" aria-pressed={map === m.name} onclick={() => (map = m.name)}><Icon name="map" />{m.name}</button>{/each}
</div>
{/if}
<div class="row mb-2" style="gap:12px;flex-wrap:wrap">
  <div class="segmented">{#each EVENTS as [v, l] (v)}<button aria-pressed={event === v} onclick={() => (event = v)}>{t(l)}</button>{/each}</div>
</div>
<div class="row mb-4" style="gap:12px;flex-wrap:wrap">
  {#if event !== "round_end"}<div class="segmented">{#each [["me", "Me"], ["team", "My team"], ["enemy", "Enemies"]] as [v, l] (v)}<button aria-pressed={layer === v} onclick={() => (layer = v)}>{t(l)}</button>{/each}</div>{/if}
  <div class="segmented">{#each [["", "Both sides"], ["atk", "Attack"], ["def", "Defense"]] as [v, l] (v)}<button aria-pressed={side === v} onclick={() => (side = v)}>{t(l)}</button>{/each}</div>
  <Select value={rtype} options={[["", "All rounds"], ["pistol", "Pistol"], ["eco", "Your eco"], ["force", "Your force buy"], ["full", "Your full buy"]]} onchange={(v) => (rtype = v)} />
  <Select value={outcome} options={[["", "Won and lost"], ["won", "Rounds won"], ["lost", "Rounds lost"]]} onchange={(v) => (outcome = v)} />
  {#if event !== "round_end"}<Select value={agent} options={[["", "Any agent"], ...((app.kb?.agents ?? []).map((a: J) => [a.name, a.name]) as [string, string][])]} onchange={(v) => (agent = v)} />{/if}
  {#if !matchId}
    <Select value={queue} icon="swords" options={[["", "All modes"], ["competitive", "Competitive"], ["unrated", "Unrated"], ["swiftplay", "Swiftplay"], ["premier", "Premier"]]} onchange={(v) => (queue = v)} />
    <Select value={String(days)} icon="clock" options={[["0", "All time"], ["7", "Last 7 days"], ["30", "Last 30 days"], ["90", "Last 90 days"]]} onchange={(v) => (days = Number(v))} />
  {/if}
  <span class="spacer"></span>
  <ExportImage svg={svgEl} title={`${map} · ${EVENTS.find((e) => e[0] === event)?.[1]} · ${layer === "me" ? "me" : layer === "team" ? "my team" : "enemies"}${matchId ? " · one match" : ` · ${d?.matches ?? 0} matches`}`} name={`postplant-heatmap-${map}-${event}`} />
</div>
{#if error}<div class="banner danger mb-4"><Icon name="alert" />{error}</div>{/if}
{#if mm}
  <div class="grid-12">
    <div class="card col-8" style="padding:8px">
      <svg bind:this={svgEl} viewBox="0 0 1000 1000" style="width:100%;height:auto;display:block" role="img" aria-label="{event} on {map}">
        <defs>
          <radialGradient id="hm-dot"><stop offset="0%" stop-color={color} stop-opacity="0.5" /><stop offset="100%" stop-color={color} stop-opacity="0" /></radialGradient>
        </defs>
        <image href={mm.icon} x="0" y="0" width="1000" height="1000" opacity="0.6" />
        {#each pts as p, i (i)}<circle cx={p.x} cy={p.y} r="34" fill="url(#hm-dot)" />{/each}
        {#each pts as p, i (i)}<circle cx={p.x} cy={p.y} r="4" class={p.good ? "hm-w" : "hm-l"} />{/each}
      </svg>
    </div>
    <div class="col-4 stack" style="gap:10px">
      <div class="stat-grid" style="grid-template-columns:1fr 1fr">
        <div class="stat"><div class="v">{pts.length}</div><div class="l">{EVENTS.find((e) => e[0] === event)?.[1]}</div></div>
        {#if matchId}<div class="stat"><div class="v">{d?.spots?.[0]?.name ?? "–"}</div><div class="l">{t("Busiest spot")}</div></div>
        {:else}<div class="stat"><div class="v">{d?.matches ?? 0}</div><div class="l">{t("Matches on")} {map}</div></div>{/if}
      </div>
      <div class="card">
        <div class="card-head"><span class="h3">{t("Hot spots")}</span><span class="spacer"></span><span class="faint" style="font-size:11px">{goodLabel} %</span></div>
        {#each d?.spots ?? [] as sp (sp.name)}
          <div class="bar-h"><span>{sp.name}</span><span class="b"><i style="width:{Math.round((100 * sp.n) / Math.max(1, pts.length))}%"></i></span><span class="mono" style="text-align:right">{sp.n} · {sp.pct}%</span></div>
        {:else}<p class="faint" style="margin:0">{t("Nothing here yet with these filters.")}</p>{/each}
      </div>
      <p class="faint" style="font-size:12px;margin:0">{t("Small dots are filled when the")} {goodLabel === "duel won" ? "duel was won" : "round was won"} {t("and hollow when it wasn't. Positions are only recorded at kills, plants and defuses.")}</p>
    </div>
  </div>
{:else if !maps.length}<div class="card empty-inline">{t("Map data is still loading.")}</div>{/if}

<style>
  .hm-w { fill: var(--text); }
  .hm-l { fill: none; stroke: var(--text); stroke-width: 1.5; }
</style>
