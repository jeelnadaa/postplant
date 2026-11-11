<script lang="ts">
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, errText, type J } from "../lib/api";
  import { parseRiotId, REGIONS } from "../lib/format";
  import { app, loadKb, nav, refreshState, savePrefs, toast } from "../lib/store.svelte";
  import Agent from "../components/Agent.svelte";
  import Icon from "../components/Icon.svelte";
  import Select from "../components/Select.svelte";

  const STEPS = ["Welcome", "API key", "Your Riot ID", "Agents you own", "Overlay check"];
  let step = $state(app.s?.has_key ? (app.s?.me ? 3 : 2) : 0);
  let key = $state("");
  let showKey = $state(false);
  let busy = $state(false);
  let keyOk = $state<J>(null);
  let rid = $state(app.s?.me ? `${app.s.me.name}#${app.s.me.tag}` : "");
  let found = $state<J>(app.s?.me ?? null);
  let region = $state(app.s?.me?.region ?? "ap");
  let error = $state("");

  const THEMES = [["ink", "Ink"], ["paper", "Paper"], ["midnight", "Midnight"], ["nebula", "Nebula"], ["frost", "Frost"], ["sandstone", "Sandstone"]];

  async function testKey() {
    error = "";
    busy = true;
    try {
      keyOk = await api.setKey(key);
      await refreshState();
    } catch (e) {
      error = errText(e);
    }
    busy = false;
  }
  async function lookup() {
    error = "";
    const p = parseRiotId(rid);
    if (!p) return (error = "Use the form Name#tag, for example Nova#ap1.");
    busy = true;
    try {
      found = await api.lookup(p.name, p.tag);
      if (found.region) region = found.region;
    } catch (e) {
      error = errText(e);
      found = null;
    }
    busy = false;
  }
  async function confirmMe() {
    busy = true;
    try {
      await api.setMe({ puuid: found.puuid, name: found.name, tag: found.tag, region, account_level: found.account_level ?? 0 });
      await refreshState();
      await loadKb();
      step = 3;
    } catch (e) {
      error = errText(e);
    }
    busy = false;
  }
  async function toggleOwned(a: J) {
    const owned = app.kb.owned.includes(a.id);
    await api.setOwned(a.id, !owned);
    await loadKb();
  }
  async function restore() {
    const path = await openDialog({ filters: [{ name: "Postplant backup", extensions: ["postplant"] }] });
    if (!path) return;
    busy = true;
    try {
      const info = await api.inspectImport(path as string);
      const m = info.manifest;
      if (!confirm(`Restore ${m.counts?.matches ?? 0} matches from ${m.owner?.name ?? "this backup"} (made ${new Date(m.created_at).toLocaleDateString()})? This costs no API requests.`)) return;
      await api.applyImport(path as string, "replace", "newer", true);
      toast("ok", "Backup restored", "Your history is back. You still need your API key on this PC.");
      await refreshState();
      step = app.s?.has_key ? 3 : 1;
    } catch (e) {
      toast("err", "Couldn't restore", errText(e));
    } finally {
      busy = false;
    }
  }
  function finish() {
    nav("home");
    api.sync().catch(() => {});
  }
</script>

