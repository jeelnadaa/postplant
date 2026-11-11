<script lang="ts">
  // Ctrl+K: jump to a screen or look up a Riot ID (plan section 26).
  import { parseRiotId } from "../lib/format";
  import { app, nav } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";

  const SCREENS: [string, string, string][] = [
    ["home", "house", "Home"], ["matches", "swords", "Matches"], ["analytics", "chart", "Analytics"], ["habits", "clock", "When you play best"],
    ["rank", "trending-up", "Rank"], ["advisor", "compass", "Comp Advisor"], ["search", "search", "Search players"],
    ["people", "users", "People & notes"], ["review", "bookmark", "Review list"], ["settings", "settings", "Settings"],
    ["settings?tab=data", "database", "Export or import data"], ["settings?tab=api", "key", "API key & usage"],
    ["settings?tab=overlay", "layers", "Overlay & hotkeys"],
  ];
  let q = $state("");
  let sel = $state(0);
  let input: HTMLInputElement | undefined = $state();
  const rid = $derived(parseRiotId(q));
  const items = $derived([
    ...(rid ? [["search", "search", `Look up ${rid.name}#${rid.tag}`, rid] as const] : []),
    ...SCREENS.filter(([, , l]) => l.toLowerCase().includes(q.toLowerCase().trim())).map((s) => [...s, null] as const),
  ]);
  $effect(() => {
    if (app.palette) {
      q = "";
      sel = 0;
      setTimeout(() => input?.focus(), 10);
    }
  });
  function go(i: number) {
    const it = items[i];
    if (!it) return;
    app.palette = false;
    const [route, , , r] = it;
    if (r) nav("search", { name: r.name, tag: r.tag });
    else {
      const [n, qs] = route.split("?");
      nav(n, Object.fromEntries(new URLSearchParams(qs ?? "")));
    }
  }
  function key(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      app.palette = !app.palette;
    }
    if (!app.palette) return;
    if (e.key === "Escape") app.palette = false;
    if (e.key === "ArrowDown") { e.preventDefault(); sel = Math.min(items.length - 1, sel + 1); }
    if (e.key === "ArrowUp") { e.preventDefault(); sel = Math.max(0, sel - 1); }
    if (e.key === "Enter") go(sel);
  }
</script>

<svelte:window onkeydown={key} />
{#if app.palette}
  <div class="scrim open" role="presentation" style="align-items:start;padding-top:12vh" onclick={(e) => e.target === e.currentTarget && (app.palette = false)}>
    <div class="modal" role="dialog" aria-label="Command palette" style="width:min(560px,100%)">
      <div style="padding:12px">
        <div class="input input-lg"><Icon name="search" /><input bind:this={input} bind:value={q} placeholder="Search screens or type name#tag" oninput={() => (sel = 0)} /></div>
      </div>
      <div class="cmd-list" style="padding:0 8px 8px">
        {#each items as it, i (it[2])}
          <button class="menu-item {i === sel ? 'is-active' : ''}" onmouseenter={() => (sel = i)} onclick={() => go(i)}><Icon name={it[1]} />{it[2]}</button>
        {:else}
          <div class="empty-inline">Nothing matches. Type a Riot ID like Name#tag to look someone up.</div>
        {/each}
      </div>
    </div>
  </div>
{/if}
