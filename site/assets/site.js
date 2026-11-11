// Shared behaviour for every page: cursor, nav, reveals, split text,
// magnetic buttons, counters. Vanilla JS, no dependencies.
(() => {
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const fine = window.matchMedia("(hover: hover) and (pointer: fine)").matches;
  const store = {
    get(k) { try { return localStorage.getItem(k); } catch { return null; } },
    set(k, v) { try { localStorage.setItem(k, v); } catch { /* storage unavailable */ } },
  };

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
  reveals();
  nav();
  cursor();
  spotlight();
  timers();
})();
