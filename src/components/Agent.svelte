<script lang="ts">
  import { agentIcon } from "../lib/format";
  import { app } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";

  let {
    name, id = "", sm = false, pressed = undefined, locked = false, picked = false, onclick = undefined, title = undefined,
  }: { name: string; id?: string; sm?: boolean; pressed?: boolean; locked?: boolean; picked?: boolean; onclick?: () => void; title?: string } = $props();

  const kbAgent = $derived(app.kb?.agents?.find((a: { name: string }) => a.name === name));
  const src = $derived(agentIcon(id || kbAgent?.id));
  const role = $derived((kbAgent?.role ?? "").toLowerCase());
  let broken = $state(false);
  const glyph = $derived(name ? name.replace(/[^A-Za-z]/g, "").slice(0, 2) : "?");
</script>

{#if onclick}
  <button
    type="button"
    class="agent {sm ? 'agent-sm' : ''} {role ? 'r-' + role : ''} {locked ? 'is-locked' : ''} {picked ? 'is-picked' : ''}"
    aria-pressed={pressed}
    title={title ?? name}
    {onclick}
  >
    {#if src && !broken}<img class="face" {src} alt="" loading="lazy" decoding="async" onerror={() => (broken = true)} />{/if}
    <span class="mono-glyph">{glyph}</span>
    <span class="nm">{name}</span>
    {#if locked}<span class="lock"><Icon name="lock" /></span>{/if}
  </button>
{:else}
  <span class="agent {sm ? 'agent-sm' : ''} {role ? 'r-' + role : ''} {locked ? 'is-locked' : ''}" title={title ?? name}>
    {#if src && !broken}<img class="face" {src} alt="" loading="lazy" decoding="async" onerror={() => (broken = true)} />{/if}
    <span class="mono-glyph">{glyph}</span>
    <span class="nm">{name}</span>
  </span>
{/if}
