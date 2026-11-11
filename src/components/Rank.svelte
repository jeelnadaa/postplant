<script lang="ts">
  import { tierClass, tierName } from "../lib/format";
  let { tier, rr = null, short = false, lg = false }: { tier: number | null | undefined; rr?: number | null; short?: boolean; lg?: boolean } = $props();
</script>

{#if tier && tier >= 3}
  <span class="rank t-{tierClass(tier)} {lg ? 'rank-lg' : ''}"><i class="rank-gem"></i>{tierName(tier, short)}{#if rr !== null && rr !== undefined}&nbsp;<span class="rr">{rr}</span>{/if}</span>
{:else}
  <span class="faint">Unranked</span>
{/if}
