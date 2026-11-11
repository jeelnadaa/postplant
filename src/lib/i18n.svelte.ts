// UI translation. Strings are keyed by their English text, so English needs no dictionary and a
// missing translation simply shows English. Add a language: copy hi.json, translate the values,
// import it, and add it to LANGS and DICTS. `node scripts/i18n-wrap.mjs --list` prints every string in the UI.
// Hindi (hi.json) is translated but switched off for now; the picker shows once LANGS has two entries.

export const LANGS: [string, string][] = [["en", "English"]];
const DICTS: Record<string, Record<string, string>> = {};

export const i18n = $state({ lang: "en" });

export function setLang(lang: string | undefined | null) {
  i18n.lang = lang && (lang === "en" || DICTS[lang]) ? lang : "en";
  if (typeof document !== "undefined") document.documentElement.lang = i18n.lang;
}

/** Translate an English UI string. Reactive: templates re-render when the language changes. */
export function t(en: string): string {
  if (i18n.lang === "en") return en;
  return DICTS[i18n.lang]?.[en] ?? en;
}
