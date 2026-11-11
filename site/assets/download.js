// Download page: reads the latest release from the GitHub API and fills in the cards.
// Falls back to the Releases page if the API is unreachable or no release exists yet.
(() => {
  const cfg = window.POSTPLANT || {};
  const repo = cfg.REPO || "OWNER/postplant";
  const A = cfg.ASSETS || {};
  const releasesUrl = `https://github.com/${repo}/releases`;
  let channel = "stable";
  const cache = {};

  const $ = (s) => document.querySelector(s);
  const fmtSize = (b) => (b > 1e6 ? (b / 1048576).toFixed(1) + " MB" : (b / 1024).toFixed(0) + " KB");
  const fmtDate = (d) => new Date(d).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });

  const store = {
    get(k) { try { return JSON.parse(sessionStorage.getItem(k)); } catch { return null; } },
    set(k, v) { try { sessionStorage.setItem(k, JSON.stringify(v)); } catch { /* ignore */ } },
  };

  async function fetchRelease(ch) {
    if (cache[ch]) return cache[ch];
    const key = `pp-rel-${repo}-${ch}`;
    const hit = store.get(key);
    if (hit && Date.now() - hit.t < 3600e3) return (cache[ch] = hit.v);
    const url = ch === "nightly"
      ? `https://api.github.com/repos/${repo}/releases/tags/nightly`
      : `https://api.github.com/repos/${repo}/releases/latest`;
    const res = await fetch(url, { headers: { Accept: "application/vnd.github+json" } });
    if (!res.ok) throw new Error(String(res.status));
    const v = await res.json();
    store.set(key, { t: Date.now(), v });
    return (cache[ch] = v);
  }

  function setCard(kind, asset) {
    const card = document.querySelector(`[data-kind="${kind}"]`);
    if (!card) return;
    const btn = card.querySelector(".btn");
    const size = card.querySelector("[data-size]");
    const file = card.querySelector("[data-file]");
    if (asset) {
      btn.href = asset.browser_download_url;
      btn.removeAttribute("aria-disabled");
      size.textContent = fmtSize(asset.size);
      file.textContent = asset.name;
    } else {
      btn.href = releasesUrl;
      size.textContent = "—";
      file.textContent = "Not in this release";
    }
  }

  async function load() {
    const status = $("[data-status]");
    const ver = $("[data-version]");
    const date = $("[data-date]");
    const notes = $("[data-notes]");
    status.textContent = "Checking the latest release…";
    try {
      if (/^OWNER\//.test(repo)) throw new Error("repository not configured");
      const rel = await fetchRelease(channel);
      const assets = rel.assets || [];
      const find = (re) => assets.find((a) => re && re.test(a.name));
      setCard("installer", find(A.installer));
      setCard("offline", find(A.offline));
      setCard("portable", find(A.portable));
      ver.textContent = rel.tag_name || "—";
      date.textContent = rel.published_at ? fmtDate(rel.published_at) : "—";
      const sums = find(A.checksums);
      $("[data-sums]").href = sums ? sums.browser_download_url : releasesUrl;
      notes.innerHTML = renderNotes(rel.body || "No release notes.");
      status.textContent = channel === "nightly" ? "Nightly builds are untested. Use them to help find bugs." : "";
    } catch (e) {
      ["installer", "offline", "portable"].forEach((k) => setCard(k, null));
      document.querySelectorAll("[data-file]").forEach((f) => { f.textContent = "Coming with the first release"; });
      ver.textContent = "Not released yet";
      date.textContent = "—";
      $("[data-sums]").href = releasesUrl;
      notes.innerHTML = '<p>The first release hasn\'t been published yet. Watch the repository on GitHub to hear when it is.</p>';
      status.textContent = "";
    }
  }

  // Minimal, safe Markdown → HTML for release notes (headings, bullets, bold, code, links).
  function renderNotes(md) {
    const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    const inline = (s) => esc(s)
      .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
      .replace(/`([^`]+)`/g, "<code>$1</code>")
      .replace(/\[([^\]]+)\]\((https:\/\/[^)\s]+)\)/g, '<a class="lime" href="$2" rel="noopener">$1</a>');
    let html = "", inList = false;
    md.split(/\r?\n/).slice(0, 80).forEach((line) => {
      const h = line.match(/^(#{1,3})\s+(.*)/);
      const li = line.match(/^\s*[-*]\s+(.*)/);
      if (li) { if (!inList) { html += "<ul>"; inList = true; } html += `<li>${inline(li[1])}</li>`; return; }
      if (inList) { html += "</ul>"; inList = false; }
      if (h) html += `<h3>${inline(h[2])}</h3>`;
      else if (line.trim()) html += `<p>${inline(line)}</p>`;
    });
    if (inList) html += "</ul>";
    return html;
  }

  /* Channel switch */
  const sw = $(".channel");
  if (sw) {
    const bg = sw.querySelector(".pillbg");
    const btns = [...sw.querySelectorAll("button")];
    const place = (b) => { bg.style.width = b.offsetWidth + "px"; bg.style.transform = `translateX(${b.offsetLeft - 4}px)`; };
    btns.forEach((b) => b.addEventListener("click", () => {
      channel = b.dataset.channel;
      btns.forEach((x) => x.setAttribute("aria-pressed", String(x === b)));
      place(b); load();
    }));
    const init = () => place(btns.find((b) => b.getAttribute("aria-pressed") === "true"));
    if (document.fonts && document.fonts.ready) document.fonts.ready.then(init);
    init();
    window.addEventListener("resize", init);
  }

  /* OS hint */
  const ua = navigator.userAgent;
  const isWin = /Windows/i.test(ua);
  const note = $("[data-os-note]");
  if (!isWin && note) {
    note.textContent = /Mac|iPhone|iPad|Android|Linux/i.test(ua)
      ? "You're not on Windows. The game and the overlay are Windows-only, so these downloads are for your gaming PC."
      : "These downloads are for Windows 10 and 11.";
    note.classList.add("show");
  }

  /* Copy buttons */
  document.querySelectorAll("[data-copy]").forEach((b) => b.addEventListener("click", async () => {
    const txt = document.querySelector(b.dataset.copy).innerText;
    try { await navigator.clipboard.writeText(txt); b.textContent = "Copied"; } catch { b.textContent = "Select & copy"; }
    setTimeout(() => (b.textContent = "Copy"), 1600);
  }));

  /* Step line fill on scroll */
  const steps = $(".steps");
  if (steps) {
    const fill = steps.querySelector(".line-fill");
    const on = () => {
      const r = steps.getBoundingClientRect();
      const p = Math.min(1, Math.max(0, (window.innerHeight * 0.7 - r.top) / r.height));
      fill.style.setProperty("--h", p * (r.height - 20) + "px");
    };
    window.addEventListener("scroll", on, { passive: true });
    on();
  }

  load();
})();
