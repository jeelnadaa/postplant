<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  let {
    open = $bindable(false), title, sub = "", icon = "info", tone = "", lg = false, children, foot,
  }: { open: boolean; title: string; sub?: string; icon?: string; tone?: string; lg?: boolean; children: Snippet; foot?: Snippet } = $props();

  function key(e: KeyboardEvent) {
    if (open && e.key === "Escape") open = false;
  }
</script>

<svelte:window onkeydown={key} />
{#if open}
  <div class="scrim open" role="presentation" onclick={(e) => e.target === e.currentTarget && (open = false)}>
    <div class="modal {lg ? 'modal-lg' : ''}" role="dialog" aria-modal="true" aria-label={title}>
      <div class="modal-head">
        <span class="modal-icon {tone}"><Icon name={icon} /></span>
        <div><div class="h2">{title}</div>{#if sub}<div class="sub">{sub}</div>{/if}</div>
        <button class="btn btn-icon btn-ghost btn-sm close" aria-label="Close" onclick={() => (open = false)}><Icon name="x" /></button>
      </div>
      <div class="modal-body">{@render children()}</div>
      {#if foot}<div class="modal-foot">{@render foot()}</div>{/if}
    </div>
  </div>
{/if}
