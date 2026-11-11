<script lang="ts">
  // Stack planner (plan 14.3): you plus up to four friends; each gets an agent from their own pool.
  import { api, errText, type J } from "../lib/api";
  import { app } from "../lib/store.svelte";
  import Agent from "./Agent.svelte";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";

  let { map }: { map: string } = $props();
  type P = { name: string; puuid?: string; me?: boolean; pool: string[] };
  let people = $state<J[]>([]);
  let players = $state<P[]>([{ name: "You", me: true, pool: [] }]);
  let res = $state<J>(null);
  let error = $state("");
  let editing = $state<number | null>(null);
  let poolOpen = $state(false);
  let addName = $state("");

  api.stackPeople().then((r) => (people = r)).catch(() => {});
  // Saved stack from last time.
  // svelte-ignore state_referenced_locally
  const saved = app.s?.prefs?.stack;
  if (Array.isArray(saved) && saved.length) players = [{ name: "You", me: true, pool: [] }, ...saved.filter((p: P) => !p.me).slice(0, 4)];

  const agents = $derived(app.kb?.agents ?? []);
  const byName = (n: string) => agents.find((a: J) => a.name === n);

  $effect(() => {
    const ps = JSON.parse(JSON.stringify(players));
    if (!map) return;
    error = "";
    api.stack(map, ps).then((r) => (res = r)).catch((e) => { res = null; error = errText(e); });
    api.savePrefs({ stack: ps.filter((p: P) => !p.me) }).catch(() => {});
  });

  function addFriend(p: J) {
    if (players.length >= 5) return;
    players = [...players, { name: p.name, puuid: p.puuid, pool: p.agents.map((a: J) => a.name) }];
  }
  function addCustom() {
    if (!addName.trim() || players.length >= 5) return;
    players = [...players, { name: addName.trim(), pool: [] }];
    addName = "";
    editing = players.length - 1;
    poolOpen = true;
  }
  function togglePool(n: string) {
    if (editing === null) return;
    const p = players[editing];
    const pool = p.pool.includes(n) ? p.pool.filter((x) => x !== n) : [...p.pool, n];
    players = players.map((x, i) => (i === editing ? { ...x, pool } : x));
  }
  const available = $derived(people.filter((p: J) => !players.some((x) => x.puuid === p.puuid)));
</script>

<div class="grid-12">
  <div class="card col-12">
    <div class="card-head"><span class="h3">Your stack</span><span class="faint" style="font-size:12px;margin-left:8px">{players.length} / 5</span></div>
    <div class="stack" style="gap:8px">
      {#each players as p, i (p.name + i)}
        <div class="list-row" style="cursor:default">
          <Icon name={p.me ? "user" : "users"} /><b>{p.name}</b>
          <span class="row" style="gap:3px;flex-wrap:wrap">{#each p.pool.slice(0, 8) as n (n)}<Agent name={n} id={byName(n)?.id} sm />{/each}{#if p.me && !p.pool.length}<span class="faint" style="font-size:12px">your owned agents</span>{:else if !p.pool.length}<span class="faint" style="font-size:12px">any agent</span>{/if}</span>
          <span class="spacer"></span>
          <button class="btn btn-sm btn-ghost" onclick={() => { editing = i; poolOpen = true; }}><Icon name="pencil" />Pool</button>
          {#if !p.me}<button class="btn btn-icon btn-sm btn-ghost" aria-label="Remove {p.name}" onclick={() => (players = players.filter((_, j) => j !== i))}><Icon name="x" /></button>{/if}
        </div>
      {/each}
    </div>
    {#if players.length < 5}
      <div class="row mt-4" style="gap:8px;flex-wrap:wrap">
        {#each available.slice(0, 8) as p (p.puuid)}<button class="chip" onclick={() => addFriend(p)} title="{p.games} games together"><Icon name="plus" />{p.name}</button>{/each}
        <form class="row" style="gap:6px" onsubmit={(e) => { e.preventDefault(); addCustom(); }}><input class="input" style="width:160px" placeholder="Someone else" bind:value={addName} /><button class="btn btn-sm">Add</button></form>
      </div>
      <p class="faint" style="font-size:12px;margin:8px 0 0">People you've queued with come with the agents they play and their record on this map. Add anyone else by name and pick their agents.</p>
    {/if}
  </div>
  {#if error}<div class="banner danger col-12"><Icon name="alert" />{error}</div>{/if}
  {#if res}
    {#each res.options as o, i (i)}
      <div class="card col-4 stack-opt" style="--i:{i}">
        <div class="card-head"><span class="h3">Option {i + 1}</span><span class="spacer"></span><span class="badge {i === 0 ? 'accent' : ''}">{o.score}</span></div>
        {#each o.assign as a (a.player)}
          <div class="row" style="gap:10px;padding:4px 0"><Agent name={a.agent} id={a.id} sm /><div style="min-width:0"><b>{a.agent}</b> <span class="faint">· {a.player}</span>
            {#if a.record}<div class="faint" style="font-size:11px">{a.record.wins}W {a.record.games - a.record.wins}L {a.record.on_map ? `on ${res.map}` : "overall"}</div>{/if}</div></div>
        {/each}
        <ul class="reasons">{#each o.reasons as r (r)}<li>{r}</li>{/each}</ul>
      </div>
    {/each}
    <p class="notice col-12">{res.footer}</p>
  {/if}
</div>

<Modal bind:open={poolOpen} title="Agents {editing !== null ? players[editing]?.name : ''} plays" icon="users">
  {#if editing !== null}
    {#each ["Duelist", "Initiator", "Controller", "Sentinel"] as role (role)}
      <div class="label mt-2 mb-2">{role}s</div>
      <div class="row" style="gap:6px;flex-wrap:wrap">
        {#each agents.filter((a: J) => a.role === role) as a (a.id)}
          <Agent name={a.name} id={a.id} pressed={players[editing].pool.includes(a.name)} locked={!players[editing].pool.includes(a.name)} title={a.name} onclick={() => togglePool(a.name)} />
        {/each}
      </div>
    {/each}
    {#if players[editing].me}<p class="faint mt-4" style="font-size:12px">Leave empty to use every agent you own.</p>{/if}
  {/if}
</Modal>

<style>
  .stack-opt { transition: transform 0.18s ease, box-shadow 0.18s ease; animation: opt-in 0.25s ease both; animation-delay: calc(var(--i) * 60ms); }
  .stack-opt:hover { transform: translateY(-2px); box-shadow: var(--shadow-2); }
  @keyframes opt-in { from { opacity: 0; transform: translateY(6px); } }
  @media (prefers-reduced-motion: reduce) { .stack-opt { animation: none; } }
</style>
