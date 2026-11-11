<script lang="ts">
  import Icon from "./Icon.svelte";
  let {
    value = $bindable(""), options, placeholder = "Choose", icon = "", minWidth = 180, onchange = undefined,
  }: { value: string; options: (string | [string, string])[]; placeholder?: string; icon?: string; minWidth?: number; onchange?: (v: string) => void } = $props();

  let open = $state(false);
  let root: HTMLDivElement;
  const opts = $derived(options.map((o) => (Array.isArray(o) ? o : [o, o])) as [string, string][]);
  const label = $derived(opts.find(([v]) => v === value)?.[1] ?? "");

  function pick(v: string) {
    value = v;
    open = false;
    onchange?.(v);
  }
  function outside(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onclick={outside} />
<div class="select {open ? 'open' : ''}" style="min-width:{minWidth}px" bind:this={root}>
  <button type="button" class="select-trigger" aria-haspopup="listbox" aria-expanded={open} onclick={() => (open = !open)}
    onkeydown={(e) => e.key === "Escape" && (open = false)}>
    {#if icon}<Icon name={icon} />{/if}
    <span class="value {label ? '' : 'placeholder'}">{label || placeholder}</span>
    <Icon name="chevron-down" cls="chev" />
  </button>
  <div class="menu" role="listbox">
    {#each opts as [v, l] (v)}
      <button type="button" class="option" role="option" aria-selected={v === value} onclick={() => pick(v)}>
        <span>{l}</span><Icon name="check" cls="check" />
      </button>
    {/each}
  </div>
</div>
