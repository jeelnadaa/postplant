#!/usr/bin/env node
// Wraps static text in Svelte markup with t("…") so it can be translated (plan: lib/i18n).
//   node scripts/i18n-wrap.mjs --list      print every string found, as JSON
//   node scripts/i18n-wrap.mjs --apply     rewrite the files
// Only text between tags is touched: no attributes, scripts or styles. Text already inside {…} is
// left alone. Missing translations fall back to English at runtime, so a partial dictionary is safe.
import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const ROOT = new URL("../src/", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const FILES = [
  "App.svelte", "Overlay.svelte",
  ...readdirSync(join(ROOT, "screens")).filter((f) => f.endsWith(".svelte")).map((f) => `screens/${f}`),
  ...readdirSync(join(ROOT, "components")).filter((f) => f.endsWith(".svelte")).map((f) => `components/${f}`),
];
const SKIP_TAGS = /<(script|style|code|pre|kbd|title|text)\b[\s\S]*?<\/\1>/g;

function split(src) {
  // Returns [markupWithPlaceholders, saved blocks] so we never edit inside script/style/code.
  const saved = [];
  const m = src.replace(SKIP_TAGS, (x) => `\u0000${saved.push(x) - 1}\u0000`);
  return [m, saved];
}
const join2 = (m, saved) => m.replace(/\u0000(\d+)\u0000/g, (_, i) => saved[+i]);

// Text nodes: runs outside tags and outside {…} expressions (brace- and quote-aware).
function segments(m) {
  const out = [];
  let i = 0, depth = 0, inTag = false, start = 0;
  const flush = (end) => { if (end > start) out.push({ index: start - 1, raw: m.slice(start, end) }); };
  while (i < m.length) {
    const c = m[i];
    if (depth > 0) {
      if (c === '"' || c === "'" || c === "`") { const q = c; i++; while (i < m.length && m[i] !== q) { if (m[i] === "\\") i++; i++; } }
      else if (c === "{") depth++;
      else if (c === "}") { depth--; if (depth === 0 && !inTag) start = i + 1; }
      i++;
      continue;
    }
    if (c === "{") { if (!inTag) flush(i); depth = 1; i++; continue; }
    if (inTag) {
      if (c === '"' || c === "'") { const q = c; i++; while (i < m.length && m[i] !== q) { if (m[i] === "{") { let d = 1; i++; while (i < m.length && d) { if (m[i] === "{") d++; else if (m[i] === "}") d--; i++; } continue; } i++; } }
      else if (c === ">") { inTag = false; start = i + 1; }
      i++;
      continue;
    }
    if (c === "<") { flush(i); inTag = true; }
    i++;
  }
  return out;
}

function strings(markup) {
  const out = [];
  for (const seg of segments(markup)) {
    const raw = seg.raw;
    const s = raw.trim().replace(/&amp;/g, "&").replace(/&nbsp;/g, " ").replace(/&lt;/g, "<").replace(/&gt;/g, ">");
    if (!s || s.length < 2 || !/[A-Za-z]{2}/.test(s) || s.includes("\u0000")) continue;
    if (/^(ACS|ADR|KDA|K\/D|HS|KAST|RR|FB|FD|ATK|DEF|VLC|mpv|JSON|API|Ctrl|Shift|Alt|Esc|Tab)$/.test(s)) continue;
    out.push({ index: seg.index, raw, s });
  }
  return out;
}

const esc = (s) => s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
const args = process.argv.slice(2);
const all = new Set();
for (const f of FILES) {
  const path = join(ROOT, f);
  const src = readFileSync(path, "utf8");
  const [markup, saved] = split(src);
  // Only the template part (after </script> placeholders) – the script is already a placeholder.
  const found = strings(markup);
  found.forEach((x) => all.add(x.s));
  if (args.includes("--apply") && found.length) {
    let out = "";
    let last = 0;
    for (const x of found) {
      const start = x.index + 1;
      const lead = x.raw.match(/^\s*/)[0];
      const trail = x.raw.match(/\s*$/)[0];
      out += markup.slice(last, start) + lead + `{t("${esc(x.s)}")}` + trail;
      last = start + x.raw.length;
    }
    out += markup.slice(last);
    let res = join2(out, saved);
    // Import t once, in the first <script> block.
    if (!/import \{ t \} from/.test(res)) {
      const rel = f.includes("/") ? "../lib/i18n.svelte" : "./lib/i18n.svelte";
      res = res.replace(/<script lang="ts">(\r?\n)/, `<script lang="ts">$1  import { t } from "${rel}";$1`);
    }
    writeFileSync(path, res);
  }
}
if (args.includes("--list")) console.log(JSON.stringify([...all].sort(), null, 1));
else console.error(`${all.size} strings`);
