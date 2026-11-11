<script lang="ts">
  import { api, errText, type J } from "../lib/api";
  import { fmtDate, hms, mmss, queueLabel, signed } from "../lib/format";
  import { nav, toast } from "../lib/store.svelte";
  import Agent from "../components/Agent.svelte";
  import Icon from "../components/Icon.svelte";
  import NoteModal from "../components/NoteModal.svelte";
  import PlayerName from "../components/PlayerName.svelte";
  import Rank from "../components/Rank.svelte";
  import Replay from "../components/Replay.svelte";

  let { id }: { id: string } = $props();
  let d = $state<J>(null);
  let error = $state("");
  let tab = $state("score");
  let noteFor = $state<J>(null);
  let noteOpen = $state(false);

  async function load() {
    try {
      d = await api.match(id);
    } catch (e) {
      error = errText(e);
    }
  }
  $effect(() => {
    load();
  });
  // Current and peak rank of everyone named in the match (cached 6 h; hidden names are never looked up).
  let ranks = $state<J>({});
  let ranksBusy = $state(false);
  $effect(() => {
    void id;
    ranksBusy = true;
    api.matchRanks(id).then((r) => (ranks = r)).catch(() => {}).finally(() => (ranksBusy = false));
  });

  const mine = $derived(d?.teams?.find((t: J) => t.mine));
  const theirs = $derived(d?.teams?.find((t: J) => !t.mine));
  const result = $derived(!d ? "" : mine?.won > theirs?.won ? "Victory" : mine?.won < theirs?.won ? "Defeat" : "Draw");
  const team = (mineTeam: boolean) => (d?.players ?? []).filter((p: J) => (p.team === d.my_team) === mineTeam);

  async function watch(t: number | null | undefined) {
    if (t === null || t === undefined) return toast("warn", "No timestamp for this moment");
    try {
      const r = await api.watch(id, t);
      if (r.how === "default") {
        await navigator.clipboard.writeText(hms(r.seconds)).catch(() => {});
        toast("info", `Opened the recording; jump to ${hms(r.seconds)}`, "Your player doesn't accept a start time, so the time was copied. VLC and mpv jump there automatically.");
      }
    } catch (e) {
      toast("err", "Couldn't open the recording", errText(e));
    }
  }
  async function review(kind: string, round: number | null, t: number | null) {
    try {
      await api.addReview(id, kind, round, t);
      toast("ok", "Added to your review list");
      load();
    } catch (e) {
      toast("err", "Couldn't add", errText(e));
    }
  }
  const reviewed = (round: number | null, kind: string) => d?.reviews?.some((r: J) => r.kind === kind && (r.round ?? null) === round);
</script>