<div class="window" style="height:100vh;border:0;border-radius:0;grid-template-rows:1fr">
  <div class="onb" style="min-height:0">
    <div class="onb-rail">
      <span class="logo" style="font-size:20px;margin-bottom:22px"><svg viewBox="0 0 48 48" style="width:26px;height:26px"><use href="#logo" /></svg>Postplant</span>
      {#each STEPS as s, k (s)}
        <div class="onb-step {k < step ? 'done' : k === step ? 'on' : ''}"><i class="n">{k < step ? "✓" : k + 1}</i>{s}</div>
      {/each}
      <div style="margin-top:auto" class="faint"><div class="caps">Private by design</div><p style="font-size:12px;margin:6px 0 0">Your data stays on this PC. No account, no telemetry.</p></div>
    </div>
    <div class="onb-main">
      {#if step === 0}
        <span class="caps">Welcome</span>
        <h1 class="display" style="font-size:56px;margin:10px 0">Your matches,<br /><span class="accent">after the spike.</span></h1>
        <p class="muted" style="max-width:520px">Postplant tracks your Valorant matches, finds patterns, and shows a small overlay after each game. It runs on your own free HenrikDev API key and keeps everything on this PC.</p>
        <div class="caps" style="margin-top:20px">Theme</div>
        <div class="row wrap" style="gap:6px;margin-top:8px">
          {#each THEMES as [id, l] (id)}<button class="chip" aria-pressed={app.s?.prefs?.theme === id} onclick={() => savePrefs({ theme: id })}>{l}</button>{/each}
        </div>
        <div class="onb-foot">
          <button class="btn" onclick={restore} disabled={busy}><Icon name="upload" />Restore from a backup</button>
          <button class="btn btn-primary" onclick={() => (step = 1)}>Get started<Icon name="arrow-right" /></button>
        </div>
      {:else if step === 1}
        <h1 class="h1">Your HenrikDev API key</h1>
        <p class="muted">Postplant reads match data through the free HenrikDev API. Each person uses their own key; it's stored in Windows Credential Manager, never in files or logs.</p>
        <div style="display:grid;grid-template-columns:1fr 320px;gap:28px;margin-top:12px">
          <div class="stack" style="gap:12px">
            <div class="field">
              <span class="label">API key</span>
              <div class="input {error ? 'is-error' : keyOk ? 'is-ok' : ''}"><Icon name="key" />
                <input type={showKey ? "text" : "password"} placeholder="HDEV-…" bind:value={key} onkeydown={(e) => e.key === "Enter" && testKey()} autocomplete="off" spellcheck="false" />
                <button class="btn btn-icon btn-ghost btn-sm" aria-label="Show key" onclick={() => (showKey = !showKey)}><Icon name={showKey ? "eye-off" : "eye"} /></button>
              </div>
              {#if error}<span class="hint error">{error}</span>{:else if keyOk}<span class="hint ok">Key works · {keyOk.limit} requests per minute</span>{/if}
            </div>
            <div class="row"><button class="btn btn-primary {busy ? 'is-loading' : ''}" onclick={testKey} disabled={!key || busy}>Test key</button></div>
          </div>
          <div class="card flat">
            <div class="h3" style="font-size:14px;margin-bottom:10px">How do I get a key?</div>
            <ol class="guide" style="padding:0;margin:0">
              <li>Open the HenrikDev docs and join their Discord from there.</li>
              <li>Use their key bot to create a <b>Basic</b> key: free and instant, 30 requests a minute. That's enough for Postplant.</li>
              <li>Paste it here and press Test key.</li>
            </ol>
            <button class="btn btn-sm mt-4" onclick={() => openUrl("https://docs.henrikdev.xyz")}><Icon name="external" />Open docs.henrikdev.xyz</button>
            <p class="faint" style="font-size:12px;margin:12px 0 0">Why not a Riot key? Riot doesn't issue personal VALORANT keys.</p>
          </div>
        </div>
        <div class="onb-foot"><button class="btn" onclick={() => (step = 0)}>Back</button><button class="btn btn-primary" disabled={!app.s?.has_key} onclick={() => { error = ""; step = 2; }}>Continue<Icon name="arrow-right" /></button></div>
      {:else if step === 2}
        <h1 class="h1">Your Riot ID</h1>
        <p class="muted">Used once to find your account. Postplant keys everything by your account id, so name changes don't break history.</p>
        <div class="row" style="gap:10px;margin-top:12px;align-items:flex-end">
          <div class="field" style="flex:1;max-width:420px"><span class="label">Riot ID</span>
            <div class="input input-lg {error ? 'is-error' : ''}"><Icon name="user" /><input placeholder="Name#tag" bind:value={rid} onkeydown={(e) => e.key === "Enter" && lookup()} /></div></div>
          <button class="btn btn-lg {busy ? 'is-loading' : ''}" onclick={lookup} disabled={busy}>Find</button>
        </div>
        {#if error}<p class="hint error">{error}</p>{/if}
        {#if found}
          <div class="card hl mt-4" style="max-width:560px;display:flex;align-items:center;gap:14px">
            <span class="avatar" style="width:44px;height:44px;font-size:18px">{found.name[0]}</span>
            <div style="flex:1"><div class="h3">{found.name}<span class="faint">#{found.tag}</span></div><div class="faint" style="font-size:12px">Level {found.account_level ?? "?"} · region detected from your account</div></div>
            <Select bind:value={region} options={REGIONS.map(([v, l]) => [v, l] as [string, string])} minWidth={170} />
          </div>
        {/if}
        <div class="onb-foot"><button class="btn" onclick={() => (step = 1)}>Back</button><button class="btn btn-primary" disabled={!found || busy} onclick={confirmMe}>This is me<Icon name="arrow-right" /></button></div>
      {:else if step === 3}
        <h1 class="h1">Agents you own</h1>
        <p class="muted">Pre-filled with the starter agents and everything in your match history. The Comp Advisor never suggests an agent you can't pick. Click to change.</p>
        {#if app.kb}
          <div class="agent-grid mt-4">
            {#each app.kb.agents as a (a.id)}
              <Agent name={a.name} id={a.id} locked={!app.kb.owned?.includes(a.id)} pressed={app.kb.owned?.includes(a.id)} onclick={() => toggleOwned(a)} title={app.kb.owned?.includes(a.id) ? `${a.name}: owned` : `${a.name}: not on your account; click to mark as owned`} />
            {/each}
          </div>
          <p class="faint mt-2" style="font-size:12px">{app.kb.owned?.length ?? 0} of {app.kb.agents.length} owned</p>
        {/if}
        <div class="onb-foot"><button class="btn" onclick={() => (step = 2)}>Back</button><button class="btn btn-primary" onclick={() => (step = 4)}>Continue<Icon name="arrow-right" /></button></div>
      {:else}
        <h1 class="h1">Overlay check</h1>
        <p class="muted">The overlay shows your last match, session and rank, and the scoreboard after each game. It's click-through, so it never steals your mouse.</p>
        <div class="callout warn mt-4"><b>Play in Windowed Fullscreen.</b> Overlays can't draw over exclusive fullscreen. In Valorant: Settings → Video → Display mode → Windowed Fullscreen.</div>
        <div class="kv mt-4" style="max-width:420px">
          <span>Show / hide overlay</span><span class="keys"><span class="kbd">Ctrl</span><span class="kbd">Shift</span><span class="kbd">H</span></span>
          <span>Move it (edit mode)</span><span class="keys"><span class="kbd">Ctrl</span><span class="kbd">Shift</span><span class="kbd">M</span></span>
          <span>Open Postplant</span><span class="keys"><span class="kbd">Ctrl</span><span class="kbd">Shift</span><span class="kbd">O</span></span>
        </div>
        <p class="faint" style="font-size:12px">Hotkeys work while VALORANT is running, so they don't take shortcuts away from other apps. Change this in Settings.</p>
        <div class="row mt-2"><button class="btn" onclick={() => api.overlay("toggle_overlay")}><Icon name="layers" />Show / hide the overlay now</button></div>
        <div class="onb-foot"><button class="btn" onclick={() => (step = 3)}>Back</button><button class="btn btn-primary" onclick={finish}>Finish<Icon name="check" /></button></div>
      {/if}
    </div>
  </div>
</div>
