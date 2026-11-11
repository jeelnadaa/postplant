<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import { api, errText, type J } from "../lib/api";
  import { fmtDate, parseRiotId, timeAgo } from "../lib/format";
  import { nav, toast } from "../lib/store.svelte";
  import Agent from "../components/Agent.svelte";
  import Icon from "../components/Icon.svelte";
  import NoteModal from "../components/NoteModal.svelte";
  import Rank from "../components/Rank.svelte";
  import Career from "../components/Career.svelte";

  let { params }: { params: Record<string, string> } = $props();
  // The screen is re-created on every route change, so reading params once is intended.
  // svelte-ignore state_referenced_locally
  let q = $state(params.name ? `${params.name}#${params.tag ?? ""}` : "");
  let res = $state<J>(null);
  let recent = $state<J[]>([]);
  let busy = $state(false);
  let error = $state("");
  let noteOpen = $state(false);

  const loadRecent = () => api.recentSearches().then((r) => (recent = r)).catch(() => {});
  loadRecent();

  async function run(force = false) {
    const id = parseRiotId(q);
    if (!id) return (error = "Type a Riot ID like Name#TAG.");
    busy = true;
    error = "";
    try {
      res = await api.search(id.name, id.tag, force);
      loadRecent();
    } catch (e) {
      res = null;
      error = errText(e);
    } finally {
      busy = false;
    }
  }
  $effect(() => {
    if (params.name) run();
  });
  const acc = $derived(res?.data?.account);
  const mmr = $derived(res?.data?.mmr);
  // Detailed career: n full matches (1 request each, kept forever, so asking again is free).
  let career = $state<J>(null);
  let careerN = $state(10);
  let careerBusy = $state(false);
  let careerErr = $state("");
  $effect(() => {
    const a = res?.data?.account;
    career = null;
    careerErr = "";
    if (a?.puuid) api.playerCareer(a.puuid, a.region, 0).then((c) => { if (c?.overall?.games) career = c; }).catch(() => {});
  });
  async function loadCareer() {
    careerBusy = true;
    careerErr = "";
    try {
      career = await api.playerCareer(acc.puuid, acc.region, careerN, "competitive");
      if (career?.error) careerErr = `Stopped early: ${career.error}`;
    } catch (e) {
      careerErr = errText(e);
    } finally {
      careerBusy = false;
    }
  }
  const note = $derived(recent.find((r: J) => r.puuid === acc?.puuid)?.note ?? null);
</script>

<div class="page-head"><div><h1 class="h1">{t("Player lookup")}</h1><div class="sub">{t("Any Riot ID · about 3 requests · cached for 10 minutes · detailed stats cost 1 request per new match")}</div></div></div>
<form class="row mb-4" style="gap:8px" onsubmit={(e) => { e.preventDefault(); if (q) nav("search", parseRiotId(q) ?? {}); }}>
  <div class="input-icon" style="flex:1;max-width:460px"><Icon name="search" /><input class="input" placeholder="Name#TAG" bind:value={q} spellcheck="false" /></div>
  <button class="btn btn-primary" disabled={busy}>{busy ? "Looking up…" : "Look up"}</button>