<div class="row mb-4"><button class="btn btn-ghost btn-sm" onclick={() => history.back()}><Icon name="arrow-left" />Back</button></div>
{#if error}<div class="banner danger"><Icon name="alert" />{error}</div>{/if}
{#if d}
  <div class="card mb-4" style="display:flex;align-items:center;gap:24px;background:linear-gradient(100deg,color-mix(in srgb,var({result === 'Victory' ? '--win' : '--loss'}) 14%,transparent),transparent 40%),var(--surface)">
    <span class="display {result === 'Victory' ? 'win' : result === 'Defeat' ? 'loss' : ''}" style="font-size:64px">{mine?.won}–{theirs?.won}</span>
    <div><div class="h2">{result} · {d.map}</div><div class="muted" style="font-size:13px">{queueLabel(d.queue?.toLowerCase?.() ?? "")} · {fmtDate(d.started_at)} · {Math.round(d.length_ms / 60000)} min{#if d.version} · {d.version}{/if}</div></div>
    <span class="spacer"></span>
    {#if d.rr !== null && d.rr !== undefined}<div class="stat"><div class="v {d.rr > 0 ? 'win' : 'loss'}">{signed(d.rr)}</div><div class="l">RR</div></div>{/if}
    <button class="btn btn-sm" disabled={reviewed(null, "match")} onclick={() => review("match", null, null)}><Icon name="bookmark" />{reviewed(null, "match") ? "In review list" : "Review later"}</button>
  </div>
  {#if d.recording}<div class="banner info mb-4"><Icon name="video" />Recording found: <b>{d.recording.name}</b>. Every round and death below has a Watch button.</div>{/if}
  <div class="tabs mb-4">
    <button role="tab" class="tab" aria-selected={tab === "score"} onclick={() => (tab = "score")}>Scoreboard</button>
    <button role="tab" class="tab" aria-selected={tab === "rounds"} onclick={() => (tab = "rounds")}>Rounds<span class="count">{d.rounds.length}</span></button>
    <button role="tab" class="tab" aria-selected={tab === "deaths"} onclick={() => (tab = "deaths")}>Your deaths<span class="count">{d.deaths.length}</span></button>
    <button role="tab" class="tab" aria-selected={tab === "replay"} onclick={() => (tab = "replay")}>2D replay</button>
  </div>

  {#if tab === "score"}
    <table class="table">
      <thead><tr><th>Player</th><th>Rank</th><th>Peak</th><th>Now</th><th class="num sorted">ACS ▾</th><th class="num">K</th><th class="num">D</th><th class="num">A</th><th class="num">HS%</th><th class="num">ADR</th><th class="num">FB</th><th></th></tr></thead>
      <tbody>
        {#each [true, false] as mineTeam (mineTeam)}
          <tr class="team-head"><td colspan="12"><span class={mineTeam ? "def" : "atk"}>● {mineTeam ? "Your team" : "Enemy team"}</span>
            <span class="badge {mineTeam === (result === 'Victory') ? 'win' : 'loss'}" style="margin-left:6px">{mineTeam ? mine?.won : theirs?.won}</span></td></tr>
          {#each team(mineTeam) as p (p.puuid)}
            <tr class={p.me ? "is-me" : ""}>
              <td><div class="player-cell"><Agent name={p.agent} id={p.agent_id} sm />
                <div class="who" style="display:flex;align-items:center;gap:6px;min-width:0">
                  {#if p.me}<PlayerName name={p.name} tag={p.tag} agent={p.agent} /><span class="badge accent">you</span>
                  {:else}<button class="row" style="gap:4px" title="Look up this player" onclick={() => p.name && nav("search", { name: p.name, tag: p.tag })}><PlayerName name={p.name} tag={p.tag} agent={p.agent} /></button>{/if}
                  {#if p.party}<span class="tag good">party</span>{/if}
                  {#if p.note?.tags?.length}{#each p.note.tags as t (t)}<span class="tag {t === 'toxic' || t === 'thrower' ? 'bad' : t.startsWith('good') ? 'good' : 'warnish'}">{t}</span>{/each}{/if}
                  {#if p.met > 0}<span class="faint" style="font-size:11px">met {p.met}×</span>{/if}
                </div></div></td>
              <td><Rank tier={p.tier} /></td>
              {#if ranks[p.puuid]?.state === "ok" || ranks[p.puuid]?.state === "unranked"}
                <td><Rank tier={ranks[p.puuid].peak_tier} short />{#if ranks[p.puuid].peak_season}<span class="faint" style="font-size:11px"> {ranks[p.puuid].peak_season}</span>{/if}</td>
                <td><Rank tier={ranks[p.puuid].tier} rr={ranks[p.puuid].tier >= 3 ? ranks[p.puuid].rr : null} short /></td>
              {:else if ranks[p.puuid]?.state === "private"}<td colspan="2" class="faint"><Icon name="lock" /> private</td>
              {:else if !p.name}<td colspan="2" class="faint">hidden</td>
              {:else}<td colspan="2" class="faint">{ranksBusy ? "…" : "—"}</td>{/if}
              <td class="num"><b>{p.acs}</b></td><td class="num">{p.k}</td><td class="num">{p.d}</td><td class="num">{p.a}</td>
              <td class="num">{p.hs}%</td><td class="num">{p.adr}</td><td class="num">{p.fb}</td>
              <td>{#if !p.me}<button class="btn btn-icon btn-ghost btn-sm" title="Note" aria-label="Note" onclick={() => { noteFor = p; noteOpen = true; }}><Icon name="note" /></button>{/if}</td>
            </tr>
          {/each}
        {/each}
      </tbody>
    </table>
    <p class="notice mt-2">Rank = rank at match time. Peak and Now are looked up per player (cached 6 hours). Players who hid their name aren't looked up.</p>
  {:else if tab === "rounds"}
    <div class="round-strip mb-4">
      {#each d.rounds as r (r.n)}<span class={r.won ? "w" : "l"} title="Round {r.n + 1}: {r.result}">{r.n + 1}</span>{#if r.n === 11}<span class="half"></span>{/if}{/each}
    </div>
    <table class="table">
      <thead><tr><th>Round</th><th>Side</th><th>Result</th><th class="num">Your kills</th><th>You</th><th class="num">Your loadout</th><th></th></tr></thead>
      <tbody>
        {#each d.rounds as r (r.n)}
          <tr>
            <td><b class={r.won ? "win" : "loss"}>{r.n + 1}</b></td>
            <td><span class={r.side}>{r.side === "atk" ? "Attack" : "Defense"}</span></td>
            <td>{r.won ? "Won" : "Lost"} · {r.result}{#if r.plant} · planted {r.plant}{/if}</td>
            <td class="num">{r.my_kills}</td>
            <td>{r.died ? "died" : "survived"}</td>
            <td class="num">{r.loadout ?? "—"}</td>
            <td><div class="row" style="justify-content:flex-end;gap:6px">
              {#if d.recording && r.start_ms !== null && r.start_ms !== undefined}<button class="btn btn-sm" onclick={() => watch(r.start_ms)}><Icon name="video" />Watch</button>{/if}
              <button class="btn btn-icon btn-ghost btn-sm" title="Review later" aria-label="Review later" disabled={reviewed(r.n, "round")} onclick={() => review("round", r.n, r.start_ms ?? null)}><Icon name="bookmark" /></button>
            </div></td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else if tab === "replay"}
    <Replay {id} myTeam={d.my_team} />
  {:else}
    <div class="stack" style="gap:10px">
      {#each d.deaths as x (x.time_in_match_ms)}
        <div class="card" style="padding:14px">
          <div class="card-head" style="margin-bottom:8px"><Icon name="crosshair" /><span class="h3">Round {x.round + 1} · {mmss(x.time_in_round_ms)}</span><span class="spacer"></span>
            <span class="badge {x.traded ? 'win' : 'loss'}">{x.traded ? "Traded" : "Not traded"}</span></div>
          <div class="stat-grid" style="grid-template-columns:repeat(4,1fr)">
            <div class="stat stat-sm"><div class="v" style="font-size:18px">{x.killer || x.killer_agent}</div><div class="l">Killed by ({x.killer_agent})</div></div>
            <div class="stat stat-sm"><div class="v" style="font-size:18px">{x.weapon || "Ability"}</div><div class="l">Weapon</div></div>
            <div class="stat stat-sm"><div class="v" style="font-size:18px">{x.nearest_ally_m !== null ? `${x.nearest_ally_m} m` : "—"}</div><div class="l">Nearest teammate</div></div>
            <div class="stat stat-sm"><div class="v" style="font-size:18px">{x.my_loadout} v {x.their_loadout}</div><div class="l">Loadout (you v them)</div></div>
          </div>
          <div class="row mt-4" style="gap:8px">
            {#if d.recording}<button class="btn btn-sm btn-primary" onclick={() => watch(x.time_in_match_ms)}><Icon name="video" />Watch at {hms(Math.max(0, (d.recording.offset_ms + x.time_in_match_ms) / 1000 - 5))}</button>{/if}
            <button class="btn btn-sm" disabled={reviewed(x.round, "death")} onclick={() => review("death", x.round, x.time_in_match_ms)}><Icon name="bookmark" />{reviewed(x.round, "death") ? "In review list" : "Review later"}</button>
          </div>
        </div>
      {:else}<div class="card empty-inline">You didn't die in this match. Nice.</div>{/each}
    </div>
  {/if}
{:else if !error}
  <div class="card" style="height:200px"><div class="skeleton" style="height:100%"></div></div>
{/if}

{#if noteFor}
  <NoteModal bind:open={noteOpen} puuid={noteFor.puuid} who={noteFor.name ? `${noteFor.name}#${noteFor.tag}` : noteFor.agent} note={noteFor.note} onsaved={load} />
{/if}
