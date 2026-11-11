<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import { api, errText } from "../lib/api";
  import { toast } from "../lib/store.svelte";
  import Modal from "./Modal.svelte";

  let { open = $bindable(false), puuid, who, note = null, onsaved = undefined }: { open: boolean; puuid: string; who: string; note?: { note: string; tags: string[] } | null; onsaved?: () => void } = $props();
  const TAGS = ["good duo", "good teammate", "toxic", "smurf?", "thrower", "shotcaller"];
  let text = $state("");
  let tags = $state<string[]>([]);
  $effect(() => {
    if (open) {
      text = note?.note ?? "";
      tags = [...(note?.tags ?? [])];
    }
  });
  async function save() {
    try {
      await api.saveNote(puuid, text, tags);
      toast("ok", "Note saved", "Notes stay on this PC.");
      open = false;
      onsaved?.();
    } catch (e) {
      toast("err", "Couldn't save the note", errText(e));
    }
  }
</script>

<Modal bind:open title="Note on {who}" sub="Private: never uploaded, never exported when you hide names." icon="note">
  <div class="row wrap" style="gap:6px;margin-bottom:12px">
    {#each TAGS as t (t)}
      <button class="chip" aria-pressed={tags.includes(t)} onclick={() => (tags = tags.includes(t) ? tags.filter((x) => x !== t) : [...tags, t])}>{t}</button>
    {/each}
  </div>
  <textarea class="input textarea" style="width:100%" rows="4" placeholder="What should you remember about them?" bind:value={text}></textarea>
  {#snippet foot()}
    <button class="btn" onclick={() => (open = false)}>{t("Cancel")}</button>
    <button class="btn btn-primary" onclick={save}>{t("Save note")}</button>
  {/snippet}
</Modal>
