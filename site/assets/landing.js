// Landing page demos: overlay mock, Comp Advisor demo, heatmap, horizontal scroll,
// quota gauge, live-match toggle, terminal typing.
(() => {
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const onView = (el, fn, threshold = 0.3) => {
    if (!el) return;
    if (!("IntersectionObserver" in window)) { fn(); return; }
    const io = new IntersectionObserver(([e]) => { if (e.isIntersecting) { fn(); io.disconnect(); } }, { threshold });
    io.observe(el);
  };

  /* ---------- Overlay mock: cycle panels ---------- */
  (() => {
    const ov = document.querySelector(".ov");
    if (!ov) return;
    const panels = [...ov.querySelectorAll(".ov-panel")];
    const tabs = [...ov.querySelectorAll(".ov-tabs i")];
    const label = ov.querySelector("[data-ov-label]");
    let i = 0;
    const show = (n) => {
      panels.forEach((p, k) => p.classList.toggle("on", k === n));
      tabs.forEach((t, k) => t.classList.toggle("on", k === n));
      if (label) label.textContent = panels[n].dataset.label;
      panels[n].querySelectorAll("[data-count]").forEach((c) => window.ppCount && window.ppCount(c, reduce));
    };
    show(0);
    if (!reduce) setInterval(() => { i = (i + 1) % panels.length; show(i); }, 3200);
  })();

  /* ---------- Comp Advisor demo (illustrative data only) ---------- */
  (() => {
    const root = document.getElementById("advisor");
    if (!root) return;
    const ROLES = {
      Duelist: ["Jett", "Phoenix", "Reyna", "Raze", "Yoru", "Neon", "Iso", "Waylay", "Hush"],
      Initiator: ["Sova", "Breach", "Skye", "KAY/O", "Fade", "Gekko", "Tejo"],
      Controller: ["Brimstone", "Omen", "Viper", "Astra", "Harbor", "Clove", "Miks"],
      Sentinel: ["Sage", "Cypher", "Killjoy", "Chamber", "Deadlock", "Vyse", "Veto"],
    };
    const roleOf = {};
    Object.entries(ROLES).forEach(([r, list]) => list.forEach((a) => (roleOf[a] = r)));
    const NEW = new Set(["Hush"]);
    // Illustrative map profiles. Pro comps seen in 2026 for Lotus, Bind, Summit and Sunset; others use role coverage only.
    const MAPS = {
      Abyss: { want: { Controller: 1, Initiator: 1, Sentinel: 1, Duelist: 1 }, flex: "Initiator", staples: [], trait: "open edges; mobility and recon matter" },
      Bind: { want: { Controller: 2, Initiator: 1, Duelist: 2, Sentinel: 0 }, flex: null, staples: ["Brimstone", "Viper", "Skye", "Neon", "Waylay"], trait: "no mid, two one-way teleporters" },
      Corrode: { want: { Controller: 1, Initiator: 1, Sentinel: 1, Duelist: 1 }, flex: "Sentinel", staples: [], trait: "back in the pool since 13.08" },
      Haven: { want: { Controller: 1, Initiator: 1, Sentinel: 1, Duelist: 1 }, flex: "Initiator", staples: [], trait: "three sites; info wins rotations" },
      Lotus: { want: { Controller: 2, Initiator: 1, Sentinel: 1, Duelist: 1 }, flex: null, staples: ["Omen", "Viper", "Fade", "Raze", "Vyse"], trait: "three sites, rotating doors" },
      Summit: { want: { Controller: 1, Initiator: 1, Sentinel: 2, Duelist: 1 }, flex: null, staples: ["Neon", "Fade"], trait: "retractable walls on A, B and mid" },
      Sunset: { want: { Controller: 1, Initiator: 2, Sentinel: 1, Duelist: 1 }, flex: null, staples: ["Omen", "Neon", "Cypher", "Breach", "Sova"], trait: "tight mid, fast retakes" },
    };
    let map = "Lotus";
    const picks = [];

    const mapChips = root.querySelector("[data-maps]");
    const slots = root.querySelector("[data-slots]");
    const grid = root.querySelector("[data-agents]");
    const out = root.querySelector("[data-sugg]");
    const comp = root.querySelector("[data-comp]");
    const conf = root.querySelector("[data-conf]");

    mapChips.innerHTML = Object.keys(MAPS).map((m) => `<button type="button" class="chip" data-map="${m}" aria-pressed="${m === map}">${m}</button>`).join("");
    grid.innerHTML = Object.entries(ROLES).map(([r, list]) => `
      <div class="role-row"><span class="mono mute">${r}s</span>
        <div class="agent-list">${list.map((a) => `<button type="button" class="agent" data-agent="${a}">${a}</button>`).join("")}</div>
      </div>`).join("");

    mapChips.addEventListener("click", (e) => {
      const b = e.target.closest("[data-map]"); if (!b) return;
      map = b.dataset.map;
      mapChips.querySelectorAll("[data-map]").forEach((x) => x.setAttribute("aria-pressed", String(x === b)));
      render();
    });
    grid.addEventListener("click", (e) => {
      const b = e.target.closest("[data-agent]"); if (!b || picks.length >= 4) return;
      picks.push(b.dataset.agent); render();
    });
    slots.addEventListener("click", (e) => {
      const b = e.target.closest("[data-remove]"); if (!b) return;
      picks.splice(+b.dataset.remove, 1); render();
    });

    function counts(team) {
      const c = { Duelist: 0, Initiator: 0, Controller: 0, Sentinel: 0 };
      team.forEach((a) => c[roleOf[a]]++);
      return c;
    }

    function score(agent, team) {
      const m = MAPS[map];
      const role = roleOf[agent];
      const c = counts(team);
      let s = 30, why = [];
      const gap = (m.want[role] || 0) - c[role];
      if (gap > 0) { s += 34 + gap * 8; why.push(c[role] === 0 ? `No ${role.toLowerCase()} yet` : `${map} wants ${m.want[role]} ${role.toLowerCase()}s`); }
      else if (m.flex === role && gap === 0) { s += 10; why.push(`Good flex pick on ${map}`); }
      else { s -= 26 + Math.abs(gap) * 10; }
      if (c.Controller === 0 && role === "Controller") { s += 10; }
      if (role === "Duelist" && c.Duelist >= 2) { s -= 30; why.push("Already two duelists"); }
      if (m.staples.includes(agent)) { s += 18; why.push(`Common on ${map} in 2026 pro play`); }
      if (!m.staples.length && why.length < 2) why.push("Role coverage only (thin data)");
      if (NEW.has(agent)) why.push("New agent, limited data");
      // tiny stable tie-breaker
      s += (agent.charCodeAt(0) % 7) / 10;
      return { agent, role, s, why };
    }

    function bestComp(team) {
      const t = [...team];
      while (t.length < 5) {
        const pool = Object.keys(roleOf).filter((a) => !t.includes(a));
        const best = pool.map((a) => score(a, t)).sort((x, y) => y.s - x.s)[0];
        t.push(best.agent);
      }
      return t;
    }

    function render() {
      const m = MAPS[map];
      slots.innerHTML = [0, 1, 2, 3].map((k) => picks[k]
        ? `<div class="slot filled"><span class="glyph">${picks[k][0]}</span><span>${picks[k]}</span><span class="role">${roleOf[picks[k]]}</span><button type="button" class="x" data-remove="${k}" aria-label="Remove ${picks[k]}">×</button></div>`
        : `<div class="slot"><span class="glyph" style="opacity:.15">?</span><span>Teammate ${k + 1}</span></div>`).join("")
        + `<div class="slot me"><span class="glyph">★</span><span>You</span><span class="role">next pick</span></div>`;
      grid.querySelectorAll("[data-agent]").forEach((b) => { b.disabled = picks.includes(b.dataset.agent) || picks.length >= 4; });

      const pool = Object.keys(roleOf).filter((a) => !picks.includes(a));
      const ranked = pool.map((a) => score(a, picks)).sort((x, y) => y.s - x.s).slice(0, 3);
      const max = ranked[0].s;
      out.innerHTML = ranked.map((r, i) => `
        <li style="--i:${i}">
          <div class="top"><span class="name">${r.agent}</span><span class="badge ${i === 0 ? "hi" : ""}">${r.role}${NEW.has(r.agent) ? " · new" : ""}</span></div>
          <div class="why">${r.why.slice(0, 2).join(" · ") || "Balanced pick"}</div>
          <div class="bar"><i style="--w:${Math.max(18, (r.s / max) * 100)}%; --i:${i}"></i></div>
        </li>`).join("");
      const full = bestComp(picks);
      comp.innerHTML = full.map((a, k) => `<span class="${k >= picks.length ? "new" : ""}">${a}</span>`).join("");
      conf.innerHTML = m.staples.length
        ? `<span class="badge hi">Medium confidence</span> <span class="mute" style="font-size:.85rem">· ${m.trait}</span>`
        : `<span class="badge lo">Low confidence · role coverage</span> <span class="mute" style="font-size:.85rem">· ${m.trait}</span>`;
    }
    render();
  })();

  /* ---------- Heatmap demo ---------- */
  (() => {
    const canvas = document.getElementById("heat");
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    let side = "atk", W = 0, H = 0, t0 = 0, raf = 0;
    let seed = 7;
    const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
    const gauss = () => { let u = 0; for (let i = 0; i < 4; i++) u += rnd(); return (u - 2) / 2; };
    function points(which) {
      seed = which === "atk" ? 11 : 29;
      const clusters = which === "atk"
        ? [[0.22, 0.28, 0.6, "k"], [0.3, 0.38, 0.5, "d"], [0.5, 0.5, 0.45, "k"], [0.74, 0.3, 0.55, "k"], [0.66, 0.42, 0.4, "d"], [0.78, 0.7, 0.35, "d"]]
        : [[0.24, 0.24, 0.55, "k"], [0.2, 0.36, 0.35, "d"], [0.52, 0.46, 0.55, "d"], [0.76, 0.26, 0.5, "k"], [0.72, 0.62, 0.4, "k"], [0.5, 0.8, 0.3, "d"]];
      const pts = [];
      clusters.forEach(([cx, cy, w, kind]) => {
        const n = Math.round(26 * w + 8);
        for (let i = 0; i < n; i++) pts.push({ x: cx + gauss() * 0.075, y: cy + gauss() * 0.075, k: kind, delay: rnd() * 1.4 });
      });
      return pts;
    }
    let pts = points(side);
    function resize() {
      const r = canvas.getBoundingClientRect();
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      W = r.width; H = r.height;
      canvas.width = W * dpr; canvas.height = H * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    }
    function layout() {
      // Invented, abstract layout (not a real game map).
      ctx.strokeStyle = "rgba(237,232,223,0.16)"; ctx.lineWidth = 1.2;
      const R = (x, y, w, h) => { ctx.beginPath(); ctx.roundRect(x * W, y * H, w * W, h * H, 10); ctx.stroke(); };
      R(0.1, 0.12, 0.28, 0.3); R(0.62, 0.12, 0.28, 0.3); R(0.36, 0.36, 0.28, 0.24); R(0.18, 0.58, 0.64, 0.12); R(0.4, 0.72, 0.2, 0.2);
      ctx.fillStyle = "rgba(237,232,223,0.35)"; ctx.font = "600 11px 'JetBrains Mono', monospace";
      ctx.fillText("A SITE", 0.12 * W, 0.16 * H + 6); ctx.fillText("B SITE", 0.64 * W, 0.16 * H + 6); ctx.fillText("MID", 0.38 * W, 0.4 * H + 6); ctx.fillText(side === "atk" ? "ATTACK SPAWN" : "DEFENSE SPAWN", 0.42 * W, 0.76 * H + 6);
      ctx.setLineDash([3, 6]); ctx.strokeStyle = "rgba(237,232,223,0.07)";
      for (let i = 1; i < 10; i++) { ctx.beginPath(); ctx.moveTo((i / 10) * W, 0); ctx.lineTo((i / 10) * W, H); ctx.stroke(); ctx.beginPath(); ctx.moveTo(0, (i / 10) * H); ctx.lineTo(W, (i / 10) * H); ctx.stroke(); }
      ctx.setLineDash([]);
    }
    function draw(now) {
      const t = (now - t0) / 1000;
      ctx.clearRect(0, 0, W, H);
      layout();
      ctx.globalCompositeOperation = "lighter";
      let done = true;
      for (const p of pts) {
        const a = Math.min(1, Math.max(0, (t - p.delay) / 0.6));
        if (a < 1) done = false;
        if (a <= 0) continue;
        const e = 1 - Math.pow(1 - a, 3);
        const x = p.x * W, y = p.y * H;
        const col = p.k === "k" ? "215,255,58" : "255,91,58";
        const g = ctx.createRadialGradient(x, y, 0, x, y, 26 * e);
        g.addColorStop(0, `rgba(${col},${0.07 * e})`); g.addColorStop(1, `rgba(${col},0)`);
        ctx.fillStyle = g; ctx.beginPath(); ctx.arc(x, y, 30 * e, 0, Math.PI * 2); ctx.fill();
        ctx.fillStyle = `rgba(${col},${0.55 * e})`; ctx.beginPath(); ctx.arc(x, y, 1.8, 0, Math.PI * 2); ctx.fill();
      }
      ctx.globalCompositeOperation = "source-over";
      if (!done && !reduce) raf = requestAnimationFrame(draw);
    }
    function play() { cancelAnimationFrame(raf); t0 = performance.now() - (reduce ? 99999 : 0); raf = requestAnimationFrame(draw); }
    resize();
    window.addEventListener("resize", () => { resize(); play(); });
    document.querySelectorAll("[data-side]").forEach((b) => b.addEventListener("click", () => {
      side = b.dataset.side; pts = points(side);
      document.querySelectorAll("[data-side]").forEach((x) => x.setAttribute("aria-pressed", String(x === b)));
      play();
    }));
    layout();
    onView(canvas, play, 0.35);
  })();

  /* ---------- Horizontal pinned scroll ---------- */
  (() => {
    const sec = document.querySelector(".hscroll");
    if (!sec) return;
    const track = sec.querySelector(".htrack");
    const bar = sec.querySelector(".hscroll-progress");
    let dist = 0;
    const mq = window.matchMedia("(max-width: 760px)");
    function measure() {
      if (mq.matches) { sec.style.height = ""; track.style.transform = ""; return; }
      dist = Math.max(0, track.scrollWidth - window.innerWidth);
      sec.style.height = window.innerHeight + dist + "px";
      update();
    }
    function update() {
      if (mq.matches) return;
      const r = sec.getBoundingClientRect();
      const p = Math.min(1, Math.max(0, -r.top / (dist || 1)));
      track.style.transform = `translate3d(${-p * dist}px,0,0)`;
      if (bar) bar.style.setProperty("--p", p.toFixed(3));
    }
    window.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", measure);
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(measure);
    measure();
  })();

  /* ---------- Quota gauge ---------- */
  (() => {
    const g = document.querySelector(".gauge");
    if (!g) return;
    const fill = g.querySelector(".fill");
    const R = 90, arc = 2 * Math.PI * R * 0.75;
    g.querySelectorAll(".track, .fill").forEach((c) => c.setAttribute("stroke-dasharray", `${arc} ${2 * Math.PI * R}`));
    const cap = g.querySelector(".cap");
    const capLen = arc * 0.012;
    cap.setAttribute("stroke-dasharray", `${capLen} ${2 * Math.PI * R}`);
    cap.setAttribute("stroke-dashoffset", String(-arc * 0.8));
    fill.setAttribute("stroke-dashoffset", String(arc));
    onView(g, () => { fill.setAttribute("stroke-dashoffset", String(arc * (1 - 4 / 30))); }, 0.4);
  })();

  /* ---------- Live toggle ---------- */
  (() => {
    const live = document.querySelector(".live");
    if (!live) return;
    const t = live.querySelector(".toggle");
    const warn = live.querySelector(".warn");
    const state = live.querySelector("[data-live-state]");
    t.addEventListener("click", () => {
      const on = t.getAttribute("aria-checked") !== "true";
      t.setAttribute("aria-checked", String(on));
      warn.classList.toggle("show", on);
      live.classList.toggle("on", on);
      state.textContent = on ? "On (demo)" : "Off";
    });
  })();

  /* ---------- Terminal typing ---------- */
  (() => {
    const pre = document.querySelector("[data-type]");
    if (!pre) return;
    const lines = JSON.parse(pre.dataset.type);
    const render = (html) => { pre.innerHTML = html + '<span class="caret"></span>'; };
    if (reduce) { render(lines.map((l) => l.html).join("\n")); return; }
    render("");
    onView(pre, async () => {
      let done = "";
      for (const l of lines) {
        if (l.out) { done += l.html + "\n"; render(done); await new Promise((r) => setTimeout(r, 260)); continue; }
        const prefix = '<span class="p">❯</span> ';
        for (let i = 1; i <= l.text.length; i++) {
          render(done + prefix + l.text.slice(0, i).replace(/</g, "&lt;"));
          await new Promise((r) => setTimeout(r, 24 + Math.random() * 40));
        }
        done += prefix + l.text.replace(/</g, "&lt;") + "\n";
        await new Promise((r) => setTimeout(r, 380));
      }
      render(done);
    }, 0.5);
  })();
})();
