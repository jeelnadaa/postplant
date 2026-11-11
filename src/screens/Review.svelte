<script lang="ts">
  import { t } from "../lib/i18n.svelte";
  import { api, errText, type J } from "../lib/api";
  import { fmtDate, hms } from "../lib/format";
  import { app, nav, toast } from "../lib/store.svelte";
  import Agent from "../components/Agent.svelte";
  import Icon from "../components/Icon.svelte";

  let done = $state(false);
  let d = $state<J>(null);
  const load = () => api.reviews(done).then((r) => (d = r)).catch((e) => toast("err", "Couldn't load", errText(e)));
  $effect(() => {
    void app.version;
    void done;
    load();
  });

  async function upd(id: number, p: { note?: string; done?: boolean; delete?: boolean }) {
    try {
      await api.updateReview(id, p);
      if (p.note === undefined) load();
    } catch (e) {
      toast("err", "Couldn't save", errText(e));
    }
  }
  async function watch(it: J) {
    try {
      const r = await api.watch(it.match_id, it.t ?? 0);
      if (r.how === "default") {
        await navigator.clipboard.writeText(hms(r.seconds)).catch(() => {});
        toast("info", `Jump to ${hms(r.seconds)}`, "Copied to the clipboard. VLC and mpv jump there automatically.");
      }
    } catch (e) {
      toast("err", "Couldn't open the recording", errText(e));
    }
  }
  const what = (it: J) => (it.kind === "match" ? "Whole match" : it.kind === "round" ? `Round ${it.round + 1}` : `Death in round ${it.round + 1}`);
</script>

<div class="page-head">
  <div><h1 class="h1">{t("Review list")}</h1><div class="sub">{t("Moments you saved to watch back")}</div></div>
  <span class="spacer"></span>
  <div class="segmented"><button aria-pressed={!done} onclick={() => (done = false)}>{t("To review")}</button><button aria-pressed={done} onclick={() => (done = true)}>{t("Done")}</button></div>
</div>
{#if !app.s.prefs.recordings_dir}
  <div class="banner info mb-4"><Icon name="video" /><div>{t("Point Postplant at your recordings folder (ShadowPlay, OBS, AMD) to get Watch buttons that jump to the moment.")} <a class="link-btn" href="#/settings?tab=recordings">{t("Set it up")}</a></div></div>
{/if}
{#if d}
  <div class="stack" style="gap:10px">
    {#each d.items as it (it.id)}
      <div class="card" style="display:flex;gap:14px;align-items:center;padding:14px">
        {#if it.agent}<Agent name={it.agent} sm />{/if}
        <div style="flex:0 0 220px;min-width:0">
          <button class="link-btn" onclick={() => nav("match", { id: it.match_id })}><b>{it.map}</b></button>
          <div class="faint" style="font-size:12px">{what(it)}{it.t !== null ? ` · ${hms(Math.round(it.t / 1000))}` : ""} · {fmtDate(it.started_at)}</div>
        </div>
        <input class="input" style="flex:1" placeholder="What to look for…" value={it.note ?? ""} onchange={(e) => upd(it.id, { note: (e.currentTarget as HTMLInputElement).value })} />
        {#if it.recording?.path}<button class="btn btn-sm btn-primary" onclick={() => watch(it)}><Icon name="video" />{t("Watch")}</button>{/if}
        <button class="btn btn-sm" onclick={() => upd(it.id, { done: !done })}><Icon name={done ? "undo" : "check"} />{done ? "Undo" : "Done"}</button>
        <button class="btn btn-icon btn-ghost btn-sm" aria-label="Remove" title="Remove" onclick={() => upd(it.id, { delete: true })}><Icon name="trash" /></button>
      </div>
    {:else}
      <div class="card empty"><Icon name="bookmark" cls="art" /><div class="h3">{done ? "Nothing reviewed yet" : "Nothing to review"}</div>
        <p>{t("Use “Review later” on a match, round or death to save it here.")}</p></div>
    {/each}
  </div>
{/if}
