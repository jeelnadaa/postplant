<script lang="ts">
  import { t } from "./lib/i18n.svelte";
  import { onMount } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { api, errText, on } from "./lib/api";
  import { timeAgo } from "./lib/format";
  import { app, init, nav, refreshState, toast } from "./lib/store.svelte";
  import Icon from "./components/Icon.svelte";
  import Rank from "./components/Rank.svelte";
  import Toasts from "./components/Toasts.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";
  import Onboarding from "./screens/Onboarding.svelte";
  import Home from "./screens/Home.svelte";
  import Matches from "./screens/Matches.svelte";
  import Match from "./screens/Match.svelte";
  import Analytics from "./screens/Analytics.svelte";
  import Habits from "./screens/Habits.svelte";
  import RankScreen from "./screens/RankScreen.svelte";
  import Advisor from "./screens/Advisor.svelte";
  import Heatmaps from "./screens/Heatmaps.svelte";
  import Search from "./screens/Search.svelte";
  import People from "./screens/People.svelte";
  import Review from "./screens/Review.svelte";
  import Settings from "./screens/Settings.svelte";

  const NAV: [string, [string, string, string][]][] = [
    ["Play", [["home", "house", "Home"], ["matches", "swords", "Matches"], ["analytics", "chart", "Analytics"], ["rank", "trending-up", "Rank"], ["advisor", "compass", "Comp Advisor"], ["heatmaps", "target", "Heatmaps"], ["review", "bookmark", "Review"]]],
    ["Players", [["search", "search", "Search"], ["people", "users", "People & notes"]]],
  ];
  const active = $derived(app.route.name === "match" ? "matches" : app.route.name === "habits" ? "analytics" : app.route.name);
  const setupDone = $derived(app.s?.has_key && app.s?.me);
  let tick = $state(Date.now());

  onMount(() => {
    init();
    // Toasts raised by the Rust side (e.g. the live table hotkey with no match running).
    const upd = on("update-available", (u: { latest: string }) => toast("info", `Postplant ${u.latest} is available`, "Settings → Updates & startup to install it."));
    const un = on("toast", (t: { kind: "ok" | "err" | "warn" | "info"; title: string; text?: string }) => toast(t.kind, t.title, t.text));
    // Drop a .postplant or .json file anywhere to import it.
    const drop = getCurrentWebview().onDragDropEvent((e) => {
      if (e.payload.type === "drop") {
        const f = e.payload.paths.find((x) => /\.(postplant|json)$/i.test(x));
        if (f) nav("settings", { tab: "data", import: f });
      }
    });
    const t = setInterval(() => (tick = Date.now()), 30000);
    return () => {
      clearInterval(t);
      un.then((f) => f());
      upd.then((f) => f());
      drop.then((f) => f());
    };
  });

  async function syncNow() {
    try {
      const r = await api.sync();
      if (r.busy) toast("info", "Already syncing");
      else if (r.new === 0) toast("info", "Up to date", "No new matches.");
      await refreshState();
    } catch (e) {
      toast("err", "Sync failed", errText(e));
    }
  }
  const usage = $derived(app.s?.usage);
</script>

<CommandPalette />
<Toasts />

