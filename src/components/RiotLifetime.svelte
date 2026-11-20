<script lang="ts">
  import { api, errText, on, type J } from "../lib/api";
  import { app } from "../lib/store.svelte";
  import { t } from "../lib/i18n.svelte";
  import { queueLabel } from "../lib/format";
  import KillTable from "./KillTable.svelte";
  import Career from "./Career.svelte";
  let { puuid, privateHistory = false }: { puuid: string; privateHistory?: boolean } = $props();
  let profile = $state<J>(null);
  let busy = $state(false);
  let message = $state("");
  let version = $state(0);
  let progress = $state<J>(null);
  $effect(() => {
    let current = true;
    void version;
    if (!privateHistory) api.riotProfile(puuid).then(r => { if(current) profile = r; }).catch(e => { if(current) message = errText(e); });
    return () => { current = false; };
  });
  $effect(() => {
    const id = puuid;
    const sub = on("history-progress", (p: J) => {
      if (p.puuid !== id) return;
      if (p.done) { progress = null; version++; } else progress = p;
    });
    return () => void sub.then(f => f());
  });
  const coverage = $derived.by(() => {
    const games = new Map<string, number>();
    for (const r of profile?.combos ?? []) games.set(r.queue, (games.get(r.queue) ?? 0) + r.games);
    return games;
  });
  async function sync() {
    busy = true;
    message = "";
    try {
      const r = await api.riotHistory(puuid);
      message = r.busy ? t("Another Riot download is in progress.") : r.partial ? t("Riot returned part of this history. Load again to retry.") : t("Finished checking the records Riot made available.");
    } catch(e) { message = errText(e); }
    finally { busy = false; version++; }
  }
</script>

<section class="card mb-4">
  <div class="card-head"><h3 class="h3">{t("Riot lifetime record")}</h3><span class="spacer"></span>
    {#if !privateHistory}<button class="btn btn-sm" disabled={busy || !app.s?.prefs?.live_enabled} onclick={sync}>{busy ? t("Reading Riot history…") : t("Load from Riot")}</button>{/if}
  </div>
  {#if privateHistory}<p>{t("This player's history is private.")}</p>
  {:else}
    {#if !app.s?.prefs?.live_enabled}<p>{t("Enable Live match in Settings and run VALORANT to download Riot records.")}</p>{/if}
    <p class="faint">{t("Account game counts come from Riot’s act records. Kill totals cover downloaded match scoreboards. Older games can be missing even after a full sync.")}</p>
    {#if progress}<p role="status">{t("Reading page")} {progress.page} · {progress.added} {t("games added")}</p>{/if}
    {#if message}<p role="status">{message}</p>{/if}
    {#if profile?.modes?.list?.length}
      <table class="table"><thead><tr><th>{t("Mode")}</th><th class="num">{t("Lifetime games")}</th><th class="num">{t("Downloaded scoreboards")}</th><th class="num">{t("Wins")}</th><th class="num">{t("Win rate")}</th></tr></thead><tbody>
        {#each profile.modes.list as m}<tr><td>{queueLabel(m.name)}</td><td class="num">{m.games}</td><td class="num">{coverage.get(m.name) ?? 0}</td><td class="num">{m.wins}</td><td class="num">{m.win}%</td></tr>{/each}
      </tbody></table>
    {/if}
  {/if}
</section>
{#if !privateHistory && profile}
  <KillTable rows={profile.combos} modes />
  {#if profile.career?.overall?.games}<h3 class="h3 mb-2">{t("Downloaded team match stats")}</h3><Career d={profile.career} quick kills={false} />{/if}
{/if}
