<script lang="ts">
  import { app } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";
  const ICON: Record<string, string> = { ok: "check", err: "alert", warn: "warn", info: "info" };
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}">
      <Icon name={ICON[t.kind]} />
      <div><div class="t">{t.title}</div>{#if t.text}<div class="d">{t.text}</div>{/if}</div>
      <button class="btn btn-icon btn-ghost btn-sm" aria-label="Dismiss" onclick={() => (app.toasts = app.toasts.filter((x) => x.id !== t.id))}><Icon name="x" /></button>
      <span class="timer"></span>
    </div>
  {/each}
</div>
