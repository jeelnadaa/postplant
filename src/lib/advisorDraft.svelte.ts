import { onMount } from "svelte";
import { api, on, type J } from "./api";

// Each advisor window hydrates after subscribing so a late status response cannot roll back a pick.
export function useAdvisorDraft() {
  let draft = $state<J>(null);
  let map = $state("");
  onMount(() => {
    let active = true;
    let revision = 0;
    const subscriptions = [
      on("advisor-draft", (value: J) => {
        revision++;
        draft = value;
        if (value?.map) map = value.map;
      }),
      on("advisor-map", (value: J) => { revision++; map = value.map; }),
      on("live-phase", (value: J) => {
        revision++;
        map = value.phase === "pregame" ? value.map : "";
        if (value.phase !== "pregame") draft = null;
      }),
    ];
    Promise.all(subscriptions).then(async () => {
      const at = revision;
      const status = await api.liveStatus();
      if (active && revision === at && status?.phase === "pregame") {
        draft = status.draft;
        map = status.map;
      }
    }).catch(() => {});
    return () => {
      active = false;
      for (const sub of subscriptions) sub.then((off) => off()).catch(() => {});
    };
  });
  return {
    get players(): J[] | undefined { return draft?.players; },
    get picks(): string[] { return [...new Set<string>((draft?.players ?? []).filter((p: J) => !p.me && p.agent).map((p: J) => p.agent))]; },
    get map() { return map; },
  };
}
