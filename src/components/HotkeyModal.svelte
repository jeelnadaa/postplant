<script lang="ts">
  // Press-to-set hotkey capture. While open, Postplant's own hotkeys are paused so the combination
  // reaches this window; the chosen one is tested with Windows before it is saved.
  import { api, errText } from "../lib/api";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";

  let {
    open = $bindable(false), label, current = "", taken = {}, onsave,
  }: { open: boolean; label: string; current?: string; taken?: Record<string, string>; onsave: (combo: string) => void } = $props();

  let combo = $state("");
  let held = $state("");
  let error = $state("");
  let busy = $state(false);

  $effect(() => {
    if (open) {
      combo = "";
      held = "";
      error = "";
      api.overlay("hotkeys_pause").catch(() => {});
      return () => {
        api.overlay("hotkeys_resume").catch(() => {});
      };
    }
  });

  const KEYMAP: Record<string, string> = {
    " ": "Space", ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right", Escape: "Escape",
    Enter: "Enter", Tab: "Tab", Backspace: "Backspace", Delete: "Delete", Insert: "Insert", Home: "Home", End: "End",
    PageUp: "PageUp", PageDown: "PageDown", "`": "Backquote", "-": "Minus", "=": "Equal", "[": "BracketLeft", "]": "BracketRight",
    ";": "Semicolon", "'": "Quote", ",": "Comma", ".": "Period", "/": "Slash", "\\": "Backslash",
  };
  function keyName(e: KeyboardEvent): string | null {
    if (["Control", "Shift", "Alt", "Meta", "AltGraph"].includes(e.key)) return null;
    if (/^F\d{1,2}$/.test(e.key)) return e.key;
    if (e.code.startsWith("Key")) return e.code.slice(3);
    if (e.code.startsWith("Digit")) return e.code.slice(5);
    if (e.code.startsWith("Numpad")) return "Num" + e.code.slice(6);
    return KEYMAP[e.key] ?? (e.key.length === 1 ? e.key.toUpperCase() : null);
  }
  function mods(e: KeyboardEvent) {
    return [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean) as string[];
  }
  function down(e: KeyboardEvent) {
    if (!open) return;
    e.preventDefault();
    e.stopPropagation();
    const k = keyName(e);
    const m = mods(e);
    held = [...m, k ?? ""].filter(Boolean).join(" + ");
    if (!k) return;
    if (k === "Escape" && !m.length) {
      open = false;
      return;
    }
    if (!m.length && !/^F\d{1,2}$/.test(k)) {
      error = "Use at least one of Ctrl, Alt or Shift (or an F key), so the game doesn't get it by accident.";
      return;
    }
    combo = [...m, k].join("+");
    error = "";
  }
  const clash = $derived(Object.entries(taken).find(([, v]) => v && combo && v.toLowerCase() === combo.toLowerCase())?.[0]);

  async function save() {
    if (!combo) return;
    busy = true;
    try {
      await api.testHotkey(combo);
      onsave(combo);
      open = false;
    } catch (e) {
      error = `Windows won't give us ${combo}: ${errText(e)}. Another app probably uses it.`;
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window onkeydown={down} />

<Modal bind:open title="Set hotkey: {label}" icon="keyboard">
  <div class="hk-capture" class:has={!!combo}>
    {#if combo}<span class="mono">{combo.split("+").join(" + ")}</span>
    {:else if held}<span class="mono faint">{held}</span>
    {:else}<span class="faint">Press the combination now…</span>{/if}
  </div>
  {#if clash}<div class="banner warn mt-4"><Icon name="warn" />Already used for "{clash}". Saving moves it here.</div>{/if}
  {#if error}<div class="banner danger mt-4"><Icon name="alert" />{error}</div>{/if}
  <p class="faint mt-4" style="font-size:12px">Current: <span class="mono">{current || "off"}</span>. Esc cancels. Your other Postplant hotkeys are paused while this is open.</p>
  {#snippet foot()}
    <button class="btn btn-ghost" onclick={() => { onsave(""); open = false; }}>Turn off</button>
    <span class="spacer"></span>
    <button class="btn" onclick={() => (open = false)}>Cancel</button>
    <button class="btn btn-primary" disabled={!combo || busy} onclick={save}><Icon name="check" />Save</button>
  {/snippet}
</Modal>

<style>
  .hk-capture { display: grid; place-items: center; min-height: 84px; border: 1.5px dashed var(--line-2); border-radius: var(--r-md, 10px); font-size: 20px; transition: border-color 0.15s ease, background-color 0.15s ease; }
  .hk-capture.has { border-style: solid; border-color: var(--accent); background: var(--accent-soft); }
</style>
