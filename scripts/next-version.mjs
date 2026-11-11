#!/usr/bin/env node
// Works out the next release version from Conventional Commits since the last v* tag.
//
//   node scripts/next-version.mjs [--bump auto|patch|minor|major] [--notes notes.md]
//
// Prints key=value lines (version, tag, skip) so a workflow can append them to $GITHUB_OUTPUT.
// Rules: "feat" → minor, anything else → patch, "!" or "BREAKING CHANGE" → major
// (minor while the version is below 1.0). With no tag yet, the first release is the
// version already in package.json. Commits that only touch docs/site/design/CI don't count.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const args = process.argv.slice(2);
const opt = (name, dflt) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 && args[i + 1] ? args[i + 1] : dflt;
};
const bumpArg = opt("bump", "auto");
const notesFile = opt("notes", null);

const git = (...a) => execFileSync("git", a, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
const parse = (v) => v.replace(/^v/, "").split(/[.-]/).slice(0, 3).map((n) => parseInt(n, 10) || 0);

let lastTag = "";
try {
  lastTag = git("describe", "--tags", "--abbrev=0", "--match", "v[0-9]*");
} catch {
  /* no release yet */
}

const APP_PATHS = ["src/", "src-tauri/", "data/", "index.html", "overlay.html", "package.json", "package-lock.json", "vite.config.ts", "svelte.config.js"];
const range = lastTag ? `${lastTag}..HEAD` : "HEAD";
const log = git("log", range, "--no-merges", "--format=%H%x1f%s%x1f%b%x1e");
const commits = log
  .split("\x1e")
  .map((c) => c.trim())
  .filter(Boolean)
  .map((c) => {
    const [hash, subject, body] = c.split("\x1f");
    let files = [];
    try {
      files = git("show", "--name-only", "--format=", hash).split("\n").filter(Boolean);
    } catch {
      /* shallow edge */
    }
    return { hash, subject, body: body ?? "", app: files.some((f) => APP_PATHS.some((p) => f === p || f.startsWith(p))) };
  });

const relevant = commits.filter((c) => c.app && !/\[skip release\]/i.test(c.subject + c.body));
const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));

let version;
let skip = false;
if (!lastTag) {
  version = pkg.version;
} else if (!relevant.length && bumpArg === "auto") {
  version = lastTag.replace(/^v/, "");
  skip = true;
} else {
  let [maj, min, pat] = parse(lastTag);
  let level = bumpArg;
  if (level === "auto") {
    const breaking = relevant.some((c) => /^\w+(\(.+\))?!:/.test(c.subject) || /BREAKING[ -]CHANGE/.test(c.body));
    const feat = relevant.some((c) => /^feat(\(.+\))?!?:/i.test(c.subject));
    level = breaking ? "major" : feat ? "minor" : "patch";
  }
  if (level === "major" && maj === 0) level = "minor";
  if (level === "major") [maj, min, pat] = [maj + 1, 0, 0];
  else if (level === "minor") [min, pat] = [min + 1, 0];
  else pat += 1;
  version = `${maj}.${min}.${pat}`;
}

if (notesFile) {
  const groups = { Features: [], Fixes: [], Other: [] };
  for (const c of relevant.length ? relevant : commits) {
    const m = c.subject.match(/^(\w+)(?:\(([^)]+)\))?!?:\s*(.+)$/);
    const type = m?.[1]?.toLowerCase();
    const text = m ? (m[2] ? `**${m[2]}:** ${m[3]}` : m[3]) : c.subject;
    (type === "feat" ? groups.Features : type === "fix" ? groups.Fixes : groups.Other).push(`- ${text} (${c.hash.slice(0, 7)})`);
  }
  let md = "";
  for (const [k, v] of Object.entries(groups)) if (v.length) md += `### ${k}\n\n${v.join("\n")}\n\n`;
  md +=
    "### Download\n\n" +
    "- **Installer:** `Postplant_" + version + "_x64-setup.exe` (installs for your user only, no admin needed)\n" +
    "- **Portable:** `Postplant_" + version + "_x64-portable.zip` (keeps its data next to the exe)\n" +
    "- Check your download against `SHA256SUMS.txt`.\n\n" +
    "Windows SmartScreen may warn because the build isn't code-signed yet: choose *More info → Run anyway*.\n" +
    (lastTag ? `\n**Full changelog:** ${lastTag}...v${version}\n` : "");
  writeFileSync(notesFile, md);
}

console.log(`version=${version}`);
console.log(`tag=v${version}`);
console.log(`skip=${skip}`);
