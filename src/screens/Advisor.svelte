<script lang="ts">
  import { api, errText, type J } from "../lib/api";
  import { app, loadKb, toast } from "../lib/store.svelte";
  import Agent from "../components/Agent.svelte";
  import Icon from "../components/Icon.svelte";
  import Modal from "../components/Modal.svelte";
  import StackPlanner from "../components/StackPlanner.svelte";

  let { params = {} }: { params?: Record<string, string> } = $props();
  // svelte-ignore state_referenced_locally
  let tab = $state(params.tab ?? "pick");
  let map = $state("");
  let team = $state<string[]>([]);
  let res = $state<J>(null);
  let learn = $state<J>(null);
  let picker = $state(false);
  let error = $state("");

  const maps = $derived((app.kb?.maps ?? []).filter((m: J) => m.in_pool));
  const agents = $derived(app.kb?.agents ?? []);
  const owned = $derived(new Set<string>(app.kb?.owned ?? []));
  const byName = (n: string) => agents.find((a: J) => a.name === n);

  $effect(() => {
    if (!map && maps.length) map = maps[0].name;
  });
  $effect(() => {
    if (!map) return;
    const t = [...team];
    error = "";
    api.suggest(map, t).then((r) => (res = r)).catch((e) => (error = errText(e)));
    api.learn(map).then((r) => (learn = r)).catch(() => (learn = null));
  });

  function add(name: string) {
    if (team.length < 4 && !team.includes(name)) team = [...team, name];
    if (team.length >= 4) picker = false;
  }
  async function toggleOwned(a: J) {
    try {
      await api.setOwned(a.id, !owned.has(a.id));
      await loadKb();
    } catch (e) {
      toast("err", "Couldn't save", errText(e));
    }
  }
</script>

<div class="page-head">
  <div><h1 class="h1">Comp Advisor</h1><div class="sub">Plan a pick from role and utility coverage. With Live match on, it also opens in the overlay during agent select. Hotkey: {app.s?.prefs?.hotkeys?.advisor || "off"}</div></div>
</div>
<div class="tabs mb-4">
  <button role="tab" class="tab" aria-selected={tab === "pick"} onclick={() => (tab = "pick")}>Plan a pick</button>
  <button role="tab" class="tab" aria-selected={tab === "stack"} onclick={() => (tab = "stack")}>Stack planner</button>
  <button role="tab" class="tab" aria-selected={tab === "learn"} onclick={() => (tab = "learn")}>Learn for this map</button>
  <button role="tab" class="tab" aria-selected={tab === "pool"} onclick={() => (tab = "pool")}>My agents<span class="count">{owned.size}</span></button>
</div>

