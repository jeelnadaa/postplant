<script lang="ts">
  import { LANGS, t } from "../lib/i18n.svelte";
  import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
  import { api, errText, on, type J } from "../lib/api";
  import { fmtDate, timeAgo } from "../lib/format";
  import { app, nav, refreshState, savePrefs, toast } from "../lib/store.svelte";
  import Icon from "../components/Icon.svelte";
  import Modal from "../components/Modal.svelte";
  import Select from "../components/Select.svelte";
  import Switch from "../components/Switch.svelte";
  import HotkeyModal from "../components/HotkeyModal.svelte";

  let { tab = "appearance", importPath = "" }: { tab?: string; importPath?: string } = $props();
  const TABS: [string, string, string][] = [
    ["appearance", "Appearance", "sun"], ["api", "API key & usage", "key"], ["overlay", "Overlay & hotkeys", "layers"], ["live", "Live match", "wifi"],
    ["recordings", "Recordings", "video"], ["data", "Data & privacy", "database"], ["updates", "Updates & startup", "download"], ["about", "About", "info"],
  ];
  const p = $derived(app.s.prefs);
  const u = $derived(app.s.usage);

  // API key
  let newKey = $state("");
  let keyBusy = $state(false);
  async function replaceKey() {
    keyBusy = true;
    try {
      await api.setKey(newKey.trim());
      newKey = "";
      toast("ok", "Key saved and tested");
      refreshState();
    } catch (e) {
      toast("err", "Key not accepted", errText(e));
    } finally {
      keyBusy = false;
    }
  }
  async function removeKey() {
    await api.removeKey();
    toast("ok", "Key removed from Windows Credential Manager");
    refreshState();
  }

  // Field check
  let fc = $state<J>(null);
  let fcBusy = $state(false);
  async function runFieldCheck() {
    fcBusy = true;
    try {
      fc = await api.fieldCheck();
    } catch (e) {
      toast("err", "Field check failed", errText(e));
    } finally {
      fcBusy = false;
    }
  }
  function copyFieldCheck() {
    const lines = [`Postplant ${fc.version} field check`, ...fc.groups.map((g: J) => `${g.group}: ${g.ok} ok${g.missing.length ? `, missing ${g.missing.join(", ")}` : ""}${g.error ? `, error ${g.error}` : ""}`),
      `Kills with player positions: ${fc.kills_with_positions}/${fc.kills}`, `Plant locations: ${fc.plant_locations ? "yes" : "no"}`];
    navigator.clipboard.writeText(lines.join("\n")).then(() => toast("ok", "Copied", "Paste it into a GitHub issue.")).catch(() => {});
  }

  // Hotkeys
  const HK: [string, string][] = [
    ["toggle_overlay", "Show / hide overlay"], ["edit_overlay", "Move overlay"], ["reset_overlay", "Reset overlay position"],
    ["cycle_panel", "Next overlay panel"], ["refresh", "Check for new matches"], ["open_app", "Open Postplant"], ["advisor", "Comp Advisor in overlay"], ["live_table", "Live match table"],
  ];
  let hkKey = $state("");
  let hkOpen = $state(false);
  const hkLabel = $derived(HK.find(([k]) => k === hkKey)?.[1] ?? "");
  const hkTaken = $derived(Object.fromEntries(HK.filter(([k]) => k !== hkKey).map(([k, l]) => [l, p.hotkeys[k] ?? ""])));
  function saveHotkey(combo: string) {
    const next = { ...p.hotkeys, [hkKey]: combo };
    // A combination moves: whatever had it before is turned off.
    if (combo) for (const [k] of HK) if (k !== hkKey && (next[k] ?? "").toLowerCase() === combo.toLowerCase()) next[k] = "";
    savePrefs({ hotkeys: next });
  }

  // Live match
  let liveWarn = $state(false);
  let liveAgree = $state(false);
  let liveSt = $state<J>(null);
  $effect(() => {
    if (tab === "live") api.liveStatus().then((v) => (liveSt = v)).catch(() => {});
  });
  function setLive(v: boolean) {
    if (v && !p.live_ack) {
      liveAgree = false;
      liveWarn = true;
      return;
    }
    savePrefs({ live_enabled: v });
  }
  function acceptLive() {
    savePrefs({ live_enabled: true, live_ack: { at: new Date().toISOString(), version: app.s.version } });
    liveWarn = false;
  }

  // Recordings
  let scan = $state<J>(null);
  async function chooseFolder() {
    const dir = await openDialog({ directory: true, title: "Choose your recordings folder" });
    if (typeof dir !== "string") return;
    try {
      scan = await api.scanRecordings(dir);
      refreshState();
    } catch (e) {
      toast("err", "Couldn't scan", errText(e));
    }
  }
  async function rescan() {
    try {
      scan = await api.scanRecordings();
    } catch (e) {
      toast("err", "Couldn't scan", errText(e));
    }
  }

  // Data
  let backups = $state<J[]>([]);
  const loadBackups = () => api.backups().then((b) => (backups = b)).catch(() => {});
  $effect(() => {
    if (tab === "data") loadBackups();
  });
  const stamp = () => new Date().toISOString().slice(0, 10);
  async function exportBackup() {
    const path = await saveDialog({ defaultPath: `postplant-${stamp()}.postplant`, filters: [{ name: "Postplant backup", extensions: ["postplant"] }] });
    if (!path) return;
    try {
      await api.exportBackup(path);
      toast("ok", "Backup saved", path);
    } catch (e) {
      toast("err", "Export failed", errText(e));
    }
  }
  let anonymise = $state(true);
  let jsonDays = $state(0);
  async function exportJson() {
    const path = await saveDialog({ defaultPath: `postplant-${stamp()}.json`, filters: [{ name: "JSON", extensions: ["json"] }] });
    if (!path) return;
    try {
      await api.exportJson(path, anonymise, jsonDays || undefined);
      toast("ok", "JSON exported", path);
    } catch (e) {
      toast("err", "Export failed", errText(e));
    }
  }
  let imp = $state<J>(null);
  let impPath = $state("");
  let impOpen = $state(false);
  let mode = $state("merge");
  let notes = $state("newer");
  let withSettings = $state(false);
  let confirmWord = $state("");
  async function chooseImport() {
    const path = await openDialog({ filters: [{ name: "Postplant backup or JSON", extensions: ["postplant", "json"] }] });
    if (typeof path !== "string") return;
    inspectPath(path);
  }
  // A file dropped on the window arrives here through the route.
  $effect(() => {
    if (importPath) inspectPath(importPath);
  });
  async function inspectPath(path: string) {
    impPath = path;
    if (path.endsWith(".json")) {
      imp = { json: true };
      impOpen = true;
      return;
    }
    try {
      imp = await api.inspectImport(path);
      mode = imp?.same_owner === false ? "replace" : "merge";
      confirmWord = "";
      impOpen = true;
    } catch (e) {
      toast("err", "Can't import this file", errText(e));
    }
  }
  async function applyImport() {
    try {
      const r = await api.applyImport(impPath, mode, notes, withSettings);
      impOpen = false;
      toast("ok", "Import finished", r?.matches !== undefined ? `${r.matches} matches added` : undefined);
      app.version++;
      refreshState();
    } catch (e) {
      toast("err", "Import failed", errText(e));
    }
  }
  async function backupNow() {
    try {
      await api.backupNow();
      loadBackups();
      toast("ok", "Backup made");
    } catch (e) {
      toast("err", "Backup failed", errText(e));
    }
  }
  let restoreId = $state<number | null>(null);
  let restoreOpen = $state(false);
  async function restore() {
    if (restoreId === null) return;
    try {
      await api.restoreBackup(restoreId);
      restoreOpen = false;
      toast("ok", "Backup restored");
      app.version++;
      refreshState();
    } catch (e) {
      toast("err", "Restore failed", errText(e));
    }
  }
  let delOpen = $state(false);
  let delKey = $state(false);
  let delWord = $state("");

  // Updates
  let autostart = $state(false);
  api.getAutostart().then((v) => (autostart = !!v)).catch(() => {});
  let upd = $state<J>(null);
  let updBusy = $state(false);
  let installing = $state(false);
  let progress = $state(0);
  on("update-progress", (p: J) => (progress = p.total ? Math.round((100 * p.got) / p.total) : 0));
  async function installUpdate() {
    installing = true;
    try {
      await api.installUpdate();
    } catch (e) {
      toast("err", "Update not installed", errText(e));
      installing = false;
    }
  }
  async function checkUpdate() {
    updBusy = true;
    try {
      upd = await api.checkUpdate();
    } catch (e) {
      toast("err", "Couldn't check for updates", errText(e));
    } finally {
      updBusy = false;
    }
  }
