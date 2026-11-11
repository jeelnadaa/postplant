<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import { api, errText, type J } from "../lib/api";
  import { timeAgo } from "../lib/format";
  import { app, nav } from "../lib/store.svelte";
  import Agent from "../components/Agent.svelte";
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
  let duos = $state<J[]>([]);
  $effect(() => {
    void app.version;
    api.duos().then((r) => (duos = r)).catch(() => {});
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

<div class="page-head"><div><h1 class="h1">{t("People")}</h1><div class="sub">{t("Players you've met more than once, and everyone you've noted. Notes stay on this PC.")}</div></div></div>
<div class="row mb-4" style="gap:8px;flex-wrap:wrap">
  <div class="segmented">
    {#each [["all", "Everyone"], ["duo", "Played in party"], ["notes", "With notes"]] as [v, l] (v)}<button aria-pressed={filter === v} onclick={() => (filter = v)}>{t(l)}</button>{/each}
  </div>
  <span class="spacer"></span>
  <div class="input-icon" style="width:240px"><Icon name="search" /><input class="input" placeholder="Filter by name" bind:value={q} /></div>
</div>
{#if error}<div class="banner danger"><Icon name="alert" />{error}</div>{/if}
{#if d}
  <table class="table">
    <thead><tr><th>{t("Player")}</th><th class="num">{t("Games")}</th><th class="num">{t("With you · win")}</th><th class="num">{t("Against · your win")}</th><th>{t("Last seen")}</th><th>{t("Note")}</th><th></th></tr></thead>
    <tbody>
      {#each list as p (p.puuid)}
        <tr>
          <td><div class="row" style="gap:6px"><button class="row" title="Look up" onclick={() => p.name && nav("search", { name: p.name, tag: p.tag })}><PlayerName name={p.name} tag={p.tag} /></button>{#if p.party}<span class="tag good">{t("party ×")}{p.party}</span>{/if}</div></td>
          <td class="num">{p.games}</td>
          <td class="num">{p.with} · {pct(p.with_wins, p.with)}</td>
          <td class="num">{p.vs} · {pct(p.vs_wins, p.vs)}</td>
          <td class="faint">{timeAgo(p.last)}</td>
          <td style="max-width:260px">{#if p.note}{#each p.note.tags as t (t)}<span class="tag" style="margin-right:4px">{t}</span>{/each}<span class="muted" style="font-size:12px">{p.note.note}</span>{/if}</td>
          <td><button class="btn btn-icon btn-ghost btn-sm" aria-label="Note" title="Note" onclick={() => { noteFor = p; noteOpen = true; }}><Icon name="note" /></button></td>
        </tr>
      {:else}<tr><td colspan="7" class="empty-inline">{t("Nobody here yet. People appear once you've met them twice.")}</td></tr>{/each}
    </tbody>
  </table>
{/if}
{#if duos.length}
  <div class="h3 mt-6 mb-2" style="margin-top:24px">{t("Duo synergy")}</div>
  <div class="grid-12">
    {#each duos.slice(0, 6) as f (f.puuid)}
      <div class="card col-6">
        <div class="card-head"><b>{f.name}</b><span class="spacer"></span><span class="badge {f.win >= 50 ? 'win' : 'loss'}">{f.win}%</span><span class="faint" style="font-size:12px;margin-left:6px">{f.wins}W / {f.games}</span></div>
        <table class="table"><thead><tr><th>{t("You")}</th><th>{t("Them")}</th><th class="num">{t("Games")}</th><th class="num">{t("Win")}</th></tr></thead><tbody>
          {#each f.pairs as p (p.mine + p.theirs)}<tr><td><div class="player"><Agent name={p.mine} sm />{p.mine}</div></td><td><div class="player"><Agent name={p.theirs} sm />{p.theirs}</div></td><td class="num">{p.games}</td><td class="num">{p.win}%</td></tr>{/each}
        </tbody></table>
      </div>
    {/each}
  </div>
{/if}
{#if noteFor}<NoteModal bind:open={noteOpen} puuid={noteFor.puuid} who="{noteFor.name}#{noteFor.tag}" note={noteFor.note} onsaved={load} />{/if}
