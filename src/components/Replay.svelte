<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  // 2D round replay: step through each kill and see where all ten players stood at that moment,
  // with an arrow from killer to victim. Filter to your own kills and deaths, or to the duels
  // between you and one player across the whole match.
  import { api, errText, type J } from "../lib/api";
  import { mmss, toMinimap } from "../lib/format";
  import { app } from "../lib/store.svelte";
  import ExportImage from "./ExportImage.svelte";
  import Icon from "./Icon.svelte";
  import Select from "./Select.svelte";

  let { id, myTeam, me = "" }: { id: string; myTeam: string; me?: string } = $props();
  let d = $state<J>(null);
  let error = $state("");
  let round = $state(0);
  let step = $state(0);
  // Hovering a kill in the list (or on the map) previews it without moving the slider.
  let hover = $state<number | null>(null);
  // "" all kills · "me" my kills and deaths · a puuid: kills between me and them (enemy) or involving them (teammate)
  let focus = $state("");
  let svgEl = $state<SVGSVGElement>();

  $effect(() => {
    api.replay(id).then((r) => (d = r)).catch((e) => (error = errText(e)));
  });
  const mm = $derived(app.kb?.maps?.find((m: J) => m.name === d?.map)?.minimap);
  const pmap = $derived(new Map<string, J>((d?.players ?? []).map((p: J) => [p.puuid, p])));
  const label = (pu: string) => {
    const p = pmap.get(pu);
    return p?.name || p?.agent || "?";
  };
  const enemy = (pu: string) => pmap.get(pu)?.team !== myTeam;
  const matches = (x: J) => {
    if (!focus) return true;
    if (focus === "me") return x.killer === me || x.victim === me;
    if (me && enemy(focus)) return (x.killer === me && x.victim === focus) || (x.killer === focus && x.victim === me);
    return x.killer === focus || x.victim === focus;
  };
  const focusOptions = $derived<[string, string][]>([
    ["", t("All kills")],
    ...(me ? [["me", t("My kills and deaths")] as [string, string]] : []),
    ...((d?.players ?? []) as J[])
      .filter((p) => p.puuid !== me)
      .sort((a, b) => Number(enemy(b.puuid)) - Number(enemy(a.puuid)))
      .map((p): [string, string] => [p.puuid, me && enemy(p.puuid) ? `${t("Me vs")} ${p.name || p.agent} (${p.agent})` : `${t("Involving")} ${p.name || p.agent} (${p.agent})`]),
  ]);
  const rounds = $derived<J[]>(d?.rounds ?? []);
  const r = $derived(rounds[round]);
  const all = $derived<J[]>(r?.kills ?? []);
  const kills = $derived(all.filter(matches));
  const shown = $derived(Math.min(hover ?? step, kills.length - 1));
  const k = $derived(kills[shown]);
  // Everyone killed earlier in the round (by anyone), so the filter never hides who was already down.
  const dead = $derived(new Set<string>(k ? all.filter((x: J) => x.t < k.t).map((x: J) => x.victim) : []));
  const perRound = $derived(rounds.map((rd: J) => (rd.kills ?? []).filter(matches).length));
  // Head-to-head tally across the match when the filter is "me vs an enemy".
  const duel = $derived.by(() => {
    if (!me || !focus || focus === "me" || !enemy(focus)) return null;
    let mine = 0, theirs = 0;
    for (const rd of rounds) for (const x of rd.kills ?? []) {
      if (x.killer === me && x.victim === focus) mine++;
      else if (x.killer === focus && x.victim === me) theirs++;
    }
    return { mine, theirs };
  });
  const pt = (x: number, y: number) => {
    const v = toMinimap(mm, x, y);
    return v ? { x: v[0] * 1000, y: v[1] * 1000 } : null;
  };
  function setRound(n: number) {
    round = Math.max(0, Math.min(rounds.length - 1, n));
    step = 0;
    hover = null;
  }
  function setFocus(v: string) {
    focus = v;
    step = 0;
    hover = null;
    // Jump to the first round that has a matching kill.
    if (v && !perRound[round]) {
      const i = perRound.findIndex((n) => n > 0);
      if (i >= 0) round = i;
    }
  }
  // Killer → victim arrow, stopping short of both markers so it never covers them.
  const arrow = $derived.by(() => {
    if (!k) return null;
    const kp = (k.pos as [string, number, number][]).find(([pu]) => pu === k.killer);
    const a = kp ? pt(kp[1], kp[2]) : null;
    const b = pt(k.at[0], k.at[1]);
    if (!a || !b) return null;
    const dx = b.x - a.x, dy = b.y - a.y, len = Math.hypot(dx, dy);
    if (len < 30) return null;
    const ux = dx / len, uy = dy / len;
    return { x1: a.x + ux * 18, y1: a.y + uy * 18, x2: b.x - ux * 20, y2: b.y - uy * 20, mx: (a.x + b.x) / 2 - uy * 24, my: (a.y + b.y) / 2 + ux * 24, long: len > 140, mine: pmap.get(k.killer)?.team === myTeam };
  });
  const imgTitle = $derived(k ? `${d?.map} · ${t("Round")} ${round + 1} · ${mmss(k.t)} · ${label(k.killer)} › ${label(k.victim)}${k.weapon ? ` (${k.weapon})` : ""}` : `${d?.map} · ${t("Round")} ${round + 1}`);
