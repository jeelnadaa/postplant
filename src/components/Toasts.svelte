<script lang="ts">
  import { app } from "../lib/store.svelte";
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";
  const ICON: Record<string, string> = { ok: "check", err: "alert", warn: "warn", info: "info" };
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as x (x.id)}
    <div class="toast {x.kind}">
      <Icon name={ICON[x.kind]} />
      <div><div class="t">{t(x.title)}</div>{#if x.text}<div class="d">{t(x.text)}</div>{/if}</div>
      <button class="btn btn-icon btn-ghost btn-sm" aria-label="Dismiss" onclick={() => (app.toasts = app.toasts.filter((y) => y.id !== x.id))}><Icon name="x" /></button>
      <span class="timer"></span>
    </div>
  {/each}
</div>
