// Shared behaviour for every page: brand preview, cursor, nav, reveals, split text,
// magnetic buttons, counters. Vanilla JS, no dependencies.
(() => {
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const fine = window.matchMedia("(hover: hover) and (pointer: fine)").matches;
  const store = {
    get(k) { try { return localStorage.getItem(k); } catch { return null; } },
    set(k, v) { try { localStorage.setItem(k, v); } catch { /* storage unavailable */ } },
  };

  /* ---------- Brand preview (temporary, remove once the brand is confirmed) ---------- */
  const NAMES = ["Postplant", "Afterspike", "Plantside", "Sitehold"];
  const LOGOS = { a: "Plant", b: "P-mark", c: "Pulse" };
  const params = new URLSearchParams(location.search);
  let logo = params.get("logo") || store.get("pp-logo") || "a";
  let name = params.get("name") || store.get("pp-name") || "Postplant";
  if (!LOGOS[logo]) logo = "a";
  if (!NAMES.some((n) => n.toLowerCase() === name.toLowerCase())) name = "Postplant";
  name = NAMES.find((n) => n.toLowerCase() === name.toLowerCase());

  function applyBrand() {
    document.querySelectorAll("use[data-logo]").forEach((u) => u.setAttribute("href", "#logo-" + logo));
    document.querySelectorAll("[data-brand-name]").forEach((el) => {
      el.textContent = el.dataset.brandName === "upper" ? name.toUpperCase() : name;
    });
    const title = document.querySelector("title");
    if (title && title.dataset.tpl) title.textContent = title.dataset.tpl.replace("{name}", name);
    document.querySelectorAll(".bopt").forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.logo === logo)));
    document.querySelectorAll(".bname").forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.name === name)));
    // Favicon follows the chosen logo.
    const sym = document.getElementById("logo-" + logo);
    const fav = document.querySelector("link[rel=icon]");
    if (sym && fav) {
      const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" color="#d7ff3a"><rect width="48" height="48" rx="12" fill="#0a0b0e"/><g transform="translate(6 6) scale(.75)">${sym.innerHTML}</g></svg>`;
      fav.href = "data:image/svg+xml," + encodeURIComponent(svg.replace(/currentColor/g, "#d7ff3a"));
    }
  }

  function buildSwitcher() {
    const wrap = document.createElement("div");
    wrap.className = "bswitch";
    wrap.innerHTML = `
      <button type="button" aria-expanded="false" aria-controls="bpanel">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M12 3v18M3 12h18"/></svg>
        Brand preview
      </button>
      <div class="bpanel" id="bpanel" role="dialog" aria-label="Brand preview">
        <h5>Pick a logo and name</h5>
        <p>Temporary panel for confirming the brand. Your choice is remembered on this device and can be shared with <code>?logo=b&amp;name=postplant</code>.</p>
        <div class="bgrid">
          ${Object.entries(LOGOS).map(([k, v]) => `<button type="button" class="bopt" data-logo="${k}" aria-pressed="false"><svg><use href="#logo-${k}"/></svg>${k.toUpperCase()} · ${v}</button>`).join("")}
        </div>
        <div class="bnames">
          ${NAMES.map((n) => `<button type="button" class="chip bname" data-name="${n}" aria-pressed="false">${n}</button>`).join("")}
        </div>
      </div>`;
    document.body.appendChild(wrap);
    const toggle = wrap.querySelector(":scope > button");
    toggle.addEventListener("click", () => {
      const open = wrap.classList.toggle("open");
      toggle.setAttribute("aria-expanded", String(open));
    });
    document.addEventListener("keydown", (e) => { if (e.key === "Escape") { wrap.classList.remove("open"); toggle.setAttribute("aria-expanded", "false"); } });
    wrap.querySelectorAll(".bopt").forEach((b) => b.addEventListener("click", () => { logo = b.dataset.logo; store.set("pp-logo", logo); applyBrand(); }));
    wrap.querySelectorAll(".bname").forEach((b) => b.addEventListener("click", () => { name = b.dataset.name; store.set("pp-name", name); applyBrand(); }));
  }

  /* ---------- Split text into animated words ---------- */
  function splitWords(el) {
    let i = 0;
    const walk = (node) => {
      [...node.childNodes].forEach((child) => {
        if (child.nodeType === 3) {
          const frag = document.createDocumentFragment();
          child.textContent.split(/(\s+)/).forEach((part) => {
            if (!part) return;
            if (/^\s+$/.test(part)) { frag.appendChild(document.createTextNode(" ")); return; }
            const w = document.createElement("span");
            w.className = "w";
            const inner = document.createElement("span");
            inner.textContent = part;
            inner.style.setProperty("--i", i++);
            w.appendChild(inner);
            frag.appendChild(w);
          });
          child.replaceWith(frag);
        } else if (child.nodeType === 1 && child.tagName !== "BR") {
          walk(child);
        }
      });
    };
    walk(el);
  }

  /* ---------- Reveal on scroll ---------- */
  function reveals() {
    document.querySelectorAll("[data-split]").forEach((el) => { el.classList.add("split"); splitWords(el); });
    const targets = document.querySelectorAll("[data-reveal], .split, [data-count], .step");
    if (reduce || !("IntersectionObserver" in window)) {
      targets.forEach((t) => { t.classList.add("in"); if (t.dataset.count) count(t, true); });
      return;
    }
    const io = new IntersectionObserver((entries) => {
      entries.forEach((e) => {
        if (!e.isIntersecting) return;
        e.target.classList.add("in");
        if (e.target.dataset.count) count(e.target);
        io.unobserve(e.target);
      });
    }, { threshold: 0.18, rootMargin: "0px 0px -6% 0px" });
    targets.forEach((t) => io.observe(t));
    // Safety net: if the observer never fires (some embedded/background views), reveal what's on screen.
    const sweep = () => targets.forEach((t) => {
      if (t.classList.contains("in")) return;
      const r = t.getBoundingClientRect();
      if (r.top < innerHeight * 0.95 && r.bottom > 0) {
        t.classList.add("in"); if (t.dataset.count) count(t); io.unobserve(t);
      }
    });
    setTimeout(sweep, 1500);
    let pending = false;
    window.addEventListener("scroll", () => {
      if (pending) return; pending = true;
      setTimeout(() => { pending = false; sweep(); }, 400);
    }, { passive: true });
  }

  /* ---------- Counters ---------- */
  function count(el, instant) {
    const to = parseFloat(el.dataset.count);
    const dec = (el.dataset.count.split(".")[1] || "").length;
    const suffix = el.dataset.suffix || "";
    if (instant) { el.textContent = to.toFixed(dec) + suffix; return; }
    const dur = 1600, t0 = performance.now();
    const tick = (t) => {
      const p = Math.min(1, (t - t0) / dur);
      const e = 1 - Math.pow(1 - p, 4);
      el.textContent = (to * e).toFixed(dec) + suffix;
      if (p < 1) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }
  window.ppCount = count;

  /* ---------- Nav ---------- */
  function nav() {
    const n = document.querySelector(".nav");
    if (!n) return;
    let last = 0;
    const on = () => {
      const y = window.scrollY;
      n.classList.toggle("is-scrolled", y > 30);
      n.classList.toggle("is-hidden", y > 400 && y > last + 4);
      if (y < last - 4) n.classList.remove("is-hidden");
      last = y;
    };
    window.addEventListener("scroll", on, { passive: true });
    on();
  }

  /* ---------- Cursor + magnetic buttons ---------- */
  function cursor() {
    if (!fine || reduce) return;
    const ring = document.createElement("div"); ring.className = "cursor";
    const dot = document.createElement("div"); dot.className = "cursor-dot";
    document.body.append(ring, dot);
    let x = innerWidth / 2, y = innerHeight / 2, rx = x, ry = y;
    window.addEventListener("pointermove", (e) => { x = e.clientX; y = e.clientY; dot.style.transform = `translate(${x}px, ${y}px)`; }, { passive: true });
    const loop = () => {
      rx += (x - rx) * 0.18; ry += (y - ry) * 0.18;
      ring.style.transform = `translate(${rx}px, ${ry}px)`;
      requestAnimationFrame(loop);
    };
    loop();
    document.addEventListener("pointerover", (e) => { if (e.target.closest("a, button, summary, [data-hover]")) ring.classList.add("is-hover"); });
    document.addEventListener("pointerout", (e) => { if (e.target.closest("a, button, summary, [data-hover]")) ring.classList.remove("is-hover"); });

    document.querySelectorAll("[data-magnetic]").forEach((el) => {
      const strength = parseFloat(el.dataset.magnetic) || 0.35;
      el.addEventListener("pointermove", (e) => {
        const r = el.getBoundingClientRect();
        const dx = e.clientX - (r.left + r.width / 2), dy = e.clientY - (r.top + r.height / 2);
        el.style.transform = `translate(${dx * strength}px, ${dy * strength}px)`;
      });
      el.addEventListener("pointerleave", () => { el.style.transform = ""; });
    });
  }

  /* ---------- Card spotlight + tilt ---------- */
  function spotlight() {
    if (!fine || reduce) return;
    document.querySelectorAll("[data-tilt]").forEach((card) => {
      card.addEventListener("pointermove", (e) => {
        const r = card.getBoundingClientRect();
        const px = (e.clientX - r.left) / r.width, py = (e.clientY - r.top) / r.height;
        card.style.setProperty("--mx", px * 100 + "%");
        card.style.setProperty("--my", py * 100 + "%");
        card.style.transform = `perspective(900px) rotateX(${(0.5 - py) * 6}deg) rotateY(${(px - 0.5) * 8}deg)`;
      });
      card.addEventListener("pointerleave", () => { card.style.transform = ""; });
    });
  }

  /* ---------- Spike timer (decorative) ---------- */
  function timers() {
    document.querySelectorAll("[data-timer]").forEach((el) => {
      const total = 45;
      let start = performance.now();
      const tick = (t) => {
        let left = total - (t - start) / 1000;
        if (left <= 0) { start = t; left = total; }
        el.textContent = "00:" + left.toFixed(2).padStart(5, "0");
        el.classList.toggle("low", left < 10);
        requestAnimationFrame(tick);
      };
      if (reduce) el.textContent = "00:45.00"; else requestAnimationFrame(tick);
    });
  }

  document.querySelectorAll("[data-year]").forEach((el) => { el.textContent = new Date().getFullYear(); });
  // Repository links come from config.js so the owner only changes one place.
  const repo = (window.POSTPLANT && window.POSTPLANT.REPO) || "OWNER/postplant";
  document.querySelectorAll("[data-repo]").forEach((a) => {
    a.href = "https://github.com/" + repo + (a.dataset.repo || "");
    a.rel = "noopener";
  });
  document.querySelectorAll("[data-type]").forEach((el) => { el.dataset.type = el.dataset.type.replace(/OWNER\/postplant/g, repo); });
  buildSwitcher();
  applyBrand();
  reveals();
  nav();
  cursor();
  spotlight();
  timers();
})();
