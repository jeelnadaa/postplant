// Thin typed wrappers over the Rust commands. Everything network-related happens in Rust.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type J = any;

export const call = <T = J>(cmd: string, args?: Record<string, unknown>) => invoke<T>(cmd, args);

export const api = {
  state: () => call("app_state"),
  savePrefs: (patch: J) => call("save_prefs", { patch }),
  setKey: (key: string) => call("set_api_key", { key }),
  removeKey: () => call("remove_api_key"),
  lookup: (name: string, tag: string) => call("lookup_account", { name, tag }),
  setMe: (me: J) => call("set_me", { me }),
  sync: (backfill?: number) => call("sync_now", { backfill }),
  home: () => call("home"),
  matches: (f: { queue?: string; map?: string; agent?: string; limit?: number }) => call("matches", f),
  match: (id: string) => call("match_detail", { id }),
  analytics: (days: number, queue: string) => call("analytics", { days, queue }),
  habits: () => call("habits"),
  rank: () => call("rank"),
  people: () => call("people"),
  search: (name: string, tag: string, force = false) => call("search_player", { name, tag, force }),
  recentSearches: () => call("recent_searches"),
  saveNote: (puuid: string, note: string, tags: string[]) => call("save_note", { puuid, note, tags }),
  reviews: (done = false) => call("reviews", { done }),
  addReview: (matchId: string, kind: string, round?: number | null, t?: number | null, note?: string) =>
    call("add_review", { matchId, kind, round: round ?? null, t: t ?? null, note: note ?? null }),
  updateReview: (id: number, p: { note?: string; done?: boolean; delete?: boolean }) => call("update_review", { id, ...p }),
  kb: () => call("agents_kb"),
  setOwned: (agentId: string, owned: boolean) => call("set_owned", { agentId, owned }),
  suggest: (map: string, team: string[]) => call("advisor_suggest", { map, team }),
  learn: (map: string) => call("advisor_learn", { map }),
  exportBackup: (path: string) => call("export_backup", { path }),
  exportJson: (path: string, anonymise: boolean, days?: number) => call("export_json", { path, anonymise, days }),
  inspectImport: (path: string) => call("inspect_import", { path }),
  applyImport: (path: string, mode: string, notes: string, includeSettings: boolean) => call("apply_import", { path, mode, notes, includeSettings }),
  backups: () => call("list_backups"),
  backupNow: () => call("backup_now"),
  restoreBackup: (id: number) => call("restore_backup", { id }),
  scanRecordings: (dir?: string) => call("recordings_scan", { dir }),
  watch: (matchId: string, t: number) => call("watch", { matchId, t }),
  overlay: (action: string) => call("overlay_cmd", { action }),
  setAutostart: (on: boolean) => call("set_autostart", { on }),
  getAutostart: () => call<boolean>("get_autostart"),
  checkUpdate: () => call("check_update"),
  installUpdate: () => call("install_update"),
  datasetStatus: () => call("dataset_status"),
  checkDataset: () => call("check_dataset"),
  deleteAll: (removeKey: boolean) => call("delete_all_data", { removeKey }),
  usage: () => call("usage"),
  overlayData: () => call("overlay_data"),
  liveStatus: () => call("live_status"),
  myCareer: (queue = "") => call("my_career", { queue }),
  playerCareer: (puuid: string, region: string, n = 0, queue = "") => call("player_career", { puuid, region, n, queue }),
  playerRank: (puuid: string, region?: string) => call("player_rank", { puuid, region: region ?? null }),
  matchRanks: (id: string) => call("match_ranks", { id }),
  testHotkey: (combo: string) => call("test_hotkey", { combo }),
  stackPeople: () => call("stack_people"),
  stack: (map: string, players: J[]) => call("advisor_stack", { map, players }),
  heatmap: (o: { map: string; event: string; side: string; layer: string; agent: string; outcome: string; rtype: string; matchId?: string }) => call("heatmap", o),
  trades: () => call("trade_stats"),
  patterns: () => call("pattern_notes"),
  duos: () => call("duo_synergy"),
  matchExtras: (id: string) => call("match_extras", { id }),
  fieldCheck: () => call("field_check"),
  replay: (id: string) => call("match_replay", { id }),
};

export function on<T = J>(event: string, fn: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(event, (e) => fn(e.payload));
}

export function errText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) return String((e as { message: unknown }).message);
  return "Something went wrong.";
}
