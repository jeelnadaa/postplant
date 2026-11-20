<script lang="ts">
  import type { J } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  let { rows, modes = false }: { rows: J[]; modes?: boolean } = $props();
  let agent = $state("");
  let map = $state("");
  let side = $state("all");
  let queue = $state("");
  const options = (field: string) => [...new Set<string>(rows.map(r => r[field]).filter(Boolean))].sort();
  const filtered = $derived(rows.filter(r => (!agent || agent === r.agent) && (!map || map === r.map) && (!queue || queue === r.queue)));
  const entries = $derived.by(() => {
    const groups = new Map<string, {agent:string; map:string; games:number; covered:number; kills:number}>();
    for (const r of filtered) {
      const key = JSON.stringify([r.agent,r.map]);
      const item = groups.get(key) ?? {agent:r.agent,map:r.map,games:0,covered:0,kills:0};
      item.games += r.games;
      item.covered += side === "all" ? r.games : (r.side_games ?? 0);
      item.kills += side === "all" ? r.k : (r[side] ?? 0);
      groups.set(key,item);
    }
    return [...groups.values()].sort((a,b) => b.kills - a.kills || a.agent.localeCompare(b.agent));
  });
  const games = $derived(entries.reduce((n,r) => n+r.games,0));
  const covered = $derived(entries.reduce((n,r) => n+r.covered,0));
  const kills = $derived(entries.reduce((n,r) => n+r.kills,0));
</script>

<section class="card mb-4">
  <div class="card-head"><h3 class="h3">{t("Kill breakdown")}</h3></div>
  <div class="row mb-2" style="gap:8px;flex-wrap:wrap">
    {#if modes}<select class="input" aria-label={t("Mode")} bind:value={queue}><option value="">{t("All modes")}</option>{#each options("queue") as q}<option value={q}>{q}</option>{/each}</select>{/if}
    <select class="input" aria-label={t("Agent")} bind:value={agent}><option value="">{t("All agents")}</option>{#each options("agent") as a}<option value={a}>{a}</option>{/each}</select>
    <select class="input" aria-label={t("Map")} bind:value={map}><option value="">{t("All maps")}</option>{#each options("map") as m}<option value={m}>{m}</option>{/each}</select>
    <select class="input" aria-label={t("Side")} bind:value={side}><option value="all">{t("Both sides")}</option><option value="atk_k">{t("Attack")}</option><option value="def_k">{t("Defense")}</option></select>
    {#if agent || map || queue || side !== "all"}<button class="btn btn-sm" onclick={() => {agent="";map="";queue="";side="all";}}>{t("Clear filters")}</button>{/if}
  </div>
  <p aria-live="polite"><b>{covered ? kills.toLocaleString() : "—"} {t("kills")}</b> · {covered} / {games} {t("matching games with data")}</p>
  {#if games}
    <table class="table"><thead><tr><th>{t("Agent")}</th><th>{t("Map")}</th><th class="num">{t("Games with data")}</th><th class="num">{t("Kills")}</th></tr></thead><tbody>
      {#each entries as r}<tr><td>{r.agent}</td><td>{r.map}</td><td class="num">{r.covered} / {r.games}</td><td class="num">{r.covered ? r.kills.toLocaleString() : "—"}</td></tr>{/each}
    </tbody></table>
  {:else}<p class="faint">{t("No matching game records.")}</p>{/if}
</section>
<style>select.input { width:auto; max-width:100%; }</style>
