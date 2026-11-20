<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import type { J } from "../lib/api";
  let { players }: { players: J[] } = $props();
</script>

<ul aria-label={t("Team selections")}>
  {#each players as player}
    <li><span title={player.name}>{player.name}</span><b>{player.agent ?? t("Choosing…")}</b>
      <small>{player.agent ? t(player.locked ? "Locked" : "Hovering") : ""}</small></li>
  {/each}
</ul>

<style>
  ul { display: grid; gap: 4px; padding: 0; margin: 4px 0 8px; list-style: none; }
  li { display: grid; grid-template-columns: minmax(0, 1fr) auto 5em; align-items: baseline; gap: 8px; font-size: 12px; }
  li span { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  small { color: var(--muted); }
</style>
