<script lang="ts">
  import { t } from "./lib/i18n.svelte";
  // The in-game overlay. Panels draw from local data; the live views show what the Rust side already
  // fetched. Agent select only ever shows the advisor (your own planning), never other players.
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api, on, type J } from "./lib/api";
  import { app, applyPrefs } from "./lib/store.svelte";
  import { kd, signed, tierName } from "./lib/format";
  import Agent from "./components/Agent.svelte";
  import Icon from "./components/Icon.svelte";
  import Rank from "./components/Rank.svelte";

  const PANELS = ["Last match", "Session", "Rank"];
  let d = $state<J>(null);
  let panel = $state(0);
  let board = $state(false);
  let edit = $state(false);
  let entering = $state(true);
  let layout = $state("normal");
  let boardTimer: ReturnType<typeof setTimeout> | undefined;
  let liveTimer: ReturnType<typeof setTimeout> | undefined;

  // Advisor
  let advMap = $state("");
  let advTeam = $state<string[]>([]);
  let advRes = $state<J>(null);
  let advAuto = $state(false);
  let picking = $state(false);

  // Live
  let live = $state<J>(null);
  let liveMode = $state("none");

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
  api.kb().then((k) => (app.kb = k)).catch(() => {});
  api.liveStatus().then((s: J) => (live = s?.roster ?? null)).catch(() => {});

  on("overlay-cycle", () => {
    board = false;
    panel = (panel + 1) % PANELS.length;
    api.savePrefs({ overlay: { panel } }).catch(() => {});
  });
  on("overlay-edit", (p: J) => (edit = !!p.edit));
  on("overlay-layout", (p: J) => (layout = p.kind));
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
      boardTimer = setTimeout(() => {
        board = false;
        api.overlay("post_done").catch(() => {});
      }, (d.prefs.overlay.post_seconds ?? 20) * 1000);
    }
  });
  on("advisor-open", (p: J) => {
    advAuto = !!p.auto;
    if (p.map) advMap = p.map;
    advTeam = [];
    picking = false;
  });
  on("advisor-close", () => (picking = false));
  on("live", (r: J) => {
    live = r;
    if (!r) liveMode = "none";
  });
  on("live-show", (p: J) => {
    liveMode = p.mode;
    clearTimeout(liveTimer);
    if (p.seconds) liveTimer = setTimeout(() => {
      liveMode = "none";
      api.overlay("post_done").catch(() => {});
    }, p.seconds * 1000);
  });
  on("reset", () => location.reload());

  const maps = $derived((app.kb?.maps ?? []).filter((m: J) => m.in_pool));
  const agents = $derived(app.kb?.agents ?? []);
  const byName = (n: string) => agents.find((a: J) => a.name === n);
  $effect(() => {
    if (!advMap && maps.length) advMap = maps[0].name;
  });
  $effect(() => {
    if (layout !== "advisor" || !advMap) return;
    const t = [...advTeam];
    api.suggest(advMap, t).then((r) => (advRes = r)).catch(() => (advRes = null));
  });
  function addTeammate(n: string) {
    if (advTeam.length < 4 && !advTeam.includes(n)) advTeam = [...advTeam, n];
    if (advTeam.length >= 4) picking = false;
  }

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
  const liveTeam = (mine: boolean) => (live?.players ?? []).filter((p: J) => !!p.mine === mine);
  const showLive = $derived(live && ["compact", "rail", "full"].includes(layout) && liveMode !== "none");
  const label = (p: J) => (p.hidden || !p.name ? p.agent || "Hidden" : p.name);
  function drag(e: MouseEvent) {
    if (edit && e.button === 0 && !(e.target as HTMLElement).closest("button")) getCurrentWindow().startDragging();
  }
</script>