{#if !app.ready}
  <div style="display:grid;place-items:center;height:100vh"><span class="spinner"></span></div>
{:else if !setupDone || app.route.name === "setup"}
  <Onboarding />
{:else}
  <div class="app-root">
    <div class="shell">
      <aside class="sidebar">
        <div class="brand"><span class="logo"><svg viewBox="0 0 48 48"><use href="#logo" /></svg>{t("Postplant")}</span></div>
        {#each NAV as [sec, items] (sec)}
          <div class="nav-sec caps">{t(sec)}</div>
          {#each items as [id, ic, label] (id)}
            <a class="nav-item" href="#/{id}" aria-current={active === id ? "page" : undefined}><Icon name={ic} />{t(label)}</a>
          {/each}
        {/each}
        <div class="nav-sec caps">{t("App")}</div>
        <a class="nav-item" href="#/settings" aria-current={active === "settings" ? "page" : undefined}><Icon name="settings" />{t("Settings")}</a>
        <button class="me-card" onclick={() => nav("rank")} title="Your rank">
          <span class="avatar">{app.s.me.name[0]?.toUpperCase()}</span>
          <div style="min-width:0;flex:1;text-align:left">
            <div style="font-weight:700;font-size:13px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis">{app.s.me.name}<span class="faint">#{app.s.me.tag}</span></div>
            <Rank tier={app.s.rank?.tier} rr={app.s.rank?.rr} />
          </div>
        </button>
      </aside>
      <div class="main">
        <div class="topbar">
          <button class="cmdk" onclick={() => (app.palette = true)}><Icon name="search" />{t("Search players, screens, actions")}<span class="keys"><span class="kbd">Ctrl</span><span class="kbd">K</span></span></button>
          <span class="spacer"></span>
          {#if app.s.game_running}<span class="status-pill" title="Game-safe mode: no looping animations or blur"><span class="dot warn"></span>{t("VALORANT running")}</span>{/if}
          <button class="status-pill btnish" onclick={syncNow} title={app.s.last_error ?? "Click to sync now"}>
            <span class="dot {app.s.last_error ? 'warn' : app.s.syncing ? '' : ''}"></span>
            {#key tick}{app.s.syncing ? "Syncing…" : app.s.last_error ? "Sync paused" : `Synced ${timeAgo(app.s.last_sync)}`}{/key}
          </button>
          <span class="status-pill tooltip"><Icon name="gauge" /><span class="mono">{usage?.minute ?? 0}/{usage?.limit ?? 30}</span><span class="tip">{t("API this minute ·")} {usage?.today ?? 0} {t("today")}</span></span>
          <button class="btn btn-icon btn-ghost" aria-label="Show or hide overlay" title="Show / hide overlay" onclick={() => api.overlay("toggle_overlay")}><Icon name="layers" /></button>
          <button class="btn btn-icon btn-ghost" aria-label="Sync now" title="Sync now" onclick={syncNow}><Icon name="refresh" /></button>
        </div>
        <div class="content">
          {#if app.s.last_error && /key/i.test(app.s.last_error)}
            <div class="banner danger mb-4"><Icon name="key" /><span>{app.s.last_error}</span><span class="spacer"></span><a href="#/settings?tab=api" class="btn btn-sm">{t("Open API key")}</a></div>
          {/if}
          <div class="page">
            {#key app.route.name + JSON.stringify(app.route.params)}
              {#if app.route.name === "home"}<Home />
              {:else if app.route.name === "matches"}<Matches />
              {:else if app.route.name === "match"}<Match id={app.route.params.id} />
              {:else if app.route.name === "analytics"}<Analytics params={app.route.params} />
              {:else if app.route.name === "habits"}<Habits />
              {:else if app.route.name === "rank"}<RankScreen />
              {:else if app.route.name === "advisor"}<Advisor params={app.route.params} />
              {:else if app.route.name === "heatmaps"}<Heatmaps />
              {:else if app.route.name === "search"}<Search params={app.route.params} />
              {:else if app.route.name === "people"}<People />
              {:else if app.route.name === "review"}<Review />
              {:else if app.route.name === "settings"}<Settings tab={app.route.params.tab} importPath={app.route.params.import} />
              {:else}<Home />{/if}
            {/key}
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<svg width="0" height="0" style="position:absolute" aria-hidden="true">
  <symbol id="logo" viewBox="0 0 48 48">
    <path d="M29.05 5.16A19.5 19.5 0 1 1 7.11 14.25" fill="none" stroke="currentColor" stroke-width="3.6" stroke-linecap="round" />
    <circle cx="16.54" cy="5.98" r="2.6" fill="currentColor" />
    <path d="M24 14.5 33.5 24 24 33.5 14.5 24Z" fill="currentColor" />
  </symbol>
</svg>