</script>

<div class="page-head"><div><h1 class="h1">{t("Settings")}</h1></div></div>
<div class="settings-layout">
  <nav class="settings-nav">
    {#each TABS as [id, label, ic] (id)}
      <button class="nav-item" aria-current={tab === id ? "page" : undefined} onclick={() => nav("settings", { tab: id })}><Icon name={ic} />{t(label)}</button>
    {/each}
  </nav>
  <div class="stack" style="gap:16px;min-width:0">
    {#if tab === "appearance"}
      <div class="card stack" style="gap:16px">
        <div class="setting"><div><b>{t("Language")}</b><div class="faint">English · हिन्दी</div></div>
          <Select value={p.lang ?? "en"} options={LANGS} onchange={(v) => savePrefs({ lang: v })} /></div>
        <div class="setting"><div><b>{t("Theme")}</b><div class="faint">{t("Ink is the default dark theme. Paper is light.")}</div></div>
          <Select value={p.theme} options={[["ink", "Ink (dark)"], ["midnight", "Midnight"], ["nebula", "Nebula"], ["paper", "Paper (light)"], ["frost", "Frost (light)"], ["sandstone", "Sandstone (light)"], ["system", "Follow Windows"]]} onchange={(v) => savePrefs({ theme: v })} /></div>
        <div class="setting"><div><b>{t("Font")}</b></div>
          <Select value={p.font} options={[["satoshi", "Satoshi"], ["general", "General Sans"], ["geist", "Geist"], ["inter", "Inter"], ["atkinson", "Atkinson Hyperlegible"]]} onchange={(v) => savePrefs({ font: v })} /></div>
        <div class="setting"><div><b>{t("Text size")}</b><div class="faint">{p.scale}%</div></div>
          <input type="range" min="85" max="130" step="5" value={p.scale} onchange={(e) => savePrefs({ scale: +(e.currentTarget as HTMLInputElement).value })} /></div>
        <div class="setting"><div><b>{t("Colour-blind friendly")}</b><div class="faint">{t("Wins and losses use blue and orange plus shapes")}</div></div>
          <Switch checked={p.cvd} onchange={(v) => savePrefs({ cvd: v })} /></div>
        <div class="setting"><div><b>{t("Reduce motion")}</b><div class="faint">{t("Animations always pause while VALORANT is running")}</div></div>
          <Select value={p.reduce_motion} options={[["system", "Follow Windows"], ["on", "On"], ["off", "Off"]]} onchange={(v) => savePrefs({ reduce_motion: v })} /></div>
      </div>
    {:else if tab === "api"}
      <div class="card stack" style="gap:14px">
        <div class="row" style="gap:10px"><Icon name="key" /><b>{t("HenrikDev API key")}</b><span class="spacer"></span>
          <span class="badge {app.s.has_key ? 'win' : 'loss'}">{app.s.has_key ? "Saved in Credential Manager" : "No key"}</span></div>
        <p class="muted" style="margin:0;font-size:13px">{t("Your own free key. It's stored by Windows, never in Postplant's files, exports or logs, and only sent to api.henrikdev.xyz.")}</p>
        <div class="row" style="gap:8px">
          <input class="input mono" style="flex:1" type="password" placeholder="HDEV-…" bind:value={newKey} autocomplete="off" />
          <button class="btn btn-primary" disabled={!newKey.trim() || keyBusy} onclick={replaceKey}>{keyBusy ? "Testing…" : app.s.has_key ? "Replace" : "Save"}</button>
          {#if app.s.has_key}<button class="btn btn-ghost" onclick={removeKey}><Icon name="trash" />{t("Remove")}</button>{/if}
        </div>
      </div>
      <div class="card">
        <div class="card-head"><span class="h3">{t("Usage")}</span></div>
        <div class="stat-grid" style="grid-template-columns:repeat(3,1fr)">
          <div class="stat stat-sm"><div class="v">{u?.minute ?? 0}<span class="faint" style="font-size:14px"> / {u?.limit ?? 30}</span></div><div class="l">{t("This minute")}</div></div>
          <div class="stat stat-sm"><div class="v">{u?.today ?? 0}</div><div class="l">{t("Today")}</div></div>
          <div class="stat stat-sm"><div class="v">{u?.paused_s ? `${u.paused_s}s` : "—"}</div><div class="l">{t("Background paused")}</div></div>
        </div>
        <div class="setting mt-4"><div><b>{t("Daily soft cap")}</b><div class="faint">{t("Background sync stops for the day after this many requests. Your own lookups still work.")}</div></div>
          <input class="input" type="number" min="100" max="20000" step="100" style="width:110px" value={p.daily_cap} onchange={(e) => savePrefs({ daily_cap: +(e.currentTarget as HTMLInputElement).value })} /></div>
        <p class="notice mt-2">{t("Background sync uses at most half of your per-minute limit, so lookups always have room.")}</p>
      </div>
      <div class="card stack" style="gap:12px">
        <div class="card-head" style="margin:0"><span class="h3">{t("Field check")}</span><span class="spacer"></span>
          {#if fc}<button class="btn btn-sm btn-ghost" onclick={copyFieldCheck}><Icon name="copy" />{t("Copy report")}</button>{/if}
          <button class="btn btn-sm" disabled={fcBusy} onclick={runFieldCheck}><Icon name="check" />{fcBusy ? "Checking…" : "Run check"}</button></div>
        <p class="faint" style="margin:0;font-size:13px">{t("Checks your newest stored match and one rank request against every field Postplant reads, so changes in HenrikDev's API show up here first. The report has field names only, no stats or names.")}</p>
        {#if fc}
          <div class="banner {fc.missing ? 'warn' : 'info'}"><Icon name={fc.missing ? "warn" : "check"} />{fc.missing ? `${fc.missing} fields missing. Stats that use them show as —.` : "Every field is present."} {t("Kills with positions:")} {fc.kills_with_positions}/{fc.kills}.</div>
          <table class="table"><tbody>
            {#each fc.groups as g (g.group)}<tr><td><b>{g.group}</b></td><td class="num">{g.ok} {t("ok")}</td><td class="faint mono" style="font-size:11px">{g.error ?? g.missing.join(", ")}</td></tr>{/each}
          </tbody></table>
        {/if}
      </div>
    {:else if tab === "overlay"}
      <div class="card stack" style="gap:16px">
        <div class="setting"><div><b>{t("Overlay")}</b><div class="faint">{t("A small always-on-top window. Click-through unless you're moving it. Works in windowed fullscreen.")}</div></div>
          <Switch checked={p.overlay.enabled} onchange={(v) => { savePrefs({ overlay: { enabled: v } }); api.overlay(v ? "show" : "hide"); }} /></div>
        <div class="setting"><div><b>{t("Show the scoreboard after each match")}</b><div class="faint">{t("For")} {p.overlay.post_seconds} {t("seconds, then back to your panel")}</div></div>
          <Switch checked={p.overlay.auto_post} onchange={(v) => savePrefs({ overlay: { auto_post: v } })} /></div>
        <div class="setting"><div><b>{t("Background opacity")}</b><div class="faint">{p.overlay.opacity}{t("%. Text always stays fully readable.")}</div></div>
          <input type="range" min="75" max="100" step="1" value={p.overlay.opacity} onchange={(e) => savePrefs({ overlay: { opacity: +(e.currentTarget as HTMLInputElement).value } })} /></div>
        <div class="setting"><div><b>{t("Size")}</b><div class="faint">{p.overlay.scale}%</div></div>
          <input type="range" min="70" max="150" step="5" value={p.overlay.scale} onchange={(e) => savePrefs({ overlay: { scale: +(e.currentTarget as HTMLInputElement).value } })} /></div>
        <div class="setting"><div><b>{t("Losses in a row before the break nudge")}</b></div>
          <input class="input" type="number" min="2" max="10" style="width:80px" value={p.tilt_after} onchange={(e) => savePrefs({ tilt_after: +(e.currentTarget as HTMLInputElement).value })} /></div>
        <div class="row" style="gap:8px"><button class="btn btn-sm" onclick={() => api.overlay("edit_on")}><Icon name="pencil" />{t("Move overlay")}</button><button class="btn btn-sm btn-ghost" onclick={() => api.overlay("reset")}><Icon name="undo" />{t("Reset position")}</button></div>
      </div>
      <div class="card stack" style="gap:12px">
        <div class="card-head" style="margin:0"><span class="h3">{t("Hotkeys")}</span></div>
        <div class="setting"><div><b>{t("Only while VALORANT is running")}</b><div class="faint">{t("So they don't clash with other apps the rest of the time")}</div></div>
          <Switch checked={p.hotkeys_game_only} onchange={(v) => savePrefs({ hotkeys_game_only: v })} /></div>
        {#each HK as [k, label] (k)}
          <div class="setting"><div>{t(label)}</div>
            <button class="btn btn-sm hk-btn" onclick={() => { hkKey = k; hkOpen = true; }}><Icon name="keyboard" /><span class="mono">{p.hotkeys[k] || "Off"}</span></button></div>
        {/each}
        <p class="notice">{t("Click a hotkey and press the new combination. Postplant checks Windows will give it to us before saving.")}</p>
      </div>
    {:else if tab === "live"}
      <div class="card stack" style="gap:16px">
        <div class="setting"><div><b>{t("Live match")}</b><div class="faint">{t("Reads the Riot Client running on this PC to know when you're in agent select or in a match. Off by default.")}</div></div>
          <Switch checked={!!p.live_enabled} onchange={setLive} /></div>
        {#if p.live_enabled}
          <div class="banner info"><Icon name="wifi" />{t("Status:")} <b style="margin-left:4px">{liveSt?.phase ?? "…"}</b>{#if liveSt?.map}&nbsp;· {liveSt.map}{/if}{#if liveSt?.error}&nbsp;· <span class="faint">{liveSt.error}</span>{/if}</div>
          <div class="setting"><div><b>{t("Open the Comp Advisor in agent select")}</b><div class="faint">{t("Your own planning panel, with the map filled in. Teammates' picks are still typed by you.")}</div></div>
            <Switch checked={p.advisor_in_select} onchange={(v) => savePrefs({ advisor_in_select: v })} /></div>
          <div class="setting"><div><b>{t("Hold Tab for ranks")}</b><div class="faint">{t("Shows rank, peak and form next to the scoreboard while Tab is held")}</div></div>
            <Switch checked={p.live_tab} onchange={(v) => savePrefs({ live_tab: v })} /></div>
          <div class="setting"><div><b>{t("Recent form")}</b><div class="faint">{t("One extra request per player for win %, K/D and experience on their agent")}</div></div>
            <Switch checked={p.live_form} onchange={(v) => savePrefs({ live_form: v })} /></div>
          <div class="setting"><div><b>{t("Full table")}</b><div class="faint">{t("Hotkey")} {p.hotkeys.live_table || "off"}</div></div>
            <button class="btn btn-sm" onclick={() => { hkKey = "live_table"; hkOpen = true; }}><Icon name="keyboard" />{t("Change")}</button></div>
        {/if}
        <p class="notice">{t("Nothing about other players is read during agent select. Players who hide their name stay hidden, and Postplant never tries to find out who they are. Riot sign-in tokens stay in memory and only go to Riot. Use windowed fullscreen so the overlay can show.")}</p>
      </div>
    {:else if tab === "recordings"}
      <div class="card stack" style="gap:14px">
        <div class="setting"><div><b>{t("Recordings folder")}</b><div class="faint mono" style="overflow-wrap:anywhere">{p.recordings_dir ?? "Not set"}</div></div>
          <div class="row" style="gap:8px"><button class="btn btn-sm" onclick={chooseFolder}><Icon name="folder" />{t("Choose…")}</button>{#if p.recordings_dir}<button class="btn btn-sm btn-ghost" onclick={rescan}><Icon name="refresh" />{t("Rescan")}</button>{/if}</div></div>
        {#if scan}<div class="banner info"><Icon name="video" />{scan.files} {t("videos found,")} {scan.matched} {t("matched to matches.")}</div>{/if}
        <p class="notice">{t("Postplant reads file names and times only, to line videos up with matches. Videos are never uploaded or changed. VLC and mpv open at the exact moment; other players get the time copied to your clipboard.")}</p>
      </div>
    {:else if tab === "data"}
      <div class="card stack" style="gap:12px">
        <div class="card-head" style="margin:0"><span class="h3">{t("Export")}</span></div>
        <div class="setting"><div><b>{t("Full backup (.postplant)")}</b><div class="faint">{t("Everything, including notes and settings. Never your API key.")}</div></div>
          <button class="btn btn-sm" onclick={exportBackup}><Icon name="download" />{t("Export backup")}</button></div>
        <div class="setting"><div><b>{t("Stats as JSON")}</b><div class="faint">{t("For spreadsheets and your own scripts")}</div></div>
          <div class="row" style="gap:10px"><Switch bind:checked={anonymise} label="Hide other players' names" /><Select value={String(jsonDays)} options={[["0", "All time"], ["30", "Last 30 days"], ["90", "Last 90 days"], ["365", "Last year"]]} onchange={(v) => (jsonDays = +v)} /><button class="btn btn-sm" onclick={exportJson}><Icon name="download" />{t("Export JSON")}</button></div></div>
      </div>
      <div class="card stack" style="gap:12px">
        <div class="card-head" style="margin:0"><span class="h3">{t("Import")}</span></div>
        <div class="setting"><div><b>{t("From a backup or JSON export")}</b><div class="faint">{t("You see what changes before anything is written")}</div></div>
          <button class="btn btn-sm" onclick={chooseImport}><Icon name="upload" />{t("Choose file…")}</button></div>
      </div>
      <div class="card">
        <div class="card-head"><span class="h3">{t("Automatic backups")}</span><span class="spacer"></span><button class="btn btn-sm" onclick={backupNow}>{t("Back up now")}</button></div>
        <div class="setting mb-2"><div><b>{t("Weekly backup")}</b><div class="faint">{t("Keeps the last")} {p.backups_keep}</div></div><Switch checked={p.auto_backup} onchange={(v) => savePrefs({ auto_backup: v })} /></div>
        <table class="table"><tbody>
          {#each backups as b (b.id)}<tr><td>{fmtDate(b.created_at)}</td><td class="faint">{b.kind}</td><td class="faint">{timeAgo(b.created_at)}</td><td class="faint mono">{b.size ? (b.size / 1048576).toFixed(1) + " MB" : ""}</td>
            <td style="text-align:right"><button class="btn btn-sm btn-ghost" onclick={() => { restoreId = b.id; restoreOpen = true; }}><Icon name="undo" />{t("Restore")}</button></td></tr>
          {:else}<tr><td class="empty-inline">{t("No backups yet.")}</td></tr>{/each}
        </tbody></table>
        <div class="faint mono mt-2" style="font-size:11px;overflow-wrap:anywhere">{t("Data folder:")} {app.s.data_dir}{app.s.portable ? " (portable)" : ""}</div>
      </div>
      <div class="card" style="border-color:color-mix(in srgb,var(--loss) 40%,var(--border))">
        <div class="setting"><div><b>{t("Delete all data")}</b><div class="faint">{t("Matches, notes, settings and cache on this PC. Backups in the backups folder stay.")}</div></div>
          <button class="btn btn-sm btn-danger" onclick={() => { delWord = ""; delOpen = true; }}><Icon name="trash" />{t("Delete…")}</button></div>
      </div>
    {:else if tab === "updates"}
      <div class="card stack" style="gap:16px">
        <div class="setting"><div><b>{t("Check for updates automatically")}</b><div class="faint">{t("At start and every 6 hours, one request to GitHub. Never while VALORANT is running, and an update never installs mid-match.")}</div></div>
          <Switch checked={p.update_check} onchange={(v) => savePrefs({ update_check: v })} /></div>
        <div class="row" style="gap:10px"><button class="btn btn-sm" disabled={updBusy} onclick={checkUpdate}><Icon name="refresh" />{t("Check now")}</button>
          {#if upd?.skipped}<span class="faint">{t("Skipped while the game runs.")}</span>
          {:else if upd?.newer && upd.signed}<span>{t("Version")} <b>{upd.latest}</b> {t("is ready.")}</span>
            <button class="btn btn-sm btn-primary" disabled={installing} onclick={installUpdate}><Icon name="download" />{installing ? `Installing… ${progress}%` : "Update and restart"}</button>
            <a class="btn btn-sm btn-ghost" href={upd.url} target="_blank" rel="noreferrer">{t("What's new")}</a>
          {:else if upd?.newer}<span>{t("Version")} <b>{upd.latest}</b> {t("is out.")}</span><a class="btn btn-sm btn-primary" href={upd.url} target="_blank" rel="noreferrer"><Icon name="external" />{t("Download")}</a>
          {:else if upd}<span class="faint">{t("You're on the latest version (")}{upd.current}).</span>{/if}</div>
        {#if upd?.newer && upd.notes}<details><summary class="faint" style="cursor:pointer">{t("Changes in")} {upd.latest}</summary><pre class="mono" style="white-space:pre-wrap;font-size:12px;max-height:240px;overflow:auto">{upd.notes}</pre></details>{/if}
        <p class="faint" style="margin:0;font-size:12px">{t("Updates are signed. Postplant only installs one whose signature matches the key built into this app.")}</p>
        <div class="setting"><div><b>{t("Start with Windows")}</b><div class="faint">{t("Starts minimised to the tray")}</div></div>
          <Switch checked={autostart} onchange={async (v) => (autostart = !!(await api.setAutostart(v).catch(() => !v)))} /></div>
        <div class="setting"><div><b>{t("Closing the window keeps Postplant in the tray")}</b></div>
          <Switch checked={p.close_to_tray} onchange={(v) => savePrefs({ close_to_tray: v })} /></div>
      </div>
    {:else}
      <div class="card stack" style="gap:10px">
        <div class="row" style="gap:10px"><svg width="28" height="28" style="color:var(--accent)"><use href="#logo" /></svg><b style="font-size:18px">{t("Postplant")}</b><span class="faint">v{app.s.version}</span></div>
        <p class="muted" style="margin:0">{t("Open-source Valorant stats, on your PC. Licensed MIT OR Apache-2.0.")}</p>
        <p class="muted" style="margin:0">{t("Data from the unofficial HenrikDev API using your own key. Images from valorant-api.com. Fonts: Satoshi and General Sans (Fontshare), Geist, Inter, Atkinson Hyperlegible, JetBrains Mono. Icons: Lucide.")}</p>
        <p class="notice">{t("Postplant isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games and all associated properties are trademarks or registered trademarks of Riot Games, Inc.")}</p>
        <div class="row" style="gap:8px"><a class="btn btn-sm" href="https://github.com/jeelnadaa/postplant" target="_blank" rel="noreferrer"><Icon name="external" />{t("Source code")}</a>
          <a class="btn btn-sm btn-ghost" href="https://github.com/jeelnadaa/postplant/issues" target="_blank" rel="noreferrer">{t("Report a problem")}</a></div>
      </div>
    {/if}
  </div>
</div>

<Modal bind:open={impOpen} title="Import" icon="upload" lg>
  {#if imp?.json}
    <p>{t("Matches from this JSON export are added to your history. Existing matches and notes are not changed.")}</p>
  {:else if imp}
    <div class="kv mb-4">
      <span>{t("Made")}</span><b>{fmtDate(imp.manifest?.created_at)}{imp.manifest?.app_version ? ` · v${imp.manifest.app_version}` : ""}</b>
      <span>{t("New matches")}</span><b>{imp.new_matches}</b><span>{t("Already here")}</span><b>{imp.present_matches}</b>
      <span>{t("New notes")}</span><b>{imp.new_notes}</b><span>{t("Notes that differ")}</span><b>{imp.note_conflicts}</b>
    </div>
    {#if !imp.same_owner}<div class="banner warn mb-4"><Icon name="warn" />{t("This backup belongs to a different Riot account.")}</div>{/if}
    <div class="segmented mb-4"><button aria-pressed={mode === "merge"} disabled={!imp.same_owner} title={imp.same_owner ? "" : "Merging another account's matches would mix two histories"} onclick={() => (mode = "merge")}>{t("Merge")}</button><button aria-pressed={mode === "replace"} onclick={() => (mode = "replace")}>{t("Replace everything")}</button></div>
    {#if mode === "merge" && imp.note_conflicts > 0}
      <div class="setting mb-2"><div>{t("When a note differs, keep")}</div><Select value={notes} options={[["newer", "The newer one"], ["mine", "Mine"], ["both", "Both, joined"]]} onchange={(v) => (notes = v)} /></div>
    {/if}
    <Switch bind:checked={withSettings} label="Also import settings (theme, hotkeys, overlay)" />
    {#if mode === "replace"}<div class="mt-4"><p class="faint" style="font-size:13px">{t("Your current data is backed up first. Type")} <b>{t("replace")}</b> {t("to confirm.")}</p><input class="input" bind:value={confirmWord} /></div>{/if}
  {/if}
  {#snippet foot()}
    <button class="btn btn-ghost" onclick={() => (impOpen = false)}>{t("Cancel")}</button>
    <button class="btn btn-primary" disabled={mode === "replace" && !imp?.json && confirmWord !== "replace"} onclick={applyImport}>{t("Import")}</button>
  {/snippet}
</Modal>

<Modal bind:open={restoreOpen} title="Restore this backup?" icon="undo" tone="warn">
  <p>{t("Your current data is replaced by the backup. A safety backup of the current data is made first.")}</p>
  {#snippet foot()}<button class="btn btn-ghost" onclick={() => (restoreOpen = false)}>{t("Cancel")}</button><button class="btn btn-primary" onclick={restore}>{t("Restore")}</button>{/snippet}
</Modal>

<Modal bind:open={delOpen} title="Delete all data?" icon="trash" tone="danger">
  <p>{t("This can't be undone. Export a backup first if you might want it.")}</p>
  <Switch bind:checked={delKey} label="Also remove my API key" danger />
  <p class="faint mt-4" style="font-size:13px">{t("Type")} <b>{t("delete")}</b> {t("to confirm.")}</p><input class="input" bind:value={delWord} />
  {#snippet foot()}<button class="btn btn-ghost" onclick={() => (delOpen = false)}>{t("Cancel")}</button>
    <button class="btn btn-danger" disabled={delWord !== "delete"} onclick={() => api.deleteAll(delKey).catch((e) => toast("err", "Couldn't delete", errText(e)))}>{t("Delete everything")}</button>{/snippet}
</Modal>

<HotkeyModal bind:open={hkOpen} label={hkLabel} current={p.hotkeys[hkKey] ?? ""} taken={hkTaken} onsave={saveHotkey} />

<Modal bind:open={liveWarn} title="Turn on Live match?" icon="wifi" tone="warn">
  <p>{t("Live match reads the Riot Client's local API on this PC, the same way well-known trackers do. It is not an official Riot integration.")}</p>
  <ul class="reasons">
    <li>{t("In agent select it only opens your Comp Advisor. Nothing about other players is read.")}</li>
    <li>{t("In a match it shows ranks, peaks and recent form, and only for players who show their name.")}</li>
    <li>{t("It never reads game memory, never changes the game, and your Riot tokens never leave this PC except to Riot.")}</li>
    <li>{t("Riot could change or block this at any time. Turn it off whenever you like.")}</li>
  </ul>
  <div class="mt-4"><Switch bind:checked={liveAgree} label="I understand and want to turn it on" /></div>
  {#snippet foot()}
    <span class="spacer"></span>
    <button class="btn" onclick={() => (liveWarn = false)}>{t("Not now")}</button>
    <button class="btn btn-primary" disabled={!liveAgree} onclick={acceptLive}>{t("Turn on")}</button>
  {/snippet}
</Modal>
