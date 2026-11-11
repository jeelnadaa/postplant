// Postplant design board: theme/font switching, interactive component behaviour,
// and small render helpers for repeated mock data. Reference only; the app
// re-implements these behaviours in Svelte.
(() => {
  const store = {
    get: (k) => { try { return localStorage.getItem(k); } catch { return null; } },
    set: (k, v) => { try { localStorage.setItem(k, v); } catch { /* ignore */ } },
  };
  const root = document.documentElement;

  /* ---------- Data ---------- */
  const THEMES = [
    { id: "ink", name: "Ink", mode: "Dark", sw: ["#0a0b0e", "#14161c", "#d7ff3a", "#ede8df"] },
    { id: "midnight", name: "Midnight", mode: "Dark", sw: ["#070b16", "#121b31", "#6cb6ff", "#e8eef9"] },
    { id: "nebula", name: "Nebula", mode: "Dark", sw: ["#0a0711", "#1a142e", "#c4a1ff", "#f0eafa"] },
    { id: "paper", name: "Paper", mode: "Light", sw: ["#ebe6dc", "#ffffff", "#c8f031", "#141519"] },
    { id: "frost", name: "Frost", mode: "Light", sw: ["#e2e8f0", "#ffffff", "#2563eb", "#0d1626"] },
    { id: "sandstone", name: "Sandstone", mode: "Light", sw: ["#eee3d5", "#fffcf8", "#c2410c", "#22180f"] },
  ];
  const FONTS = [
    { id: "satoshi", name: "Satoshi", note: "Default · geometric, warm" },
    { id: "general", name: "General Sans", note: "Sharper, more neutral" },
    { id: "geist", name: "Geist", note: "Technical, compact" },
    { id: "inter", name: "Inter", note: "Most familiar, very legible" },
    { id: "atkinson", name: "Atkinson Hyperlegible", note: "Built for low vision" },
  ];
  const AGENTS = {
    Duelist: ["Jett", "Phoenix", "Reyna", "Raze", "Yoru", "Neon", "Iso", "Waylay", "Hush"],
    Initiator: ["Sova", "Breach", "Skye", "KAY/O", "Fade", "Gekko", "Tejo"],
    Controller: ["Brimstone", "Omen", "Viper", "Astra", "Harbor", "Clove", "Miks"],
    Sentinel: ["Sage", "Cypher", "Killjoy", "Chamber", "Deadlock", "Vyse", "Veto"],
  };
  const ROLE_OF = {};
  Object.entries(AGENTS).forEach(([r, l]) => l.forEach((a) => (ROLE_OF[a] = r)));
  const MAPS = { Abyss: "#4a6fa5", Bind: "#c98a4b", Corrode: "#7d8f6a", Haven: "#b58a5a", Lotus: "#5a9a8a", Summit: "#8c9fb8", Sunset: "#d07a5a" };

  /* ---------- Render helpers (used by pages via data attributes) ---------- */
  const glyph = (a) => (a === "KAY/O" ? "K/O" : a.slice(0, 2));
  const lockSvg = '<svg class="lock" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/></svg>';
  function agentTile(a, opts = {}) {
    const cls = ["agent", "r-" + ROLE_OF[a].toLowerCase()];
    if (opts.sm) cls.push("agent-sm");
    if (opts.locked) cls.push("is-locked");
    if (opts.picked) cls.push("is-picked");
    const tag = opts.button ? "button" : "span";
    const extra = opts.button ? ` type="button" aria-pressed="${opts.pressed ? "true" : "false"}"${opts.locked ? ' aria-disabled="true" title="' + a + ' is locked on your account"' : ""}` : "";
    return `<${tag} class="${cls.join(" ")}"${extra}><span class="mono-glyph">${glyph(a)}</span><span class="nm">${a}</span>${opts.locked ? lockSvg : ""}</${tag}>`;
  }
  window.PP = { THEMES, FONTS, AGENTS, ROLE_OF, MAPS, agentTile, glyph };

  /* ---------- Theme / font / CVD ---------- */
  function apply(theme, font, cvd) {
    root.dataset.theme = theme;
    root.dataset.font = font;
    root.dataset.cvd = cvd ? "on" : "off";
    store.set("ppd-theme", theme); store.set("ppd-font", font); store.set("ppd-cvd", cvd ? "1" : "0");
    document.querySelectorAll("[data-theme-label]").forEach((el) => { const t = THEMES.find((x) => x.id === theme); el.textContent = t.name + " · " + t.mode; });
    document.querySelectorAll("[data-font-label]").forEach((el) => { el.textContent = FONTS.find((x) => x.id === font).name; });
    document.querySelectorAll("[data-set-theme]").forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.setTheme === theme)));
    document.querySelectorAll('[data-select="theme"] .option').forEach((o) => o.setAttribute("aria-selected", String(o.dataset.value === theme)));
    document.querySelectorAll('[data-select="font"] .option').forEach((o) => o.setAttribute("aria-selected", String(o.dataset.value === font)));
    document.querySelectorAll("[data-cvd-switch]").forEach((s) => s.setAttribute("aria-checked", String(cvd)));
  }
  let theme = store.get("ppd-theme") || "ink";
  let font = store.get("ppd-font") || "satoshi";
  let cvd = store.get("ppd-cvd") === "1";
  if (!THEMES.some((t) => t.id === theme)) theme = "ink";
  if (!FONTS.some((f) => f.id === font)) font = "satoshi";

  /* ---------- Design toolbar ---------- */
  function toolbar() {
    const host = document.querySelector("[data-toolbar]");
    if (!host) return;
    const page = document.body.dataset.page;
    const link = (href, label, id) => `<a href="${href}" class="nav-item" ${page === id ? 'aria-current="page"' : ""}>${label}</a>`;
    host.innerHTML = `
      <div class="dt-inner">
        <a class="logo" href="index.html"><svg viewBox="0 0 48 48"><use href="#logo"/></svg>Postplant <span class="badge">Design</span></a>
        <nav class="dt-nav">${link("index.html", "Foundations", "foundations")}${link("components.html", "Components", "components")}${link("screens.html", "Screens", "screens")}${link("overlay.html", "Overlay", "overlay")}</nav>
        <div class="dt-ctl">
          <div class="select" data-select="theme" style="min-width:170px">
            <button type="button" class="select-trigger" aria-haspopup="listbox" aria-expanded="false"><i data-lucide="palette" class="icon"></i><span class="value" data-theme-label></span><i data-lucide="chevron-down" class="icon chev"></i></button>
            <div class="menu" role="listbox">
              ${["Dark", "Light"].map((m) => `<div class="menu-group caps">${m}</div>` + THEMES.filter((t) => t.mode === m).map((t) => `<button type="button" class="option" role="option" data-value="${t.id}"><span class="swatch-dot" style="background:linear-gradient(135deg, ${t.sw[0]} 50%, ${t.sw[2]} 50%)"></span>${t.name}<i data-lucide="check" class="icon check"></i></button>`).join("")).join("")}
            </div>
          </div>
          <div class="select" data-select="font" style="min-width:150px">
            <button type="button" class="select-trigger" aria-haspopup="listbox" aria-expanded="false"><i data-lucide="type" class="icon"></i><span class="value" data-font-label></span><i data-lucide="chevron-down" class="icon chev"></i></button>
            <div class="menu" role="listbox" style="min-width:240px">
              ${FONTS.map((f) => `<button type="button" class="option" role="option" data-value="${f.id}"><span style="font-family:var(--f-${f.id})">${f.name}</span><span class="meta">${f.id === "satoshi" ? "Default" : ""}</span><i data-lucide="check" class="icon check"></i></button>`).join("")}
            </div>
          </div>
          <label class="row" style="gap:8px; font-size:var(--fs-sm); color:var(--text-2)"><button type="button" class="switch" role="switch" aria-checked="false" data-cvd-switch aria-label="Colour-blind-safe colours"></button>CVD</label>
        </div>
      </div>`;
  }

  /* ---------- Behaviours ---------- */
  function closeAllSelects(except) {
    document.querySelectorAll(".select.open").forEach((s) => { if (s !== except) { s.classList.remove("open"); s.querySelector(".select-trigger")?.setAttribute("aria-expanded", "false"); } });
  }
  document.addEventListener("click", (e) => {
    const trig = e.target.closest(".select-trigger");
    if (trig) {
      const sel = trig.closest(".select");
      const open = !sel.classList.contains("open");
      closeAllSelects(sel);
      sel.classList.toggle("open", open);
      trig.setAttribute("aria-expanded", String(open));
      if (open) sel.querySelector(".menu-search input")?.focus();
      return;
    }
    const opt = e.target.closest(".select .option");
    if (opt && opt.getAttribute("aria-disabled") !== "true") {
      const sel = opt.closest(".select");
      const kind = sel.dataset.select;
      if (kind === "theme") { theme = opt.dataset.value; apply(theme, font, cvd); }
      else if (kind === "font") { font = opt.dataset.value; apply(theme, font, cvd); }
      else if (!sel.hasAttribute("data-multi")) {
        sel.querySelectorAll(".option").forEach((o) => o.setAttribute("aria-selected", String(o === opt)));
        const v = sel.querySelector(".select-trigger .value");
        if (v) v.innerHTML = opt.querySelector("[data-label]")?.innerHTML || opt.textContent.trim();
      } else {
        opt.setAttribute("aria-selected", String(opt.getAttribute("aria-selected") !== "true"));
        return;
      }
      sel.classList.remove("open");
      sel.querySelector(".select-trigger").setAttribute("aria-expanded", "false");
      return;
    }
    if (!e.target.closest(".select")) closeAllSelects();

    const sw = e.target.closest('.switch, .check-box, [role="switch"]');
    if (sw && !sw.hasAttribute("data-static")) {
      if (sw.hasAttribute("data-cvd-switch")) { cvd = !cvd; apply(theme, font, cvd); return; }
      sw.setAttribute("aria-checked", String(sw.getAttribute("aria-checked") !== "true"));
      if (sw.dataset.reveal) document.querySelector(sw.dataset.reveal)?.toggleAttribute("hidden", sw.getAttribute("aria-checked") !== "true");
    }
    const radio = e.target.closest(".choice[data-group]");
    if (radio) {
      document.querySelectorAll(`.choice[data-group="${radio.dataset.group}"] .radio-dot`).forEach((r) => r.setAttribute("aria-checked", "false"));
      radio.querySelector(".radio-dot").setAttribute("aria-checked", "true");
    }
    const seg = e.target.closest(".segmented button, .tabs .tab, [data-chipgroup] .chip");
    if (seg) {
      const group = seg.parentElement;
      const attr = seg.classList.contains("tab") ? "aria-selected" : "aria-pressed";
      group.querySelectorAll(seg.classList.contains("tab") ? ".tab" : "button").forEach((b) => b.setAttribute(attr, String(b === seg)));
    }
    const setT = e.target.closest("[data-set-theme]");
    if (setT) { theme = setT.dataset.setTheme; apply(theme, font, cvd); }
    const openM = e.target.closest("[data-open]");
    if (openM) { const m = document.getElementById(openM.dataset.open); m?.classList.add("open"); m?.querySelector("[autofocus], input, .btn-primary, .btn-danger")?.focus(); }
    if (e.target.closest("[data-close]") || e.target.classList.contains("scrim")) { e.target.closest(".scrim")?.classList.remove("open"); }
    const toastBtn = e.target.closest("[data-toast]");
    if (toastBtn) toast(toastBtn.dataset.toast);
    const ag = e.target.closest("button.agent");
    if (ag && !ag.classList.contains("is-locked") && !ag.closest("[data-static]")) ag.setAttribute("aria-pressed", String(ag.getAttribute("aria-pressed") !== "true"));
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") { closeAllSelects(); document.querySelectorAll(".scrim.open").forEach((s) => s.classList.remove("open")); }
    const sel = document.activeElement?.closest?.(".select.open");
    if (sel && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
      e.preventDefault();
      const opts = [...sel.querySelectorAll(".option:not([aria-disabled='true'])")];
      let i = opts.indexOf(document.activeElement);
      i = e.key === "ArrowDown" ? Math.min(opts.length - 1, i + 1) : Math.max(0, i - 1);
      opts[i]?.focus();
    }
  });
  document.addEventListener("input", (e) => {
    const s = e.target.closest(".menu-search input");
    if (!s) return;
    const q = s.value.toLowerCase();
    s.closest(".menu").querySelectorAll(".option").forEach((o) => { o.hidden = !o.textContent.toLowerCase().includes(q); });
  });

  const TOASTS = {
    ok: ["check-circle-2", "Key works", "Basic key · 30 requests per minute"],
    err: ["x-circle", "Couldn't reach HenrikDev", "Showing saved data. We'll retry in 30 s."],
    warn: ["alert-triangle", "Rate limit close", "Background sync paused until the minute resets."],
    info: ["download", "Update ready", "Postplant 0.4.0 installs when you close the game."],
  };
  function toast(kind) {
    let host = document.querySelector(".toasts");
    if (!host) { host = document.createElement("div"); host.className = "toasts"; document.body.appendChild(host); }
    const [ic, t, d] = TOASTS[kind] || TOASTS.info;
    const el = document.createElement("div");
    el.className = "toast " + kind;
    el.setAttribute("role", "status");
    el.innerHTML = `<i data-lucide="${ic}" class="icon"></i><div><div class="t">${t}</div><div class="d">${d}</div></div><button type="button" class="btn btn-ghost btn-icon btn-sm" aria-label="Dismiss" onclick="this.closest('.toast').remove()"><i data-lucide="x" class="icon"></i></button><span class="timer"></span>`;
    host.appendChild(el);
    icons();
    setTimeout(() => el.remove(), 5000);
  }

  /* ---------- Fill [data-agents] grids ---------- */
  function fillAgents() {
    document.querySelectorAll("[data-agent-grid]").forEach((host) => {
      const locked = (host.dataset.locked || "").split(",").filter(Boolean);
      const picked = (host.dataset.picked || "").split(",").filter(Boolean);
      const pressed = (host.dataset.pressed || "").split(",").filter(Boolean);
      const sm = host.hasAttribute("data-sm");
      host.innerHTML = Object.entries(AGENTS).map(([r, list]) => `
        <div class="agent-row"><span class="role-pill" style="--role:var(--role-${r.toLowerCase()})">${r}s</span>
          <div class="agent-list">${list.map((a) => agentTile(a, { button: true, sm, locked: locked.includes(a), picked: picked.includes(a), pressed: pressed.includes(a) })).join("")}</div></div>`).join("");
    });
    document.querySelectorAll("[data-agent]").forEach((el) => {
      const a = el.dataset.agent;
      el.outerHTML = agentTile(a, { sm: el.hasAttribute("data-sm") });
    });
  }

  function icons() { if (window.lucide) window.lucide.createIcons({ attrs: { "stroke-width": 2 } }); }

  toolbar();
  fillAgents();
  apply(theme, font, cvd);
  document.dispatchEvent(new CustomEvent("pp:ready"));
  icons();
  window.PP.icons = icons;
  window.addEventListener("load", icons);
})();
