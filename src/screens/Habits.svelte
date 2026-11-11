<script lang="ts">
  import { api, type J } from "../lib/api";
  import { app, nav } from "../lib/store.svelte";
  import Icon from "../components/Icon.svelte";

  let d = $state<J>(null);
  $effect(() => {
    void app.version;
    api.habits().then((r) => (d = r));
  });
  const bar = (r: J) => (r.win === null ? 0 : r.win);
</script>

<div class="page-head"><div><h1 class="h1">Analytics</h1><div class="sub">Competitive · {d?.total ?? 0} matches · sessions end after 45 min without a match</div></div></div>
<div class="tabs mb-4"><button role="tab" class="tab" aria-selected="false" onclick={() => nav("analytics")}>Overview</button><button role="tab" class="tab" aria-selected="true">When you play best</button></div>
{#if d}
  {#if !d.enough}
    <div class="banner info mb-4"><Icon name="info" />Verdicts appear after about 40 competitive matches ({d.total} so far). The numbers below show sample sizes; small ones swing a lot.</div>
  {:else if d.verdict}
    <div class="note-insight mb-4"><span class="ic"><Icon name="lightbulb" /></span><div style="font-size:14px"><b>{d.verdict}</b></div><span></span></div>
  {/if}
  <div class="grid-12">
    {#each [["By match in a session", d.by_index], ["By time of day", d.by_hour], ["Party size", d.party]] as [title, rows] (title)}
      <div class="card col-4"><div class="card-head"><span class="h3">{title}</span></div>
        <div class="stack" style="gap:10px">{#each rows as r (r.label)}
          <div class="bar-h"><span>{r.label}</span><span class="b"><i style="width:{bar(r)}%;background:{r.win !== null && r.win >= 50 ? 'var(--win)' : 'var(--loss)'}"></i></span><span class="mono" style="text-align:right">{r.win ?? "—"}{r.win !== null ? "%" : ""} · {r.games}</span></div>{/each}</div>
        <p class="faint" style="font-size:12px;margin:10px 0 0">Win rate · matches</p></div>
    {/each}
    <div class="card col-6"><div class="card-head"><span class="h3">After a losing streak</span></div>
      <div class="stat-grid" style="grid-template-columns:1fr 1fr"><div class="stat stat-sm"><div class="v">{d.overall.win ?? "—"}%</div><div class="l">Normally · {d.overall.games} g</div></div>
        <div class="stat stat-sm"><div class="v {d.after_two_losses.win !== null && d.after_two_losses.win < d.overall.win ? 'loss' : ''}">{d.after_two_losses.win ?? "—"}{d.after_two_losses.win !== null ? "%" : ""}</div><div class="l">After 2+ losses · {d.after_two_losses.games} g</div></div></div></div>
    <div class="card col-6"><div class="card-head"><span class="h3">First match of the day</span></div>
      <div class="stat-grid" style="grid-template-columns:1fr 1fr"><div class="stat stat-sm"><div class="v">{d.first_of_day_acs ?? "—"}</div><div class="l">ACS · first match</div></div>
        <div class="stat stat-sm"><div class="v">{d.later_acs ?? "—"}</div><div class="l">ACS · later matches</div></div></div>
      {#if d.first_of_day_acs && d.later_acs && d.later_acs - d.first_of_day_acs >= 10}<p class="faint" style="font-size:12px;margin:10px 0 0">You start slower. A deathmatch first may help.</p>{/if}</div>
  </div>
{/if}
