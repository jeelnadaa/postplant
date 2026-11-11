<script lang="ts">
  // The in-game overlay. Everything here is drawn from local data; no requests are made from this window.
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api, on, type J } from "./lib/api";
  import { applyPrefs } from "./lib/store.svelte";
  import { kd, signed, tierName } from "./lib/format";
  import Agent from "./components/Agent.svelte";
  import Icon from "./components/Icon.svelte";

  const PANELS = ["Last match", "Session", "Rank"];
  let d = $state<J>(null);
  let panel = $state(0);
  let board = $state(false);
  let edit = $state(false);
  let entering = $state(true);
  let boardTimer: ReturnType<typeof setTimeout> | undefined;

  async function load() {
    d = await api.overlayData().catch(() => null);
    if (d?.prefs) {
      applyPrefs(d.prefs, true);
      panel = d.prefs.overlay?.panel ?? panel;
    }
    entering = true;
    setTimeout(() => (entering = false), 300);
  }
  load();

  on("overlay-cycle", () => {
    board = false;
    panel = (panel + 1) % PANELS.length;
    api.savePrefs({ overlay: { panel } }).catch(() => {});
  });
  on("overlay-edit", (p: J) => (edit = !!p.edit));
  on("prefs", (p: J) => {
    if (d) d.prefs = p;
    applyPrefs(p, true);
  });
  on("sync-state", (p: J) => {
    if (p.new > 0) load();
  });
  on("new-match", async () => {
    await load();
    if (d?.prefs?.overlay?.auto_post && d.board) {
      board = true;
      clearTimeout(boardTimer);
      boardTimer = setTimeout(() => (board = false), (d.prefs.overlay.post_seconds ?? 20) * 1000);
    }
  });
  on("reset", () => location.reload());

  const ov = $derived(d?.prefs?.overlay ?? {});
  // The glass slider changes only the background (75–100%); text stays fully opaque (plan 24.1).
  const glass = $derived(Math.min(100, Math.max(75, ov.opacity ?? 88)));
  const last = $derived(d?.home?.last);
  const s = $derived(d?.home?.session);
  const rank = $derived(d?.home?.rank);
  const team = (mine: boolean) => (d?.board?.players ?? []).filter((p: J) => (p.team === d.board.my_team) === mine);
  const score = $derived.by(() => {
    const t = d?.board?.teams ?? [];
    const m = t.find((x: J) => x.mine), o = t.find((x: J) => !x.mine);
    return [m?.won ?? 0, o?.won ?? 0];
  });
  function drag(e: MouseEvent) {
    if (edit && e.button === 0 && !(e.target as HTMLElement).closest("button")) getCurrentWindow().startDragging();
  }
</script>

{#if d}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="ov {board ? 'ov-board' : ''} {edit ? 'edit' : ''} {entering ? 'is-entering' : ''}" data-glass-low={glass < 88 ? "" : undefined} style="background:color-mix(in srgb, var(--bg) {glass}%, transparent);zoom:{(ov.scale ?? 100) / 100}" onmousedown={drag}>
    <div class="ov-head">
      <span class="logo"><svg style="color:var(--accent)"><use href="#logo" /></svg></span>
      <b style="font-size:12px">{board ? "Match result" : PANELS[panel]}</b>
      {#if edit}<button class="btn btn-sm" style="margin-left:auto" onclick={() => api.overlay("edit_off")}><Icon name="check" />Done</button>
      {:else}<span class="ov-dots">{#each PANELS as _, i (i)}<i class={!board && i === panel ? "on" : ""}></i>{/each}</span>{/if}
    </div>

    {#if board && d.board}
      <div class="row" style="gap:8px;margin-bottom:6px"><span class="big {score[0] > score[1] ? 'win' : score[0] < score[1] ? 'loss' : ''}">{score[0]}–{score[1]}</span>
        <span class="faint" style="font-size:11px">{d.board.map}{#if d.board.rr !== null && d.board.rr !== undefined} · {signed(d.board.rr)} RR{/if}</span></div>
      {#each [true, false] as mine (mine)}
        <div class="caps faint" style="font-size:10px;margin:6px 0 2px">{mine ? "Your team" : "Enemy team"}</div>
        {#each team(mine) as p, i (p.puuid)}
          <div class="r lr {p.me ? 'me' : ''}" style="--i:{i}">
            <Agent name={p.agent} id={p.agent_id} sm />
            <span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{p.name || p.agent}{#if p.note?.tags?.length} <span class="faint">· {p.note.tags[0]}</span>{/if}</span>
            <span class="mono">{p.acs}</span><span class="mono faint">{p.k}/{p.d}/{p.a}</span>
          </div>
        {/each}
      {/each}
    {:else if panel === 0}
      {#if last}
        <div class="row" style="gap:10px"><Agent name={last.agent} id={last.agent_id} sm />
          <div><span class="big {last.result === 'W' ? 'win' : last.result === 'L' ? 'loss' : ''}">{last.score[0]}–{last.score[1]}</span><div class="faint" style="font-size:11px">{last.map}</div></div>
          {#if last.rr !== null && last.rr !== undefined}<span class="badge {last.rr > 0 ? 'win' : 'loss'}" style="margin-left:auto">{signed(last.rr)}</span>{/if}</div>
        <div class="ov-grid"><div><b>{last.k}/{last.d}/{last.a}</b><small>KDA</small></div><div><b>{last.acs}</b><small>ACS</small></div><div><b>{last.hs}%</b><small>HS</small></div></div>
      {:else}<div class="faint">No matches yet.</div>{/if}
    {:else if panel === 1}
      {#if s}
        <div class="row" style="gap:6px;flex-wrap:wrap">{#each s.results as r, i (i)}<span class="badge {r === 'W' ? 'win' : r === 'L' ? 'loss' : ''}">{r}</span>{/each}</div>
        <div class="ov-grid"><div><b class={s.net_rr > 0 ? "win" : s.net_rr < 0 ? "loss" : ""}>{signed(s.net_rr)}</b><small>Net RR</small></div>
          <div><b>{s.agg.acs}</b><small>ACS</small></div><div><b>{kd(s.agg.k, s.agg.d)}</b><small>K/D</small></div></div>
        {#if s.active && s.tilt}<div class="tilt"><Icon name="coffee" /><span>{s.loss_streak} losses in a row. A short break usually helps.</span></div>{/if}
      {:else}<div class="faint">No session yet.</div>{/if}
    {:else}
      {#if rank?.tier}
        <div class="big">{tierName(rank.tier)}</div>
        <div class="progress" style="--v:{rank.rr}%;margin:10px 0 6px"><i></i></div>
        <div class="faint" style="font-size:11px">{rank.rr} / 100 RR{#if rank.last_change} · last {signed(rank.last_change)}{/if}</div>
      {:else}<div class="faint">Rank appears after the first sync.</div>{/if}
    {/if}
  </div>
{/if}

<svg width="0" height="0" style="position:absolute" aria-hidden="true">
  <symbol id="logo" viewBox="0 0 48 48">
    <path d="M29.05 5.16A19.5 19.5 0 1 1 7.11 14.25" fill="none" stroke="currentColor" stroke-width="3.6" stroke-linecap="round" />
    <circle cx="16.54" cy="5.98" r="2.6" fill="currentColor" />
    <path d="M24 14.5 33.5 24 24 33.5 14.5 24Z" fill="currentColor" />
  </symbol>
</svg>
