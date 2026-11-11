<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import { api, errText, type J } from "../lib/api";
  import { queueLabel } from "../lib/format";
  import { app, toast } from "../lib/store.svelte";
  import Icon from "../components/Icon.svelte";
  import MatchRow from "../components/MatchRow.svelte";
  import Modal from "../components/Modal.svelte";
  import Select from "../components/Select.svelte";

  let queue = $state("");
  let map = $state("");
  let agent = $state("");
  let result = $state("");
  let d = $state<J>(null);
  let backfillOpen = $state(false);
  let busy = $state(false);

  $effect(() => {
    void app.version;
    api.matches({ queue, map, agent, limit: 300 }).then((r) => (d = r));
  });
  const rows = $derived((d?.rows ?? []).filter((m: J) => !result || m.result === result));

  async function backfill() {
    busy = true;
    try {
      const r = await api.sync(20);
      toast("ok", `${r.new} older matches fetched`);
      app.version++;
      backfillOpen = false;
    } catch (e) {
      toast("err", "Couldn't fetch", errText(e));
    }
    busy = false;
  }
</script>

<div class="page-head">
  <div><h1 class="h1">{t("Matches")}</h1><div class="sub">{rows.length} {t("shown · filtering is instant because everything is local")}</div></div>
  <span class="spacer"></span>
  <button class="btn" onclick={() => (backfillOpen = true)}><Icon name="download" />{t("Fetch older matches")}</button>
</div>
{#if d}
  <div class="row wrap mb-4" style="gap:8px">
    <Select bind:value={queue} icon="swords" options={[["", "All modes"], ...d.queues.map((q: string) => [q, queueLabel(q)])]} minWidth={170} />
    <Select bind:value={map} icon="map" options={[["", "All maps"], ...d.maps.map((m: string) => [m, m])]} minWidth={150} />
    <Select bind:value={agent} icon="user" options={[["", "All agents"], ...d.agents.map((a: string) => [a, a])]} minWidth={150} />
    <div class="segmented">
      {#each [["", "All"], ["W", "Wins"], ["L", "Losses"]] as [v, l] (v)}<button aria-pressed={result === v} onclick={() => (result = v)}>{t(l)}</button>{/each}
    </div>
  </div>
  <div class="stack" style="gap:8px">
    {#each rows as m (m.id)}<MatchRow {m} />{:else}<div class="card empty-inline">{t("No matches for these filters.")}</div>{/each}
  </div>
{/if}

<Modal bind:open={backfillOpen} title="Fetch older matches" sub="Up to 20 of your most recent matches that aren't stored yet." icon="download">
  <p>{t("This costs about")} <b>{t("1 request per missing match")}</b> {t("plus 3, at most 23 requests, spread under your per-minute limit. Matches already on this PC are never fetched again.")}</p>
  <p class="faint" style="font-size:13px">{t("HenrikDev keeps a limited history per account, so very old matches may not be available.")}</p>
  {#snippet foot()}
    <button class="btn" onclick={() => (backfillOpen = false)}>{t("Cancel")}</button>
    <button class="btn btn-primary {busy ? 'is-loading' : ''}" onclick={backfill}>{t("Fetch up to 20")}</button>
  {/snippet}
</Modal>
