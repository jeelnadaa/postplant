<script lang="ts">
  import { api, errText, type J } from "../lib/api";
  import { signed, tierName } from "../lib/format";
  import { app, nav } from "../lib/store.svelte";
  import Icon from "../components/Icon.svelte";
  import MatchRow from "../components/MatchRow.svelte";
  import Rank from "../components/Rank.svelte";
  import Spark from "../components/Spark.svelte";

  let d = $state<J>(null);
  let error = $state("");
  $effect(() => {
    void app.version;
    api.home().then((r) => (d = r)).catch((e) => (error = errText(e)));
  });
  const s = $derived(d?.session);
</script>

<div class="page-head">
  <div><h1 class="h1">Welcome back, {app.s.me.name}</h1><div class="sub">{d ? `${d.total} matches stored on this PC` : "Loading…"}</div></div>
</div>
{#if error}<div class="banner danger"><Icon name="alert" />{error}</div>{/if}
{#if d && d.total === 0}
  <div class="card empty">
    <Icon name="history" cls="art" /><div class="h3">No matches yet</div>
    <p>Postplant fetches your newest matches now, then checks every few minutes while VALORANT is running.</p>
    <button class="btn btn-primary" onclick={() => api.sync(20)}><Icon name="download" />Fetch my last 20 matches (about 22 requests)</button>
  </div>
{:else if d}
  <div class="grid-12">
    <div class="card col-4">
      <div class="card-head"><span class="h3">Rank</span><span class="spacer"></span><a class="link-btn" href="#/rank">Details</a></div>
      {#if d.rank?.tier}
        <Rank tier={d.rank.tier} lg />
        <div class="progress" style="--v:{d.rank.rr}%;margin:12px 0 6px"><i></i></div>
        <div class="faint" style="font-size:12px">{d.rank.rr} / 100 RR{#if d.rank.peak_tier} · peak {tierName(d.rank.peak_tier)}{#if d.rank.peak_season} ({d.rank.peak_season}){/if}{/if}</div>
      {:else}<p class="faint">Rank appears after the first sync.</p>{/if}
    </div>
    <div class="card col-8">
      <div class="card-head"><span class="h3">{s?.active ? "This session" : "Last session"}</span><span class="spacer"></span>
        {#if s}<span class="badge {s.net_rr > 0 ? 'win' : s.net_rr < 0 ? 'loss' : ''}">{signed(s.net_rr)} RR</span>{/if}</div>
      {#if s}
        <div class="row" style="gap:28px">
          <div class="stat"><div class="v"><span class="win">{s.results.filter((r: string) => r === "W").length}</span><span class="faint">–</span><span class="loss">{s.results.filter((r: string) => r === "L").length}</span></div><div class="l">{s.minutes} min played</div></div>
          <div class="stat stat-sm"><div class="v">{s.agg.acs}</div><div class="l">Avg ACS</div></div>
          <div class="stat stat-sm"><div class="v">{s.agg.kd}</div><div class="l">K/D</div></div>
          <div class="stat stat-sm"><div class="v">{s.agg.hs}%</div><div class="l">Headshot</div></div>
        </div>
        <div class="round-strip" style="margin-top:12px">{#each s.results as r, i (i)}<span class={r === "W" ? "w" : r === "L" ? "l" : "d"} style="height:8px"></span>{/each}</div>
        {#if s.tilt && s.active}
          <div class="callout warn mt-4"><b>{s.loss_streak} losses in a row.</b> Your numbers usually drop on streaks like this; a 10-minute break is the cheapest RR you'll save today. <a href="#/habits">See when you play best</a></div>
        {/if}
      {/if}
    </div>
    <div class="card col-8">
      <div class="card-head"><span class="h3">Form · ACS, last {d.form.length} competitive</span><span class="spacer"></span><span class="faint" style="font-size:12px">dashed = your average {d.season_acs}</span></div>
      <Spark values={d.form.map((f: J) => f.acs)} base={d.season_acs} />
    </div>
    <div class="card col-4">
      <div class="card-head"><span class="h3">Noticed lately</span></div>
      <div class="stack" style="gap:8px">
        {#each d.insights as n, i (i)}
          <div class="note-insight"><span class="ic"><Icon name={n.icon === "user" ? "user" : "map-pin"} /></span><div style="font-size:13px">{n.text}</div><span></span></div>
        {:else}<p class="faint" style="font-size:13px">Patterns appear after a few more competitive matches.</p>{/each}
      </div>
    </div>
    <div class="col-12">
      <div class="row mb-4"><span class="h3">Recent matches</span><span class="spacer"></span><button class="link-btn" onclick={() => nav("matches")}>All matches</button></div>
      <div class="stack" style="gap:8px">{#each d.recent as m (m.id)}<MatchRow {m} />{/each}</div>
    </div>
  </div>
{:else}
  <div class="grid-12">{#each [4, 8, 8, 4] as w, i (i)}<div class="card col-{w}" style="height:150px"><div class="skeleton" style="height:100%"></div></div>{/each}</div>
{/if}
