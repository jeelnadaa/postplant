// App-wide reactive state (Svelte 5 runes) and a tiny hash router.
import { api, errText, on, type J } from "./api";

export type Route = { name: string; params: Record<string, string> };

function parseHash(): Route {
  const h = location.hash.replace(/^#\/?/, "");
  const [path, q] = h.split("?");
  const params: Record<string, string> = Object.fromEntries(new URLSearchParams(q ?? ""));
  return { name: path || "home", params };
}

export const app = $state({
  ready: false,
  s: null as J, // app_state()
  route: parseHash(),
  kb: null as J,
  toasts: [] as { id: number; kind: string; title: string; text?: string }[],
  palette: false,
  version: 0, // bump to make screens reload their data
});

export function nav(name: string, params: Record<string, string> = {}) {
  const q = new URLSearchParams(params).toString();
  location.hash = `#/${name}${q ? "?" + q : ""}`;
}

let tid = 0;
export function toast(kind: "ok" | "err" | "warn" | "info", title: string, text?: string) {
  const id = ++tid;
  app.toasts.push({ id, kind, title, text });
  setTimeout(() => (app.toasts = app.toasts.filter((t) => t.id !== id)), kind === "err" ? 8000 : 5000);
}

export async function refreshState() {
  try {
    app.s = await api.state();
    applyPrefs(app.s.prefs, app.s.game_running);
  } catch (e) {
    toast("err", "Couldn't read app state", errText(e));
  }
}

export async function loadKb() {
  try {
    app.kb = await api.kb();
  } catch {
    /* before setup there is no player yet */
  }
}

export function applyPrefs(p: J, game = false) {
  if (!p) return;
  const el = document.documentElement;
  let theme = p.theme;
  if (theme === "system") theme = matchMedia("(prefers-color-scheme: light)").matches ? "paper" : "ink";
  el.dataset.theme = theme;
  el.dataset.font = p.font;
  if (p.cvd) el.dataset.cvd = "on";
  else delete el.dataset.cvd;
  const reduce = p.reduce_motion === "on" || (p.reduce_motion === "system" && matchMedia("(prefers-reduced-motion: reduce)").matches);
  if (reduce) el.dataset.motion = "reduce";
  else delete el.dataset.motion;
  if (game) el.dataset.perf = "game";
  else delete el.dataset.perf;
  el.style.fontSize = `${p.scale ?? 100}%`;
}

export async function savePrefs(patch: J) {
  try {
    const p = await api.savePrefs(patch);
    if (app.s) app.s.prefs = p;
    applyPrefs(p, app.s?.game_running);
  } catch (e) {
    toast("err", "Couldn't save the setting", errText(e));
  }
}

export function agentId(name: string): string | undefined {
  return app.kb?.agents?.find((a: J) => a.name === name)?.id;
}

export async function init() {
  addEventListener("hashchange", () => (app.route = parseHash()));
  await refreshState();
  await loadKb();
  app.ready = true;
  on("sync-state", (p: J) => {
    if (!app.s) return;
    app.s.syncing = !!p.syncing;
    if (p.at) app.s.last_sync = p.at;
    if (p.error !== undefined) app.s.last_error = p.error;
    else if (p.at) app.s.last_error = null;
    if (p.new > 0) {
      app.version++;
      toast("ok", p.new === 1 ? "New match synced" : `${p.new} new matches synced`);
      loadKb();
    }
  });
  on("game", (p: J) => {
    if (app.s) app.s.game_running = p.running;
    applyPrefs(app.s?.prefs, p.running);
  });
  on("navigate", (r: string) => nav(r));
  on("overlay-visible", (p: J) => app.s && (app.s.overlay_visible = p.visible));
  on("hotkeys", (p: J) => {
    if (p.failed?.length) toast("warn", "Some hotkeys couldn't be registered", `Another app is using: ${p.failed.join(", ")}. Rebind them in Settings → Overlay & hotkeys.`);
  });
  on("reset", () => location.reload());
  setInterval(async () => {
    if (app.s) app.s.usage = await api.usage().catch(() => app.s.usage);
  }, 15000);
}
