// Hero canvas: a dot field with a planted "spike" that beeps faster and faster.
// Each beep sends a ring through the field; the last one detonates in lime, then it resets.
(() => {
  const canvas = document.getElementById("hero-canvas");
  if (!canvas) return;
  const ctx = canvas.getContext("2d", { alpha: false });
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  let W = 0, H = 0, dpr = 1, dots = [], gap = 26, sx = 0, sy = 0;
  const mouse = { x: -9999, y: -9999 };
  const pulses = [];
  let nextBeep = 0, interval = 1.15, blastAt = -1, running = true, visible = true;

  function resize() {
    const r = canvas.getBoundingClientRect();
    dpr = Math.min(2, window.devicePixelRatio || 1);
    W = r.width; H = r.height;
    canvas.width = Math.round(W * dpr); canvas.height = Math.round(H * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    gap = W < 700 ? 22 : 26;
    const mobile = W < 860;
    sx = mobile ? W * 0.62 : W * 0.74;
    sy = mobile ? H * 0.24 : H * 0.36;
    dots = [];
    for (let y = gap / 2; y < H; y += gap) {
      for (let x = gap / 2; x < W; x += gap) {
        dots.push({ x, y, d: Math.hypot(x - sx, y - sy), a: Math.atan2(y - sy, x - sx) });
      }
    }
  }

  function schedule(t) {
    if (blastAt > 0) return;
    if (t >= nextBeep) {
      pulses.push({ t0: t, big: false });
      interval *= 0.84;
      if (interval < 0.16) {
        blastAt = t + 0.35;
      }
      nextBeep = t + interval;
    }
  }

  const buckets = 7;
  const paths = Array.from({ length: buckets }, () => []);

  function frame(now) {
    const t = now / 1000;
    if (!running) return;
    schedule(t);
    if (blastAt > 0 && t >= blastAt) {
      pulses.push({ t0: t, big: true });
      blastAt = -1;
      interval = 1.15;
      nextBeep = t + 2.6;
    }
    for (let i = pulses.length - 1; i >= 0; i--) if (t - pulses[i].t0 > (pulses[i].big ? 3.2 : 1.9)) pulses.splice(i, 1);

    ctx.fillStyle = "#0a0b0e";
    ctx.fillRect(0, 0, W, H);

    // radar rings (static)
    ctx.strokeStyle = "rgba(237,232,223,0.05)";
    ctx.lineWidth = 1;
    for (let r = 80; r < Math.max(W, H); r += 140) { ctx.beginPath(); ctx.arc(sx, sy, r, 0, Math.PI * 2); ctx.stroke(); }

    for (const p of paths) p.length = 0;
    let lime = [];
    for (const d of dots) {
      let push = 0, glow = 0, hot = 0;
      for (const p of pulses) {
        const age = t - p.t0;
        const speed = p.big ? 900 : 560;
        const r = age * speed;
        const sigma = p.big ? 70 : 34;
        const amp = (p.big ? 1.6 : 1) * Math.max(0, 1 - age / (p.big ? 3.2 : 1.9));
        const k = Math.exp(-((d.d - r) ** 2) / (2 * sigma * sigma)) * amp;
        push += k * (p.big ? 22 : 9);
        glow += k;
        if (p.big) hot += k;
      }
      let x = d.x + Math.cos(d.a) * push, y = d.y + Math.sin(d.a) * push;
      const md = Math.hypot(x - mouse.x, y - mouse.y);
      if (md < 150) {
        const f = (1 - md / 150) ** 2;
        x += ((x - mouse.x) / (md || 1)) * f * 20;
        y += ((y - mouse.y) / (md || 1)) * f * 20;
        glow += f * 0.9;
      }
      const base = Math.max(0, 0.22 - d.d / 5200);
      if (hot > 0.25 || glow > 0.85) lime.push(x, y, Math.min(1, glow));
      else {
        const b = Math.min(buckets - 1, Math.floor((base + glow * 0.8) * buckets * 1.4));
        paths[b].push(x, y);
      }
    }
    for (let b = 0; b < buckets; b++) {
      const arr = paths[b];
      if (!arr.length) continue;
      ctx.fillStyle = `rgba(237,232,223,${0.07 + b * 0.11})`;
      for (let i = 0; i < arr.length; i += 2) ctx.fillRect(arr[i] - 0.8, arr[i + 1] - 0.8, 1.6, 1.6);
    }
    for (let i = 0; i < lime.length; i += 3) {
      ctx.fillStyle = `rgba(215,255,58,${0.35 + lime[i + 2] * 0.65})`;
      ctx.fillRect(lime[i] - 1.2, lime[i + 1] - 1.2, 2.4, 2.4);
    }

    // pulse rings
    for (const p of pulses) {
      const age = t - p.t0;
      const r = age * (p.big ? 900 : 560);
      const a = Math.max(0, 1 - age / (p.big ? 3.2 : 1.9));
      ctx.strokeStyle = p.big ? `rgba(215,255,58,${a * 0.7})` : `rgba(237,232,223,${a * 0.22})`;
      ctx.lineWidth = p.big ? 2 : 1;
      ctx.beginPath(); ctx.arc(sx, sy, r, 0, Math.PI * 2); ctx.stroke();
    }

    // the spike
    const last = pulses.length ? t - pulses[pulses.length - 1].t0 : 9;
    const flash = Math.max(0, 1 - last * 5);
    const g = ctx.createRadialGradient(sx, sy, 0, sx, sy, 90);
    g.addColorStop(0, `rgba(215,255,58,${0.18 + flash * 0.4})`);
    g.addColorStop(1, "rgba(215,255,58,0)");
    ctx.fillStyle = g; ctx.beginPath(); ctx.arc(sx, sy, 90, 0, Math.PI * 2); ctx.fill();
    ctx.save();
    ctx.translate(sx, sy);
    ctx.rotate(Math.PI / 4 + t * 0.15);
    ctx.strokeStyle = "#d7ff3a"; ctx.lineWidth = 2;
    ctx.strokeRect(-17, -17, 34, 34);
    ctx.rotate(-t * 0.3);
    ctx.strokeStyle = "rgba(215,255,58,0.35)";
    ctx.strokeRect(-27, -27, 54, 54);
    ctx.restore();
    ctx.fillStyle = flash > 0.05 ? "#d7ff3a" : "rgba(215,255,58,0.75)";
    ctx.fillRect(sx - 2, sy - 9, 4, 12);
    ctx.fillRect(sx - 2, sy + 6, 4, 4);

    if (!reduce) requestAnimationFrame(frame);
  }

  function start() {
    if (running || reduce) return;
    running = true;
    requestAnimationFrame(frame);
  }

  resize();
  window.addEventListener("resize", () => { resize(); if (reduce) frame(performance.now()); });
  canvas.parentElement.addEventListener("pointermove", (e) => {
    const r = canvas.getBoundingClientRect();
    mouse.x = e.clientX - r.left; mouse.y = e.clientY - r.top;
  }, { passive: true });
  canvas.parentElement.addEventListener("pointerleave", () => { mouse.x = mouse.y = -9999; });

  if ("IntersectionObserver" in window) {
    new IntersectionObserver(([e]) => {
      visible = e.isIntersecting;
      if (visible && !document.hidden) start(); else running = false;
    }).observe(canvas);
  }
  document.addEventListener("visibilitychange", () => { if (document.hidden) running = false; else if (visible) start(); });

  requestAnimationFrame(frame);
})();
