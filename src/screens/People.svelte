<script lang="ts">
  import { api, errText, type J } from "../lib/api";
  import { timeAgo } from "../lib/format";
  import { app, nav } from "../lib/store.svelte";
  import Icon from "../components/Icon.svelte";
  import NoteModal from "../components/NoteModal.svelte";
  import PlayerName from "../components/PlayerName.svelte";

  let d = $state<J>(null);
  let error = $state("");
  let filter = $state("all");
  let q = $state("");
  let noteFor = $state<J>(null);
  let noteOpen = $state(false);
  const load = () => api.people().then((r) => (d = r)).catch((e) => (error = errText(e)));
  $effect(() => {
    void app.version;
    load();
  });
  const pct = (w: number, g: number) => (g ? `${Math.round((100 * w) / g)}%` : "—");
  const list = $derived(
    (d?.people ?? []).filter((p: J) => {
      if (filter === "duo" && !p.party) return false;
      if (filter === "notes" && !p.note) return false;
      if (q && !`${p.name}#${p.tag}`.toLowerCase().includes(q.toLowerCase())) return false;
      return true;
    }),
  );
</script>

<div class="page-head"><div><h1 class="h1">People</h1><div class="sub">Players you've met more than once, and everyone you've noted. Notes stay on this PC.</div></div></div>
<div class="row mb-4" style="gap:8px;flex-wrap:wrap">
  <div class="segmented">
    {#each [["all", "Everyone"], ["duo", "Played in party"], ["notes", "With notes"]] as [v, l] (v)}<button aria-pressed={filter === v} onclick={() => (filter = v)}>{l}</button>{/each}
  </div>
  <span class="spacer"></span>
  <div class="input-icon" style="width:240px"><Icon name="search" /><input class="input" placeholder="Filter by name" bind:value={q} /></div>
</div>
{#if error}<div class="banner danger"><Icon name="alert" />{error}</div>{/if}
{#if d}
  <table class="table">
    <thead><tr><th>Player</th><th class="num">Games</th><th class="num">With you · win</th><th class="num">Against · your win</th><th>Last seen</th><th>Note</th><th></th></tr></thead>
    <tbody>
      {#each list as p (p.puuid)}
        <tr>
          <td><div class="row" style="gap:6px"><button class="row" title="Look up" onclick={() => p.name && nav("search", { name: p.name, tag: p.tag })}><PlayerName name={p.name} tag={p.tag} /></button>{#if p.party}<span class="tag good">party ×{p.party}</span>{/if}</div></td>
          <td class="num">{p.games}</td>
          <td class="num">{p.with} · {pct(p.with_wins, p.with)}</td>
          <td class="num">{p.vs} · {pct(p.vs_wins, p.vs)}</td>
          <td class="faint">{timeAgo(p.last)}</td>
          <td style="max-width:260px">{#if p.note}{#each p.note.tags as t (t)}<span class="tag" style="margin-right:4px">{t}</span>{/each}<span class="muted" style="font-size:12px">{p.note.note}</span>{/if}</td>
          <td><button class="btn btn-icon btn-ghost btn-sm" aria-label="Note" title="Note" onclick={() => { noteFor = p; noteOpen = true; }}><Icon name="note" /></button></td>
        </tr>
      {:else}<tr><td colspan="7" class="empty-inline">Nobody here yet. People appear once you've met them twice.</td></tr>{/each}
    </tbody>
  </table>
{/if}
{#if noteFor}<NoteModal bind:open={noteOpen} puuid={noteFor.puuid} who="{noteFor.name}#{noteFor.tag}" note={noteFor.note} onsaved={load} />{/if}
