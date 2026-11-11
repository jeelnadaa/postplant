// Formatting helpers shared by the app and overlay. Pure functions (tested in format.test.ts).

const TIERS = ["iron", "bronze", "silver", "gold", "platinum", "diamond", "ascendant", "immortal"];

/** Competitive tier id (3–27) → CSS tier class used by .rank (t-<class>). */
export function tierClass(tier: number | null | undefined): string {
  if (!tier || tier < 3) return "";
  if (tier >= 27) return "radiant";
  return TIERS[Math.floor((tier - 3) / 3)] ?? "";
}

/** Tier id → "Diamond 2". 0–2 → "Unranked". */
export function tierName(tier: number | null | undefined, short = false): string {
  if (!tier || tier < 3) return "Unranked";
  if (tier >= 27) return "Radiant";
  const base = TIERS[Math.floor((tier - 3) / 3)];
  const div = ((tier - 3) % 3) + 1;
  if (short) {
    const ab: Record<string, string> = { iron: "I", bronze: "B", silver: "S", gold: "G", platinum: "P", diamond: "D", ascendant: "Asc ", immortal: "Imm " };
    return `${ab[base]}${div}`;
  }
  return `${base[0].toUpperCase()}${base.slice(1)} ${div}`;
}

export function timeAgo(ms: number | null | undefined, now = Date.now()): string {
  if (!ms) return "never";
  const s = Math.max(0, Math.round((now - ms) / 1000));
  if (s < 45) return "just now";
  const m = Math.round(s / 60);
  if (m < 60) return `${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h} h ago`;
  const d = Math.round(h / 24);
  if (d < 30) return `${d} d ago`;
  return new Date(ms).toLocaleDateString();
}

export function fmtDate(ms: number): string {
  return new Date(ms).toLocaleString(undefined, { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
}

export function mmss(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

export function hms(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return `${h ? h + ":" + String(m).padStart(2, "0") : m}:${String(s % 60).padStart(2, "0")}`;
}

export function kd(k: number, d: number): string {
  return (k / Math.max(1, d)).toFixed(2);
}

export function signed(n: number | null | undefined): string {
  if (n === null || n === undefined) return "—";
  return n > 0 ? `+${n}` : `${n}`;
}

export function queueLabel(q: string): string {
  const m: Record<string, string> = { competitive: "Competitive", unrated: "Unrated", swiftplay: "Swiftplay", spikerush: "Spike Rush", deathmatch: "Deathmatch", premier: "Premier", hurm: "Team Deathmatch", ggteam: "Escalation" };
  return m[q] ?? (q ? q[0].toUpperCase() + q.slice(1) : "Other");
}

/** Split "Name#tag" (also tolerates spaces around #). */
export function parseRiotId(s: string): { name: string; tag: string } | null {
  const i = s.lastIndexOf("#");
  if (i <= 0 || i === s.length - 1) return null;
  const name = s.slice(0, i).trim();
  const tag = s.slice(i + 1).trim();
  if (!name || !tag || tag.length > 5 || name.length > 16) return null;
  return { name, tag };
}

export const agentIcon = (id: string | undefined | null) => (id ? `https://media.valorant-api.com/agents/${id}/displayicon.png` : "");

export const REGIONS = [
  ["ap", "Asia Pacific"], ["eu", "Europe"], ["na", "North America"], ["kr", "Korea"], ["latam", "Latin America"], ["br", "Brazil"],
] as const;
