<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import { api, errText, on, type J } from "../lib/api";
  import { queueLabel } from "../lib/format";
  import { app, nav, toast } from "../lib/store.svelte";
  import Agent from "../components/Agent.svelte";
  import Career from "../components/Career.svelte";
  import Icon from "../components/Icon.svelte";
  import Select from "../components/Select.svelte";
  import Switch from "../components/Switch.svelte";

  let { params = {} }: { params?: Record<string, string> } = $props();
  let days = $state(30);
  let aq = $state("competitive");
  let d = $state<J>(null);
  $effect(() => {
    void app.version;
    api.analytics(days, aq).then((r) => (d = r));
  });
  const o = $derived(d?.overall);
  let tr = $state<J>(null);
  let pat = $state<J>(null);
  $effect(() => {
    void app.version;
    api.trades().then((r) => (tr = r)).catch(() => {});
    api.patterns().then((r) => (pat = r)).catch(() => {});
  });
  // svelte-ignore state_referenced_locally
  let view = $state(params.view === "lifetime" ? "lifetime" : "overview");
  // Lifetime: matches held in full plus your HenrikDev record (totals only) for everything else.
  let cq = $state("competitive");
  let cmap = $state("");
  let cagent = $state("");
  let cseason = $state("");
  let cdays = $state(0);
  let withHist = $state(true);
  let career = $state<J>(null);
  let prog = $state<J>(null);
  let histBusy = $state(false);
  $effect(() => {
    void app.version;
    if (view === "lifetime")
      api.myCareer({ queue: cq, map: cmap, agent: cagent, season: cseason, days: cdays, history: withHist }).then((r) => (career = r)).catch(() => (career = null));
  });
  $effect(() => {
    const un = on("history-progress", (p: J) => {
      if (p.done) {
        prog = null;
        app.version++;
      } else prog = p;
    });
    return () => void un.then((f) => f());
  });
  async function loadHistory() {
    histBusy = true;
    try {
      const r = await api.historyBackfill();
      if (r.busy) toast("info", t("Already downloading your record"));
      else toast("ok", `${r.added} ${t("matches added to your record")}`);
      app.version++;
    } catch (e) {
      toast("err", t("Couldn't download your record"), errText(e));
    }
    histBusy = false;
  }
  const seasonLabel = (x: string) => x.replace(/^e(\d+)a(\d+)$/i, "E$1 A$2").replace(/^s(\d+)a(\d+)$/i, "S$1 A$2");
  const day = (ms: number) => (ms ? new Date(ms).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" }) : "—");
  const src = $derived(career?.sources);
  const hist = $derived(career?.history);
</script>

<div class="page-head">
  <div><h1 class="h1">{t("Analytics")}</h1><div class="sub">{view === "lifetime" ? t("Lifetime · your whole record") : `${aq ? queueLabel(aq) : t("All modes")} · ${o?.games ?? 0} ${t("matches · all from this PC, zero API requests")}`}</div></div>
  <span class="spacer"></span>
  {#if view === "overview"}
  <Select bind:value={aq} icon="swords" options={[["competitive", "Competitive"], ["unrated", "Unrated"], ["swiftplay", "Swiftplay"], ["premier", "Premier"], ["", "All modes"]]} minWidth={160} />
  <div class="segmented">{#each [[7, "7 d"], [30, "30 d"], [90, "90 d"], [0, "All"]] as [v, l] (v)}<button aria-pressed={days === v} onclick={() => (days = v as number)}>{t(String(l))}</button>{/each}</div>
  {/if}
</div>
<div class="tabs mb-4"><button role="tab" class="tab" aria-selected={view === "overview"} onclick={() => (view = "overview")}>{t("Overview")}</button><button role="tab" class="tab" aria-selected={view === "lifetime"} onclick={() => (view = "lifetime")}>{t("Lifetime")}</button><button role="tab" class="tab" aria-selected="false" onclick={() => nav("habits")}>{t("When you play best")}</button></div>
{#if view === "lifetime"}
  <div class="card mb-4">
    <div class="row" style="gap:12px;flex-wrap:wrap;align-items:center">
      <Icon name="history" />
      <div style="flex:1;min-width:260px">
        {#if src}
          <b>{(src.full ?? 0) + (src.history ?? 0)} {t("matches")}</b>
          <span class="muted">· {src.full} {t("in full on this PC")}{#if withHist}{" + "}{src.history} {t("from your HenrikDev record")}{/if}{#if src.first}{" · "}{day(src.first)} – {day(src.last)}{/if}</span>
        {/if}
        <div class="faint" style="font-size:12px;margin-top:2px">
          {#if prog}{t("Downloading your record…")} {t("page")} {prog.page} · {prog.added} {t("new so far")}
          {:else if hist?.complete}{t("Your whole HenrikDev record is downloaded")} ({hist.count}). {t("It tops up by itself every 6 hours.")}
          {:else if hist?.count}{t("Part of your record is downloaded")} ({hist.count}). {t("The rest downloads in the background, or now with the button.")}
          {:else}{t("Your record hasn't been downloaded yet. It starts by itself a minute after Postplant opens, or now with the button.")}{/if}
        </div>
      </div>
      <div class="row" style="gap:8px;font-size:13px"><Switch bind:checked={withHist} label="Include HenrikDev record" />{t("Include record")}</div>
      <button class="btn btn-sm {histBusy || prog ? 'is-loading' : ''}" onclick={loadHistory} disabled={histBusy || !!prog}><Icon name="download" />{hist?.complete ? t("Check for more") : t("Download record")}</button>
    </div>
    {#if withHist && src?.history > 0}<p class="faint" style="font-size:12px;margin:10px 0 0">{t("Totals (games, win rate, K/D, ACS, ADR, headshots) count every match. Round-level stats (KAST, first duels, plants, clutches, trades, economy) come from the")} {career?.overall?.detail_games ?? src.full} {t("matches held in full.")}</p>{/if}
  </div>
  <div class="row mb-4" style="gap:10px;flex-wrap:wrap">
    <div class="segmented">{#each [["competitive", "Competitive"], ["unrated", "Unrated"], ["swiftplay", "Swiftplay"], ["", "All modes"]] as [v, l] (v)}{@const n = v ? career?.queues?.find((q: J) => q.name === v)?.games : career?.queues?.reduce((a: number, q: J) => a + q.games, 0)}<button aria-pressed={cq === v} onclick={() => (cq = v)}>{t(l)}{#if n}<span class="faint" style="margin-left:6px">{n}</span>{/if}</button>{/each}</div>
    <Select bind:value={cmap} icon="map" options={[["", "All maps"], ...((career?.avail?.maps ?? []) as string[]).map((m): [string, string] => [m, m])]} minWidth={140} />
    <Select bind:value={cagent} icon="user" options={[["", "All agents"], ...((career?.avail?.agents ?? []) as string[]).map((a): [string, string] => [a, a])]} minWidth={150} />
    <Select bind:value={cseason} icon="flag" options={[["", "All acts"], ...((career?.avail?.seasons ?? []) as string[]).map((x): [string, string] => [x, seasonLabel(x)])]} minWidth={130} />
    <Select value={String(cdays)} icon="clock" options={[["0", "All time"], ["7", "Last 7 days"], ["30", "Last 30 days"], ["90", "Last 90 days"], ["365", "Last year"]]} onchange={(v) => (cdays = Number(v))} minWidth={150} />
    {#if cmap || cagent || cseason || cdays}<button class="btn btn-sm btn-ghost" onclick={() => { cmap = ""; cagent = ""; cseason = ""; cdays = 0; }}><Icon name="x" />{t("Clear filters")}</button>{/if}
  </div>
  {#if career}<Career d={career} />{:else}<div class="card" style="height:200px"><div class="skeleton" style="height:100%"></div></div>{/if}
{:else if d && o.games === 0}
  <div class="card empty-inline">{t("No competitive matches in this period.")}</div>
{:else if d}
  <div class="grid-12">
    <div class="col-12 stat-grid" style="grid-template-columns:repeat(6,1fr)">
      {#each [[`${o.win}%`, "Win rate"], [o.acs, "ACS"], [o.kd, "K/D"], [`${o.hs}%`, "Headshot"], [o.adr, "ADR"], [d.untraded !== null ? `${d.untraded}%` : "—", "Untraded deaths"]] as [v, l] (l)}
        <div class="stat"><div class="v">{v}</div><div class="l">{l}</div></div>
      {/each}
    </div>
    <div class="card col-6"><div class="card-head"><span class="h3">{t("By agent")}</span></div>
      <table class="table"><thead><tr><th>{t("Agent")}</th><th class="num">{t("Games")}</th><th class="num">{t("Win")}</th><th class="num">ACS</th><th class="num">K/D</th></tr></thead><tbody>
        {#each d.agents.slice(0, 8) as a (a.name)}<tr><td><div class="player"><Agent name={a.name} id={a.agent_id} sm /><b>{a.name}</b></div></td><td class="num">{a.games}</td><td class="num">{a.win}%</td><td class="num">{a.acs}</td><td class="num">{a.kd}</td></tr>{/each}
      </tbody></table></div>
    <div class="card col-6"><div class="card-head"><span class="h3">{t("By map & side")}</span><span class="spacer"></span><span class="faint" style="font-size:12px">{t("round win %")}</span></div>
      <table class="table"><thead><tr><th>{t("Map")}</th><th class="num">{t("Games")}</th><th class="num atk">ATK</th><th class="num def">DEF</th><th class="num">{t("Match win")}</th></tr></thead><tbody>
        {#each d.maps as m (m.name)}<tr><td><b>{m.name}</b></td><td class="num">{m.games}</td><td class="num">{m.atk ?? "—"}{m.atk !== null ? "%" : ""}</td><td class="num">{m.def ?? "—"}{m.def !== null ? "%" : ""}</td><td class="num">{m.win}%</td></tr>{/each}
      </tbody></table></div>
    <div class="card col-6"><div class="card-head"><span class="h3">{t("Economy")}</span><span class="spacer"></span><span class="faint" style="font-size:12px">{t("round win % by your loadout")}</span></div>
      <div class="stack" style="gap:10px">{#each d.economy as e (e.name)}
        <div class="bar-h"><span>{e.name}</span><span class="b"><i style="width:{e.win ?? 0}%;background:var(--chart-2)"></i></span><span class="mono" style="text-align:right">{e.win ?? "—"}{e.win !== null ? "%" : ""}</span></div>{/each}</div>
      <p class="faint" style="font-size:12px;margin:10px 0 0">{t("Eco under 2,000 · force under 3,900 · pistol = rounds 1 and 13")}</p></div>
    {#if pat?.notes?.length}
      <div class="card col-12"><div class="card-head"><span class="h3">{t("Patterns in your last")} {pat.matches} {t("matches")}</span><span class="spacer"></span><span class="faint" style="font-size:12px">{t("facts, not advice")}</span></div>
        <div class="stack" style="gap:8px">{#each pat.notes as n, i (i)}
          <div class="note-insight"><span class="ic"><Icon name={n.kind === "opening" ? "crosshair" : "map-pin"} /></span><div style="font-size:13px">{n.text}
            {#if n.refs?.length}<div class="row" style="gap:4px;flex-wrap:wrap;margin-top:4px">{#each n.refs.slice(0, 6) as r (r.match_id + r.round)}<button class="tag" onclick={() => nav("match", { id: r.match_id })}>R{r.round + 1}</button>{/each}</div>{/if}</div><span></span></div>
        {/each}</div></div>
    {/if}
    {#if tr?.deaths}
      <div class="card col-6"><div class="card-head"><span class="h3">{t("Trades")}</span><span class="spacer"></span><span class="faint" style="font-size:12px">{t("last")} {tr.matches} {t("competitive")}</span></div>
        <div class="stat-grid" style="grid-template-columns:repeat(3,1fr)">
          <div class="stat stat-sm"><div class="v">{tr.traded_pct}%</div><div class="l">{t("Your deaths traded")}</div></div>
          <div class="stat stat-sm"><div class="v">{tr.avg_trade_ms ? (tr.avg_trade_ms / 1000).toFixed(1) + " s" : "—"}</div><div class="l">{t("Average time to trade")}</div></div>
          <div class="stat stat-sm"><div class="v">{tr.team_traded_pct}%</div><div class="l">{t("Team deaths traded")}</div></div>
        </div>
        {#if tr.partners.length}<div class="label mt-4 mb-2">{t("Best trading partners")}</div>
          <table class="table"><thead><tr><th>{t("Teammate")}</th><th class="num">{t("Traded you")}</th><th class="num">{t("You traded them")}</th></tr></thead><tbody>
            {#each tr.partners as p (p.puuid)}<tr><td><b>{p.name}</b></td><td class="num">{p.traded_me}</td><td class="num">{p.i_traded}</td></tr>{/each}
          </tbody></table>{/if}
      </div>
    {/if}
    <div class="card col-6"><div class="card-head"><span class="h3">{t("Your kills by weapon")}</span></div>
      <div class="stack" style="gap:10px">{#each d.weapons as w (w.name)}
        <div class="bar-h"><span>{w.name}</span><span class="b"><i style="width:{w.share}%"></i></span><span class="mono" style="text-align:right">{w.share}%</span></div>{:else}<p class="faint">{t("No kill data yet.")}</p>{/each}</div></div>
  </div>
{/if}