{#snippet rankCell(r: J, peak = false)}
  {#if !r || r.state === "queued"}<span class="faint">·</span>
  {:else if r.state === "loading"}<span class="faint pulse">…</span>
  {:else if r.state === "hidden"}<span class="faint">{t("hidden")}</span>
  {:else if r.state === "private"}<span class="faint">{t("private")}</span>
  {:else if r.state === "error"}<span class="faint">n/a</span>
  {:else if peak}{#if r.peak_tier}<Rank tier={r.peak_tier} short />{:else}<span class="faint">–</span>{/if}
  {:else}<Rank tier={r.tier} rr={r.tier ? r.rr : null} short />{/if}
{/snippet}

{#snippet formCell(f: J)}
  {#if !f || f.state === "queued"}<span class="faint">·</span>
  {:else if f.state === "loading"}<span class="faint pulse">…</span>
  {:else if f.state === "ok"}<span class="mono">{f.win}%</span> <span class="faint mono">{f.kd}</span>
  {:else if f.state === "empty"}<span class="faint">{t("no comp")}</span>
  {:else}<span class="faint">{f.state === "hidden" ? "hidden" : "n/a"}</span>{/if}
{/snippet}

{#if d}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="ov {board ? 'ov-board' : ''} {edit ? 'edit' : ''} {entering ? 'is-entering' : ''} ov-{layout}" data-glass-low={glass < 88 ? "" : undefined} style="background:color-mix(in srgb, var(--bg) {glass}%, transparent);zoom:{(ov.scale ?? 100) / 100}" onmousedown={drag}>
    {#if layout === "advisor"}
      <div class="ov-head">
        <span class="logo"><svg style="color:var(--accent)"><use href="#logo" /></svg></span>
        <b style="font-size:12px">{t("Comp advisor")}{advAuto ? " · agent select" : ""}</b>
        <button class="btn btn-icon btn-ghost btn-sm" style="margin-left:auto" aria-label="Close advisor" onclick={() => api.overlay("advisor")}><Icon name="x" /></button>
      </div>
      <select class="input" style="width:100%;margin-bottom:8px" bind:value={advMap}>
        {#each maps as m (m.name)}<option value={m.name}>{m.name}</option>{/each}
      </select>
      <div class="caps faint" style="font-size:10px;margin-bottom:4px">{t("Teammates locked so far · type them in")}</div>
      <div class="row" style="gap:4px;flex-wrap:wrap;margin-bottom:8px">
        {#each advTeam as n (n)}<Agent name={n} id={byName(n)?.id} sm title="Remove {n}" onclick={() => (advTeam = advTeam.filter((x) => x !== n))} />{/each}
        {#if advTeam.length < 4}<button class="btn btn-sm" onclick={() => (picking = !picking)}><Icon name={picking ? "x" : "plus"} />{picking ? "Done" : "Add"}</button>{/if}
      </div>
      {#if picking}
        <div class="ov-agents">
          {#each agents.filter((a: J) => !advTeam.includes(a.name)) as a (a.id)}<Agent name={a.name} id={a.id} sm title={a.name} onclick={() => addTeammate(a.name)} />{/each}
        </div>
      {:else if advRes}
        {#each advRes.agents.slice(0, 3) as a, i (a.name)}
          <div class="ov-pick" style="--i:{i}">
            <Agent name={a.name} id={a.id} sm />
            <div style="min-width:0"><b>{i + 1}. {a.name}</b> <span class="faint" style="font-size:10px">{a.role}</span>
              <div class="faint" style="font-size:11px">{a.reasons[0] ?? ""}</div></div>
          </div>
        {:else}<div class="faint">{t("Mark the agents you own in the app first.")}</div>{/each}
        {#if advRes.comps?.[0]}
          <div class="caps faint" style="font-size:10px;margin:8px 0 4px">{t("Best full comp")}</div>
          <div class="row" style="gap:4px">{#each advRes.comps[0].agents as n (n)}<Agent name={n} id={byName(n)?.id} sm picked={n === advRes.comps[0].you} />{/each}</div>
          <div class="faint" style="font-size:10.5px;margin-top:4px">{advRes.comps[0].roles}</div>
        {/if}
        {#if advRes.notes}<div class="faint" style="font-size:10.5px;margin-top:8px">{advRes.notes}</div>{/if}
      {/if}
    {:else if showLive}
      <div class="ov-head">
        <span class="logo"><svg style="color:var(--accent)"><use href="#logo" /></svg></span>
        <b style="font-size:12px">{live.map || "Live match"}</b>
        {#if live.context?.tier}<span class="faint" style="font-size:11px;margin-left:auto">{tierName(live.context.tier, true)} · {live.context.rr} RR{#if live.context.streak > 1} · {live.context.streak_kind}{live.context.streak}{/if}</span>{/if}
      </div>
      {#if live.context?.agent && (live.context.wins + live.context.losses) > 0}<div class="faint" style="font-size:11px;margin-bottom:4px">{live.context.map} + {live.context.agent}: {live.context.wins}–{live.context.losses}</div>{/if}
      {#if live.rematch}<div class="tilt" style="margin:4px 0 6px"><Icon name="users" /><span>{live.rematch}</span></div>{/if}
      {#each [true, false] as mine (mine)}
        <div class="caps faint" style="font-size:10px;margin:6px 0 2px">{t(mine ? "Your team" : "Enemy team")}</div>
        {#if liveMode === "full"}
          <div class="lt lt-h"><span></span><span>{t("Player")}</span><span>{t("Rank")}</span><span>{t("Peak")}</span><span>{t("Last act")}</span><span>{t("Recent")}</span><span>{t("On agent")}</span><span>{t("Met")}</span></div>
          {#each liveTeam(mine) as p, i (p.puuid)}
            <div class="lt {p.me ? 'me' : ''}" style="--i:{i}">
              <Agent name={p.agent} id={p.agent_id} sm />
              <span class="nm">{label(p)}{#if p.tag && !p.hidden}<span class="faint">#{p.tag}</span>{/if}{#if p.level} <span class="faint">· {p.level}</span>{/if}{#if p.note?.tags?.length} <span class="tag">{p.note.tags[0]}</span>{/if}</span>
              <span>{@render rankCell(p.rank)}</span><span>{@render rankCell(p.rank, true)}</span>
              <span>{#if p.rank?.last_act_tier}<Rank tier={p.rank.last_act_tier} short />{:else}<span class="faint">–</span>{/if}</span>
              <span>{@render formCell(p.form)}</span>
              <span class="mono">{#if p.form?.on_agent}{p.form.on_agent.wins}/{p.form.on_agent.games}{:else}<span class="faint">·</span>{/if}</span>
              <span class="mono faint">{p.met || ""}</span>
            </div>
          {/each}
        {:else}
          {#each liveTeam(mine) as p, i (p.puuid)}
            <div class="lr2 {p.me ? 'me' : ''} {liveMode === 'rail' ? 'wide' : ''}" style="--i:{i}">
              <Agent name={p.agent} id={p.agent_id} sm />
              <span class="nm">{label(p)}{#if p.met} <span class="faint">{t("· met")} {p.met}</span>{/if}</span>
              <span>{@render rankCell(p.rank)}</span>
              {#if liveMode === "rail"}<span>{@render rankCell(p.rank, true)}</span><span>{@render formCell(p.form)}</span>{/if}
            </div>
          {/each}
        {/if}
      {/each}
      <div class="faint" style="font-size:10px;margin-top:6px">{t("Hidden names stay hidden.")} {liveMode === "compact" ? "Hold Tab for more." : ""}</div>
    {:else}
      <div class="ov-head">
        <span class="logo"><svg style="color:var(--accent)"><use href="#logo" /></svg></span>
        <b style="font-size:12px">{t(board ? "Match result" : PANELS[panel])}</b>
        {#if edit}<button class="btn btn-sm" style="margin-left:auto" onclick={() => api.overlay("edit_off")}><Icon name="check" />{t("Done")}</button>
        {:else}<span class="ov-dots">{#each PANELS as _, i (i)}<i class={!board && i === panel ? "on" : ""}></i>{/each}</span>{/if}
      </div>

      {#if board && d.board}
        <div class="row" style="gap:8px;margin-bottom:6px"><span class="big {score[0] > score[1] ? 'win' : score[0] < score[1] ? 'loss' : ''}">{score[0]}–{score[1]}</span>
          <span class="faint" style="font-size:11px">{d.board.map}{#if d.board.rr !== null && d.board.rr !== undefined} · {signed(d.board.rr)} RR{/if}</span></div>
        {#if d.board.facts?.length}<div class="ov-facts">{#each d.board.facts as f (f)}<div>{f}</div>{/each}</div>{/if}
        {#each [true, false] as mine (mine)}
          <div class="caps faint" style="font-size:10px;margin:6px 0 2px">{t(mine ? "Your team" : "Enemy team")}</div>
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
        {:else}<div class="faint">{t("No matches yet.")}</div>{/if}
      {:else if panel === 1}
        {#if s}
          <div class="row" style="gap:6px;flex-wrap:wrap">{#each s.results as r, i (i)}<span class="badge {r === 'W' ? 'win' : r === 'L' ? 'loss' : ''}">{r}</span>{/each}</div>
          <div class="ov-grid"><div><b class={s.net_rr > 0 ? "win" : s.net_rr < 0 ? "loss" : ""}>{signed(s.net_rr)}</b><small>{t("Net RR")}</small></div>
            <div><b>{s.agg.acs}</b><small>ACS</small></div><div><b>{kd(s.agg.k, s.agg.d)}</b><small>K/D</small></div></div>
          {#if s.active && s.tilt}<div class="tilt"><Icon name="coffee" /><span>{s.loss_streak} {t("losses in a row. A short break usually helps.")}</span></div>{/if}
        {:else}<div class="faint">{t("No session yet.")}</div>{/if}
      {:else}
        {#if rank?.tier}
          <div class="big">{tierName(rank.tier)}</div>
          <div class="progress" style="--v:{rank.rr}%;margin:10px 0 6px"><i></i></div>
          <div class="faint" style="font-size:11px">{rank.rr} {t("/ 100 RR")}{#if rank.last_change} {t("· last")} {signed(rank.last_change)}{/if}{#if rank.peak_tier} {t("· peak")} {tierName(rank.peak_tier, true)}{/if}</div>
        {:else}<div class="faint">{t("Rank appears after the first sync.")}</div>{/if}
      {/if}
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
