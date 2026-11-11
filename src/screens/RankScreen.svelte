<script lang="ts">
  import { api, type J } from "../lib/api";
  import { fmtDate, signed, tierName } from "../lib/format";
  import { app, nav } from "../lib/store.svelte";
  import Agent from "../components/Agent.svelte";
  import Rank from "../components/Rank.svelte";
  import Spark from "../components/Spark.svelte";

  let d = $state<J>(null);
  $effect(() => {
    void app.version;
    api.rank().then((r) => (d = r));
  });
  const elo = $derived((d?.snapshots ?? []).map((s: J) => s.elo).filter((x: number) => x > 0));
</script>

<div class="page-head"><div><h1 class="h1">Rank</h1><div class="sub">From your own snapshots: one after each new match</div></div></div>
{#if d}
  <div class="grid-12">
    <div class="card col-4">
      {#if d.now?.tier}
        <Rank tier={d.now.tier} lg />
        <div class="progress" style="--v:{d.now.rr}%;margin:14px 0 6px"><i></i></div>
        <div class="kv mt-2">
          <span>RR</span><b>{d.now.rr} / 100</b>
          <span>Last change</span><b class={d.now.last_change > 0 ? "win" : d.now.last_change < 0 ? "loss" : ""}>{signed(d.now.last_change)}</b>
          <span>Peak</span><b>{tierName(d.now.peak_tier)}{d.now.peak_season ? ` · ${d.now.peak_season}` : ""}</b>
          <span>Average win</span><b>{d.avg_gain ? `+${d.avg_gain} RR` : "—"}</b>
          <span>To next rank</span><b>{d.wins_to_next ? `~${d.wins_to_next} wins` : "—"}</b>
        </div>
      {:else}<p class="faint">No rank yet. It appears after your first sync.</p>{/if}
    </div>
    <div class="card col-8"><div class="card-head"><span class="h3">Rating over time</span><span class="spacer"></span><span class="faint" style="font-size:12px">{elo.length} snapshots</span></div>
      {#if elo.length > 1}<Spark values={elo} h={150} />{:else}<p class="faint">The graph fills in as you play.</p>{/if}</div>
    {#if d.seasons.length}
      <div class="card col-5"><div class="card-head"><span class="h3">Acts</span></div>
        <table class="table"><thead><tr><th>Act</th><th>Final rank</th><th class="num">Wins</th><th class="num">Games</th></tr></thead><tbody>
          {#each d.seasons.slice(-8).reverse() as s (s.season)}<tr><td class="mono">{s.season}</td><td><Rank tier={s.end_tier} /></td><td class="num">{s.wins}</td><td class="num">{s.games}</td></tr>{/each}
        </tbody></table></div>
    {/if}
    <div class="card {d.seasons.length ? 'col-7' : 'col-12'}"><div class="card-head"><span class="h3">RR per match</span></div>
      <table class="table"><tbody>
        {#each d.changes.slice(0, 15) as m (m.id)}
          <tr class="is-link" onclick={() => nav("match", { id: m.id })}><td><div class="player"><Agent name={m.agent} id={m.agent_id} sm /><b>{m.map}</b></div></td>
            <td class={m.result === "W" ? "win" : "loss"}>{m.score[0]}–{m.score[1]}</td><td class="faint">{fmtDate(m.started_at)}</td>
            <td class="num"><span class="badge {m.rr > 0 ? 'win' : m.rr < 0 ? 'loss' : ''}">{signed(m.rr)}</span></td></tr>
        {:else}<tr><td class="empty-inline">RR changes appear after the next competitive match.</td></tr>{/each}
      </tbody></table></div>
  </div>
{/if}