</script>

{#if error}<div class="banner danger"><Icon name="alert" />{error}</div>
{:else if d && !mm}<div class="card empty-inline">{t("No minimap data for")} {d.map}.</div>
{:else if d}
  <div class="row mb-2" style="gap:10px;flex-wrap:wrap">
    <Select value={focus} icon="user" options={focusOptions} onchange={setFocus} minWidth={240} />
    {#if duel}<span class="badge {duel.mine > duel.theirs ? 'win' : duel.mine < duel.theirs ? 'loss' : ''}" title={t("Kills between you and this player in the whole match")}>{t("You")} {duel.mine} – {duel.theirs} {label(focus)}</span>{/if}
    <span class="spacer"></span>
    <ExportImage svg={svgEl} title={imgTitle} name={`postplant-${d.map}-round-${round + 1}`} />
  </div>
  <div class="rounds-strip mb-4" role="tablist" aria-label="Rounds">
    {#each rounds as rd, i (i)}
      <button role="tab" aria-selected={i === round} class="rchip {rd.winner === myTeam ? 'won' : 'lost'} {focus && !perRound[i] ? 'dim' : ''}" title="{t('Round')} {i + 1} · {rd.winner === myTeam ? t('won') : t('lost')}{focus ? ` · ${perRound[i]} ${t('matching kills')}` : ''}" onclick={() => setRound(i)}>{i + 1}{#if focus && perRound[i]}<i></i>{/if}</button>
    {/each}
  </div>
  <div class="grid-12">
    <div class="card col-8" style="padding:8px">
      <svg bind:this={svgEl} viewBox="0 0 1000 1000" class="replay" role="img" aria-label="Positions in round {round + 1}">
        <defs>
          <marker id="rp-head-mine" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M0 0 10 5 0 10z" class="head mine" /></marker>
          <marker id="rp-head-theirs" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M0 0 10 5 0 10z" class="head theirs" /></marker>
        </defs>
        <image href={mm.icon} x="0" y="0" width="1000" height="1000" opacity="0.55" />
        {#each kills.slice(0, step) as x, i (i)}
          {@const v = pt(x.at[0], x.at[1])}
          {#if v}<g class="earlier" role="button" tabindex="-1" onmouseenter={() => (hover = i)} onmouseleave={() => (hover = null)} onclick={() => (step = i)} onkeydown={() => {}}><title>{label(x.killer)} › {label(x.victim)}{x.weapon ? ` · ${x.weapon}` : ""}</title><circle cx={v.x} cy={v.y} r="14" fill="transparent" /><line x1={v.x - 7} y1={v.y - 7} x2={v.x + 7} y2={v.y + 7} /><line x1={v.x + 7} y1={v.y - 7} x2={v.x - 7} y2={v.y + 7} /></g>{/if}
        {/each}
        {#if r?.plant?.at?.[0] !== null && r?.plant && (!k || r.plant.t <= k.t)}
          {@const p = pt(r.plant.at[0], r.plant.at[1])}
          {#if p}<rect x={p.x - 9} y={p.y - 9} width="18" height="18" transform="rotate(45 {p.x} {p.y})" class="spike" />{/if}
        {/if}
        {#if k}
          {#each k.pos as [pu, x, y] (pu)}
            {@const p = pt(x, y)}
            {@const pl = pmap.get(pu)}
            {#if p && pl && !dead.has(pu)}<g class="dot {pl.team === myTeam ? 'mine' : 'theirs'} {pu === me ? 'is-me' : ''} {focus && focus !== 'me' && pu === focus ? 'is-focus' : ''}"><circle cx={p.x} cy={p.y} r="13" /><text x={p.x} y={p.y + 30} text-anchor="middle">{pl.agent}</text></g>{/if}
          {/each}
          {#if arrow}
            <line class="kill-arrow {arrow.mine ? 'mine' : 'theirs'}" x1={arrow.x1} y1={arrow.y1} x2={arrow.x2} y2={arrow.y2} marker-end="url(#rp-head-{arrow.mine ? 'mine' : 'theirs'})" />
            {#if k.weapon && arrow.long}<text class="kill-weapon" x={arrow.mx} y={arrow.my + 6} text-anchor="middle">{k.weapon}</text>{/if}
          {/if}
          {@const v = pt(k.at[0], k.at[1])}
          {#if v}<g class="victim"><title>{label(k.killer)} › {label(k.victim)}{k.weapon ? ` · ${k.weapon}` : ""}</title><line x1={v.x - 11} y1={v.y - 11} x2={v.x + 11} y2={v.y + 11} /><line x1={v.x + 11} y1={v.y - 11} x2={v.x - 11} y2={v.y + 11} /></g>{/if}
        {/if}
      </svg>
    </div>
    <div class="col-4 stack" style="gap:10px">
      <div class="card">
        <div class="row" style="gap:8px">
          <button class="btn btn-icon btn-sm" aria-label="Previous round" onclick={() => setRound(round - 1)}><Icon name="chevron-left" /></button>
          <b style="flex:1;text-align:center">{t("Round")} {round + 1} · {r?.winner === myTeam ? t("won") : t("lost")}</b>
          <button class="btn btn-icon btn-sm" aria-label="Next round" onclick={() => setRound(round + 1)}><Icon name="chevron-right" /></button>
        </div>
        {#if kills.length > 1}<input type="range" min="0" max={kills.length - 1} bind:value={step} style="width:100%;margin-top:12px" />{/if}
        <div class="faint" style="font-size:12px">{r?.result}{r?.plant ? ` · planted ${r.plant.site} at ${mmss(r.plant.t)}` : ""}</div>
      </div>
      <div class="card" style="padding:8px">
        {#each kills as x, i (i)}
          {@const kl = pmap.get(x.killer)}
          {@const vc = pmap.get(x.victim)}
          <button class="list-row {i === step ? 'on' : ''} {hover === i ? 'hov' : ''}" onclick={() => (step = i)} onmouseenter={() => (hover = i)} onmouseleave={() => (hover = null)} onfocus={() => (hover = i)} onblur={() => (hover = null)}>
            <span class="mono faint">{mmss(x.t)}</span>
            <span class={kl?.team === myTeam ? "def" : "atk"}>{kl?.name || kl?.agent || "?"}</span><span class="faint">›</span>
            <span class={vc?.team === myTeam ? "def" : "atk"}>{vc?.name || vc?.agent}</span>
            <span class="spacer"></span><span class="faint" style="font-size:11px">{x.weapon || "ability"}</span>
          </button>
        {:else}<p class="faint">{focus ? t("No matching kills this round.") : t("No kills this round.")}</p>{/each}
        {#if dead.size}<div class="faint" style="font-size:11px;margin-top:6px">{dead.size} {t("down before this kill")}</div>{/if}
      </div>
    </div>
  </div>
{:else}<div class="card" style="height:300px"><div class="skeleton" style="height:100%"></div></div>{/if}

<style>
  .replay { width: 100%; height: auto; display: block; }
  .dot circle { stroke: var(--bg); stroke-width: 3; transition: cx 0.25s ease, cy 0.25s ease; }
  .dot.mine circle { fill: var(--def, #4fb8b0); }
  .dot.theirs circle { fill: var(--atk, #ff5a5f); }
  .dot.is-me circle, .dot.is-focus circle { stroke: var(--text); stroke-width: 4; }
  .dot text { font-size: 18px; fill: var(--text); paint-order: stroke; stroke: var(--bg); stroke-width: 4px; }
  .victim line { stroke: var(--text); stroke-width: 4; }
  .spike { fill: var(--warn, #f5b041); }
  .list-row.on { background: var(--accent-soft); }
  .list-row.hov:not(.on) { background: var(--surface-2, rgba(255, 255, 255, 0.05)); }
  .kill-arrow { stroke-width: 5; stroke-linecap: round; pointer-events: none; }
  .kill-arrow.mine, .head.mine { stroke: var(--def, #4fb8b0); fill: var(--def, #4fb8b0); }
  .kill-arrow.theirs, .head.theirs { stroke: var(--atk, #ff5a5f); fill: var(--atk, #ff5a5f); }
  .kill-weapon { font-size: 17px; fill: var(--text); paint-order: stroke; stroke: var(--bg); stroke-width: 4px; pointer-events: none; }
  .earlier line { stroke: var(--text); stroke-width: 3; opacity: 0.35; }
  .earlier { cursor: pointer; }
  .earlier:hover line { opacity: 0.9; }
  .rounds-strip { display: flex; flex-wrap: wrap; gap: 4px; }
  .rchip { position: relative; min-width: 30px; height: 28px; border-radius: 7px; border: 1px solid var(--line, rgba(255, 255, 255, 0.1)); background: transparent; color: var(--muted, #a8a49c); font: inherit; font-size: 12px; cursor: pointer; }
  .rchip.won { box-shadow: inset 0 -3px 0 var(--win, #a3e635); }
  .rchip.lost { box-shadow: inset 0 -3px 0 var(--loss, #ff6a4d); }
  .rchip[aria-selected="true"] { background: var(--accent-soft); color: var(--text); border-color: var(--accent); }
  .rchip.dim { opacity: 0.35; }
  .rchip i { position: absolute; top: 3px; right: 3px; width: 5px; height: 5px; border-radius: 50%; background: var(--accent); }
</style>