{#if tab !== "pool"}
  <div class="row mb-4" style="flex-wrap:wrap;gap:8px">
    {#each maps as m (m.name)}<button class="chip" aria-pressed={map === m.name} onclick={() => (map = m.name)}><Icon name="map" />{m.name}</button>{/each}
  </div>
{/if}
{#if error}<div class="banner danger mb-4"><Icon name="alert" />{error}</div>{/if}

{#if tab === "pick"}
  <div class="grid-12">
    <div class="card col-12">
      <div class="card-head"><span class="h3">Teammates' picks so far</span><span class="spacer"></span>{#if team.length}<button class="link-btn" onclick={() => (team = [])}>Clear</button>{/if}</div>
      <div class="row" style="gap:10px;flex-wrap:wrap">
        {#each team as n (n)}
          <div class="chip" style="padding-right:4px"><Agent name={n} id={byName(n)?.id} sm />{n}<button class="btn btn-icon btn-ghost btn-sm" aria-label="Remove {n}" onclick={() => (team = team.filter((x) => x !== n))}><Icon name="x" /></button></div>
        {/each}
        {#if team.length < 4}<button class="btn btn-sm" onclick={() => (picker = true)}><Icon name="plus" />Add a teammate's agent</button>{/if}
      </div>
      <p class="faint" style="font-size:12px;margin:10px 0 0">Type them in yourself. Postplant never reads agent select.</p>
    </div>
    {#if res}
      {#if res.notes}<div class="note-insight col-12"><span class="ic"><Icon name="map-pin" /></span><div style="font-size:13px">{res.notes}</div><span></span></div>{/if}
      <div class="col-7 stack" style="gap:10px">
        <div class="h3">Suggested for you</div>
        {#each res.agents as a, i (a.name)}
          <div class="card" style="display:flex;gap:14px;align-items:flex-start;padding:14px">
            <Agent name={a.name} id={a.id} />
            <div style="flex:1;min-width:0">
              <div class="row" style="gap:8px"><b style="font-size:15px">{i + 1}. {a.name}</b><span class="tag">{a.role}</span>{#if a.limited}<span class="tag warnish">limited data</span>{/if}</div>
              <ul class="reasons">{#each a.reasons as r (r)}<li>{r}</li>{/each}</ul>
            </div>
            <span class="faint" style="font-size:11px;white-space:nowrap">{a.tier} · {a.confidence}</span>
          </div>
        {:else}<div class="card empty-inline">No owned agents left to suggest. Mark agents you own in My agents.</div>{/each}
      </div>
      <div class="col-5 stack" style="gap:10px">
        <div class="h3">Full comps</div>
        {#each res.comps as c, ci (ci)}
          <div class="card" style="padding:14px">
            <div class="row" style="gap:6px">{#each c.agents as n (n)}<Agent name={n} id={byName(n)?.id} sm picked={n === c.you} title={n === c.you ? `${n} (you)` : n} />{/each}</div>
            <div class="faint" style="font-size:12px;margin-top:8px">{c.roles}{#if c.you} · you: <b>{c.you}</b>{/if}</div>
          </div>
        {/each}
      </div>
      <p class="notice col-12">{res.footer}</p>
    {/if}
  </div>
{:else if tab === "stack"}
  <StackPlanner {map} />
{:else if tab === "learn"}
  {#if learn}
    <p class="muted mb-4">Agents outside your regulars ({learn.regular.length ? learn.regular.join(", ") : "none yet"}) that fit {learn.map}.</p>
    <div class="grid-12">
      {#each learn.picks as a (a.name)}
        <div class="card col-6" style="display:flex;gap:14px;align-items:flex-start">
          <Agent name={a.name} id={a.id} locked={!a.owned} />
          <div><div class="row" style="gap:8px"><b style="font-size:15px">{a.name}</b><span class="tag">{a.role}</span>{#if !a.owned}<span class="tag warnish">not unlocked</span>{/if}</div>
            <ul class="reasons">{#each a.reasons as r (r)}<li>{r}</li>{/each}</ul></div>
        </div>
      {/each}
    </div>
  {/if}
{:else}
  <div class="card">
    <div class="card-head"><span class="h3">Agents you own</span><span class="spacer"></span><span class="faint" style="font-size:12px">Filled in from your matches. Click to correct.</span></div>
    {#each ["Duelist", "Initiator", "Controller", "Sentinel"] as role (role)}
      <div class="label mt-4 mb-2">{role}s</div>
      <div class="row" style="gap:8px;flex-wrap:wrap">
        {#each agents.filter((a: J) => a.role === role) as a (a.id)}
          <Agent name={a.name} id={a.id} pressed={owned.has(a.id)} locked={!owned.has(a.id)} title="{a.name}: {owned.has(a.id) ? 'owned' : 'not owned'}" onclick={() => toggleOwned(a)} />
        {/each}
      </div>
    {/each}
  </div>
{/if}

<Modal bind:open={picker} title="Add a teammate's agent" icon="users">
  <div class="row" style="gap:8px;flex-wrap:wrap">
    {#each agents.filter((a: J) => !team.includes(a.name)) as a (a.id)}
      <Agent name={a.name} id={a.id} title={a.name} onclick={() => add(a.name)} />
    {/each}
  </div>
</Modal>