</form>
{#if error}<div class="banner danger mb-4"><Icon name="alert" />{error}</div>{/if}

{#if acc}
  <div class="grid-12">
    <div class="card col-4">
      <div class="row" style="gap:12px">
        {#if acc.card?.small}<img src={acc.card.small} alt="" width="56" height="56" style="border-radius:var(--r-sm)" />{/if}
        <div style="min-width:0"><div class="h2" style="overflow-wrap:anywhere">{acc.name}<span class="faint">#{acc.tag}</span></div>
          <div class="faint" style="font-size:12px">{t("Level")} {acc.account_level} · {String(acc.region).toUpperCase()}</div></div>
      </div>
      <div class="mt-4">
        {#if mmr?.ok?.current}<Rank tier={mmr.ok.current.tier?.id} rr={mmr.ok.current.rr} lg />
          <div class="kv mt-2" style="font-size:12px">
            <span>{t("Peak")}</span><b>{#if mmr.ok.peak?.tier?.id}<Rank tier={mmr.ok.peak.tier.id} short />{mmr.ok.peak.season?.short ? ` · ${mmr.ok.peak.season.short}` : ""}{:else}—{/if}</b>
            {#if mmr.ok.current.last_change !== undefined}<span>{t("Last change")}</span><b>{mmr.ok.current.last_change > 0 ? "+" : ""}{mmr.ok.current.last_change}</b>{/if}
            {#if mmr.ok.seasonal?.length}<span>{t("This act")}</span><b>{mmr.ok.seasonal[mmr.ok.seasonal.length - 1].wins}W / {mmr.ok.seasonal[mmr.ok.seasonal.length - 1].games} {t("games")}</b>{/if}
          </div>
        {:else if mmr?.private}<div class="row faint" style="gap:6px"><Icon name="lock" />{t("Rank is private")}</div>
        {:else if mmr?.error}<div class="faint">{t("Rank unavailable:")} {mmr.error}</div>{/if}
      </div>
      {#if note}<div class="banner info mt-4"><Icon name="note" /><div>{#each note.tags as t (t)}<span class="tag" style="margin-right:4px">{t}</span>{/each}{note.note}</div></div>{/if}
      <div class="row mt-4" style="gap:8px;flex-wrap:wrap">
        <button class="btn btn-sm" onclick={() => (noteOpen = true)}><Icon name="note" />{note ? "Edit note" : "Add note"}</button>
        <button class="btn btn-sm btn-ghost" disabled={busy} onclick={() => run(true)} title="Uses about 3 requests"><Icon name="refresh" />{t("Refresh")}</button>
      </div>
      <div class="faint mt-2" style="font-size:11px">{res.cached ? `Cached ${timeAgo(res.cached_at)}` : "Fetched just now"}</div>
    </div>
    <div class="card col-8">
      <div class="card-head"><span class="h3">{t("Recent competitive")}</span></div>
      {#if Array.isArray(res.data.recent)}
        <table class="table"><tbody>
          {#each res.data.recent as m (m.id)}
            <tr><td><div class="player"><Agent name={m.agent} id={m.agent_id} sm /><b>{m.map}</b></div></td>
              <td class={m.result === "W" ? "win" : m.result === "L" ? "loss" : ""}>{m.score[0]}–{m.score[1]}</td>
              <td class="mono">{m.k}/{m.d}/{m.a}</td><td class="num">{m.acs} ACS</td><td class="num">{m.hs}{t("% HS")}</td>
              <td><Rank tier={m.tier} short /></td><td class="faint">{fmtDate(m.started_at)}</td></tr>
          {:else}<tr><td class="empty-inline">{t("No recent competitive matches.")}</td></tr>{/each}
        </tbody></table>
      {:else if res.data.recent?.private}<p class="row faint" style="gap:6px"><Icon name="lock" />{t("Match history is private.")}</p>
      {:else}<p class="faint">{t("Match history unavailable")}{res.data.recent?.error ? `: ${res.data.recent.error}` : ""}.</p>{/if}
    </div>
  </div>
  <div class="card mt-4">
    <div class="card-head"><span class="h3">{t("Career")}</span><span class="spacer"></span>
      <select class="input" style="width:auto" bind:value={careerN}>{#each [5, 10, 15, 20, 25] as n (n)}<option value={n}>{t("Last")} {n} {t("competitive")}</option>{/each}</select>
      <button class="btn btn-sm btn-primary" disabled={careerBusy} onclick={loadCareer} title="About 1 request per match not already on this PC"><Icon name="download" />{careerBusy ? "Loading…" : "Load detailed stats"}</button></div>
    {#if careerErr}<div class="banner warn mb-4"><Icon name="warn" />{careerErr}</div>{/if}
    {#if career}
      <p class="faint" style="font-size:12px;margin-top:0">{t("From")} {career.held} {t("full matches held on this PC")}{career.fetched ? ` (${career.fetched} just fetched)` : ""}.</p>
      <Career d={career} />
    {:else if res.data.summary}
      <Career d={res.data.summary} quick />
    {:else}<p class="faint">{t("Load detailed stats for per-agent kills, defuses, clutches, KAST, trades and more.")}</p>{/if}
  </div>
  <NoteModal bind:open={noteOpen} puuid={acc.puuid} who="{acc.name}#{acc.tag}" {note} onsaved={() => { loadRecent(); toast("ok", "Note saved"); }} />
{:else if !busy && recent.length}
  <div class="card">
    <div class="card-head"><span class="h3">{t("Recent lookups")}</span></div>
    <div class="stack" style="gap:4px">
      {#each recent as r (r.puuid)}
        <button class="list-row" onclick={() => nav("search", { name: r.name, tag: r.tag })}>
          <Icon name="user" /><b>{r.name}<span class="faint">#{r.tag}</span></b>
          {#if r.note?.tags?.length}{#each r.note.tags as t (t)}<span class="tag">{t}</span>{/each}{/if}
          <span class="spacer"></span><span class="faint" style="font-size:12px">{timeAgo(r.at)}</span></button>
      {/each}
    </div>
  </div>
{/if}
