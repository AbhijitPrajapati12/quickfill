#!/usr/bin/env node
// Bump the app version everywhere, commit, tag and push.
// Pushing the tag starts .github/workflows/release.yml, which builds the
// installer and publishes the GitHub release.
//
//   pnpm release 0.2.0          exact version
//   pnpm release patch|minor|major
//   pnpm release 0.2.0 --dry-run   show what would change, touch nothing
//   pnpm release 0.2.0 --no-push   commit + tag locally only

import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const noPush = args.includes("--no-push");
const input = args.find((a) => !a.startsWith("--"));

const sh = (cmd) => execSync(cmd, { cwd: root, encoding: "utf8" }).trim();
const fail = (msg) => {
  console.error(`\n✖ ${msg}\n`);
  process.exit(1);
};

// Each file: path + a regex whose first group is the text before the version.
// Replacing in place (instead of re-serialising) keeps formatting and line endings.
const files = [
  { path: "package.json", re: /("version":\s*")\d+\.\d+\.\d+(")/ },
  { path: "src-tauri/tauri.conf.json", re: /("version":\s*")\d+\.\d+\.\d+(")/ },
  { path: "src-tauri/Cargo.toml", re: /(\[package\][^[]*?\nversion\s*=\s*")\d+\.\d+\.\d+(")/ },
  { path: "src-tauri/Cargo.lock", re: /(\[\[package\]\]\r?\nname = "quickfill"\r?\nversion = ")\d+\.\d+\.\d+(")/ },
];

const current = JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8")).version;

function nextVersion(cur, what) {
  const [maj, min, pat] = cur.split(".").map(Number);
  if (what === "major") return `${maj + 1}.0.0`;
  if (what === "minor") return `${maj}.${min + 1}.0`;
  if (what === "patch") return `${maj}.${min}.${pat + 1}`;
  return what;
}

function isNewer(a, b) {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) if (pa[i] !== pb[i]) return pa[i] > pb[i];
  return false;
}

if (!input) fail(`Give a version, e.g. "pnpm release 0.2.0" or "pnpm release patch". Current: ${current}`);
const version = nextVersion(current, input.replace(/^v/, ""));
if (!/^\d+\.\d+\.\d+$/.test(version)) fail(`"${input}" isn't a version like 1.2.3 or patch/minor/major.`);
if (!isNewer(version, current)) fail(`${version} must be newer than the current ${current}.`);
const tag = `v${version}`;

// Git checks
if (sh("git status --porcelain")) {
  if (dryRun) console.warn("⚠ Working tree has uncommitted changes (a real release would stop here).");
  else fail("Working tree has uncommitted changes. Commit or stash them first.");
}
const branch = sh("git rev-parse --abbrev-ref HEAD");
if (sh(`git tag --list ${tag}`)) fail(`Tag ${tag} already exists.`);
if (branch !== "main") console.warn(`⚠ You're on "${branch}", not main. The release will be built from this branch.`);

// Update files
console.log(`\nQuickFill ${current} → ${version}\n`);
for (const f of files) {
  const full = join(root, f.path);
  const text = readFileSync(full, "utf8");
  if (!f.re.test(text)) fail(`Couldn't find the version in ${f.path}.`);
  const updated = text.replace(f.re, `$1${version}$2`);
  console.log(`  ${dryRun ? "would update" : "updated"} ${f.path}`);
  if (!dryRun) writeFileSync(full, updated);
}

if (dryRun) {
  console.log(`\n  would commit "Release ${tag}", tag ${tag}${noPush ? "" : `, push ${branch} + ${tag}`}`);
  console.log("\nDry run: nothing changed.\n");
  process.exit(0);
}

sh(`git add ${files.map((f) => `"${f.path}"`).join(" ")}`);
sh(`git commit -m "Release ${tag}"`);
sh(`git tag -a ${tag} -m "QuickFill ${tag}"`);
console.log(`\n  committed and tagged ${tag}`);

if (noPush) {
  console.log(`\nNot pushed. When ready: git push origin ${branch} ${tag}\n`);
} else {
  sh(`git push origin ${branch}`);
  sh(`git push origin ${tag}`);
  const remote = sh("git remote get-url origin").replace(/\.git$/, "");
  console.log(`\n  pushed. GitHub is now building the installer:\n  ${remote}/actions\n`);
}
