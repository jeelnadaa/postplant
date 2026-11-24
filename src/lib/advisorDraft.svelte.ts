import { settleDraft } from "./settledDraft";
import { onMount } from "svelte";
import { api, on, type J } from "./api";

// Each advisor window hydrates after subscribing so a late status response cannot roll back a pick.
export function useAdvisorDraft() {
  let draft = $state<J>(null);
  let settled = $state<J[] | undefined>();
  let map = $state("");
  onMount(() => {
    const settle = settleDraft<J>(players => settled = players);
    function receive(value: J) {
      if (value?.map && value.map !== map) settle();
      draft = value;
      if (value?.map) map = value.map;
      settle(value?.players);
    }
    let active = true;
    let revision = 0;
    const subscriptions = [
      on("advisor-draft", (value: J) => {
        revision++;
        if (active) receive(value);
      }),
      on("advisor-map", (value: J) => { revision++; map = value.map; }),
      on("live-phase", (value: J) => {
        revision++;
        map = value.phase === "pregame" ? value.map : "";
        if (active && value.phase !== "pregame") receive(null);
      }),
    ];
    Promise.all(subscriptions).then(async () => {
      const at = revision;
      const status = await api.liveStatus();
      if (active && revision === at && status?.phase === "pregame") {
        receive(status.draft);
        map = status.map;
      }
    }).catch(() => {});
    return () => {
      active = false;
      settle();
      for (const sub of subscriptions) sub.then((off) => off()).catch(() => {});
    };
  });
  return {
    get players(): J[] | undefined { return draft?.players; },
    get settledPlayers() { return settled; },
    get map() { return map; },
  };
}
