<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  // Name#tag with a copy button. Players who hid their name in game get an eye-off tag (plan section 11).
  import { toast } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";
  let { name, tag = "", agent = "", hidden = false }: { name: string; tag?: string; agent?: string; hidden?: boolean } = $props();
  const missing = $derived(!name);

  async function copy(e: MouseEvent) {
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(`${name}#${tag}`);
      toast("ok", "Riot ID copied", `${name}#${tag}`);
    } catch {
      toast("err", "Couldn't copy");
    }
  }
</script>

{#if missing}
  <span class="hidden-name" title="Name hidden by the player"><Icon name="eye-off" cls="eo" />{agent}</span>
{:else}
  <span class="nm">{name}</span>{#if tag}<span class="faint">#{tag}</span>{/if}
  {#if hidden}<span class="tag" title="Hid their name during the match"><Icon name="eye-off" /> {t("hidden in match")}</span>{/if}
  <button class="btn btn-icon btn-ghost btn-sm copy-btn" title="Copy Riot ID" aria-label="Copy Riot ID" onclick={copy}><Icon name="copy" /></button>
{/if}
