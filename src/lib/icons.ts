// Only the icons the app uses, so the bundle stays small (Lucide, ISC licence).
import {
  createElement, House, Swords, ChartColumn, TrendingUp, Compass, Search, Users, Settings, RefreshCw, Layers, Gauge, ChevronDown,
  ChevronRight, ChevronLeft, Check, X, Copy, Eye, EyeOff, Lock, KeyRound, User, MapPin, Map, Crosshair, Coins, Lightbulb, Video,
  Bookmark, Folder, Download, Upload, Trash2, Plus, Minus, Coffee, Clock, CircleAlert, TriangleAlert, Info, ExternalLink, Play,
  Keyboard, Monitor, Shield, ArrowLeft, ArrowRight, StickyNote, Star, Database, Sparkles, Moon, Sun, Wifi, WifiOff, History,
  Pencil, RotateCcw, Target, Flag, type IconNode,
} from "lucide";

const ICONS: Record<string, IconNode> = {
  house: House, swords: Swords, chart: ChartColumn, "trending-up": TrendingUp, compass: Compass, search: Search, users: Users,
  settings: Settings, refresh: RefreshCw, layers: Layers, gauge: Gauge, "chevron-down": ChevronDown, "chevron-right": ChevronRight,
  "chevron-left": ChevronLeft, check: Check, x: X, copy: Copy, eye: Eye, "eye-off": EyeOff, lock: Lock, key: KeyRound, user: User,
  "map-pin": MapPin, map: Map, crosshair: Crosshair, coins: Coins, lightbulb: Lightbulb, video: Video, bookmark: Bookmark,
  folder: Folder, download: Download, upload: Upload, trash: Trash2, plus: Plus, minus: Minus, coffee: Coffee, clock: Clock,
  alert: CircleAlert, warn: TriangleAlert, info: Info, external: ExternalLink, play: Play, keyboard: Keyboard, monitor: Monitor,
  shield: Shield, "arrow-left": ArrowLeft, "arrow-right": ArrowRight, note: StickyNote, star: Star, database: Database,
  sparkles: Sparkles, moon: Moon, sun: Sun, wifi: Wifi, "wifi-off": WifiOff, history: History, pencil: Pencil, undo: RotateCcw,
  target: Target, flag: Flag,
};

/** Svelte action: <i use:icon={"house"}></i> renders the SVG inside. */
export function icon(node: HTMLElement, name: string) {
  const draw = (n: string) => {
    node.replaceChildren();
    const def = ICONS[n];
    if (!def) return;
    const svg = createElement(def);
    svg.setAttribute("class", `icon lucide lucide-${n}`);
    svg.setAttribute("aria-hidden", "true");
    node.appendChild(svg);
  };
  draw(name);
  return { update: draw };
}
