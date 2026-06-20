#!/usr/bin/env node
// Repo-wide workspace conformance linter (see document/ARCHITECTURE.md, phase 0).
//
// Validates every member crate of every compositor* Cargo workspace root:
//   1. LAYOUT  — crates live exactly at <root>/<L0>/<L1>/<crate>/Cargo.toml.
//   2. CHAIN   — each dir level continues the chain prefix: the child dir name
//                starts with "<parent's last dot-segment>." (root tail counts as
//                the first parent; containers like compositor.support chain into
//                their roots: support.smithay).
//   3. NAMING  — package name == the dedup-joined dotted chain with "." -> "_"
//                (e.g. compositor/compositor.action/action.camera/camera.find
//                 -> compositor_action_camera_find). No y5_* names.
//   4. SIZE    — 30..100 LOC of .rs per crate (only >100 fails; <30 is advice),
//                and a single module file besides lib.rs/main.rs/build.rs.
//   5. FLAT    — the crate is flat: lib.rs and its single module sit next to
//                Cargo.toml; no src/ dir, no .rs in subdirs (tests/ exempt).
//
// Allowlist: workspace.lint.allow.json — { "<repo-relative crate dir>": ["size"|"layout"|"naming"|...] }
// Exempted rules are reported as warnings, not failures. The list is meant to shrink.
//
// Exit code 0 = clean (warnings allowed), 1 = violations.

const fs = require('fs');
const path = require('path');

const REPO_ROOT = __dirname;
const ALLOW_PATH = path.join(REPO_ROOT, 'workspace.lint.allow.json');
let allow = {};
if (fs.existsSync(ALLOW_PATH)) {
  const raw = fs.readFileSync(ALLOW_PATH, 'utf-8').trim();
  if (raw) allow = JSON.parse(raw);
}

const failures = [];
const warnings = [];

function isAllowed(crateRel, rule) {
  const rules = allow[crateRel];
  return Array.isArray(rules) && (rules.includes(rule) || rules.includes('*'));
}

function report(crateRel, rule, msg, meta) {
  if (meta && meta.rules.includes(rule)) {
    if (meta.reason) warnings.push(`[structural:${rule}] ${crateRel}: ${meta.reason}`);
    else failures.push(`[${rule}] ${crateRel}: metadata.lint allow without a reason — ${msg}`);
    return;
  }
  if (isAllowed(crateRel, rule)) warnings.push(`[allow:${rule}] ${crateRel}: ${msg}`);
  else failures.push(`[${rule}] ${crateRel}: ${msg}`);
}

function lastSeg(name) {
  const parts = name.split('.');
  return parts[parts.length - 1];
}

// The chain continuation a child dir must start with: the parent dir's name
// minus its first dot-segment (or the whole name when single-segment).
// compositor -> "compositor.", action.camera -> "camera.",
// state.xdg.activation -> "xdg.activation."
function chainPrefix(parentDirName) {
  const parts = parentDirName.split('.');
  return (parts.length > 1 ? parts.slice(1).join('.') : parentDirName) + '.';
}

function isWorkspaceRoot(dir) {
  const toml = path.join(dir, 'Cargo.toml');
  if (!fs.existsSync(toml)) return false;
  return /^\[workspace\]/m.test(fs.readFileSync(toml, 'utf-8'));
}

// Discover workspace roots: top-level compositor* dirs that are roots, or
// containers whose immediate children are roots. chainBase is the dotted prefix
// accumulated from the container nesting (used for package-name derivation).
function discoverRoots() {
  const roots = [];
  for (const entry of fs.readdirSync(REPO_ROOT, { withFileTypes: true })) {
    if (!entry.isDirectory() || !entry.name.startsWith('compositor')) continue;
    const dir = path.join(REPO_ROOT, entry.name);
    if (isWorkspaceRoot(dir)) {
      roots.push({ dir, chain: entry.name });
    } else {
      for (const sub of fs.readdirSync(dir, { withFileTypes: true })) {
        if (!sub.isDirectory()) continue;
        const subDir = path.join(dir, sub.name);
        if (!isWorkspaceRoot(subDir)) continue;
        // Container chaining (see document/ARCHITECTURE.md "Package naming"):
        // - root chains from the container name (support.smithay under
        //   compositor.support) -> merged chain compositor.support.smithay
        // - root named compositor.<x> (compositor.y5, compositor.remote,
        //   compositor.monitor under expansion/extension containers) -> the chain
        //   starts fresh at the root; the container is organizational only
        let chain;
        if (sub.name.startsWith('compositor.')) {
          chain = sub.name;
        } else if (sub.name.split('.')[0] === lastSeg(entry.name)) {
          chain = entry.name + '.' + sub.name.split('.').slice(1).join('.');
        } else {
          warnings.push(`[chain] ${entry.name}/${sub.name}: workspace root does not chain from its container name`);
          chain = entry.name + '.' + sub.name;
        }
        roots.push({ dir: subDir, chain });
      }
    }
  }
  return roots;
}

// Resolve member globs of a workspace root to crate dirs (manual parse, same
// approach as workspace.link.js).
function memberDirs(rootDir) {
  const rootToml = fs.readFileSync(path.join(rootDir, 'Cargo.toml'), 'utf-8');
  const membersMatch = rootToml.match(/members\s*=\s*\[([\s\S]*?)\]/);
  if (!membersMatch) return [];
  const body = membersMatch[1].replace(/#[^\n]*/g, '');
  const raw = Array.from(body.matchAll(/["']([^"']+)["']/g)).map(m => m[1]);
  const dirs = new Set();
  for (const member of raw) {
    if (/[*?[\]]/.test(member)) {
      const pattern = path.posix.join(member, 'Cargo.toml');
      for (const rel of fs.globSync(pattern, { cwd: rootDir })) {
        dirs.add(path.dirname(path.resolve(rootDir, rel)));
      }
    } else {
      const dir = path.resolve(rootDir, member);
      if (fs.existsSync(path.join(dir, 'Cargo.toml'))) dirs.add(dir);
    }
  }
  return Array.from(dirs).sort();
}

function packageName(crateDir) {
  const toml = fs.readFileSync(path.join(crateDir, 'Cargo.toml'), 'utf-8');
  const block = toml.match(/\[package\]([\s\S]*?)(?:\n\[|$)/);
  if (!block) return null;
  const name = block[1].match(/name\s*=\s*["']([^"']+)["']/);
  return name ? name[1] : null;
}

// Structural in-crate exemption: [package.metadata.lint] allow = [..],
// reason = "..." — for PERMANENT constraints (e.g. orphan-rule-bound trait
// impls that must live with their type), never for pending work. A missing
// reason makes the exemption itself a failure.
function metadataAllow(crateDir) {
  const toml = fs.readFileSync(path.join(crateDir, 'Cargo.toml'), 'utf-8');
  const block = toml.match(/\[package\.metadata\.lint\]([\s\S]*?)(?:\n\[|$)/);
  if (!block) return null;
  const allow = block[1].match(/allow\s*=\s*\[([^\]]*)\]/);
  const reason = block[1].match(/reason\s*=\s*["']([^"']+)["']/);
  const rules = allow ? Array.from(allow[1].matchAll(/["']([^"']+)["']/g)).map(m => m[1]) : [];
  return { rules, reason: reason ? reason[1] : null };
}

// DEPS: every dependency in a member crate must be `{name}.workspace = true`
// (optionally with `optional = true`). No path deps (internal or vendored), no
// external version deps, no per-crate feature selection — paths + features live
// only at the workspace root (generated block / manual [workspace.dependencies] /
// link.features.json). Returns a list of human-readable violations.
function depViolations(crateDir) {
  const lines = fs.readFileSync(path.join(crateDir, 'Cargo.toml'), 'utf-8').split('\n');
  const viols = [];
  let inDeps = false;
  for (let i = 0; i < lines.length; i++) {
    const header = lines[i].match(/^\s*\[([^\]]+)\]/);
    if (header) { inDeps = header[1].endsWith('dependencies'); continue; }
    if (!inDeps) continue;
    const m = lines[i].match(/^\s*([A-Za-z0-9_.-]+?)(\s*=|\.workspace\s*=)/);
    if (!m) continue;
    // `name.workspace = true` dotted form — always compliant.
    if (/^\s*[A-Za-z0-9_-]+\.workspace\s*=\s*true\s*$/.test(lines[i])) continue;
    const name = m[1];
    // Gather the (possibly multi-line) value.
    let rest = lines[i].slice(lines[i].indexOf('=') + 1).trim();
    let raw = rest;
    if (rest.startsWith('{')) {
      let depth = (rest.match(/\{/g) || []).length - (rest.match(/\}/g) || []).length;
      let j = i;
      while (depth > 0 && j + 1 < lines.length) { j++; raw += '\n' + lines[j]; depth += (lines[j].match(/\{/g) || []).length - (lines[j].match(/\}/g) || []).length; }
      i = j;
    }
    if (!/workspace\s*=\s*true/.test(raw)) viols.push(`${name} (not workspace=true)`);
    if (/\bpath\s*=/.test(raw)) viols.push(`${name} (path dep)`);
    if (/\bversion\s*=/.test(raw) || /^"/.test(raw)) viols.push(`${name} (external version dep)`);
    if (/\bfeatures\s*=\s*\[/.test(raw)) viols.push(`${name} (feature selection)`);
    if (/\bdefault-features\s*=/.test(raw)) viols.push(`${name} (default-features selection)`);
  }
  return viols;
}

// Concatenated .rs source of a crate (for content checks like the world-id rule).
function rustSource(crateDir) {
  let src = '';
  const walk = dir => {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) {
        if (e.name === 'target' || e.name === 'node_modules') continue;
        walk(p);
      } else if (e.name.endsWith('.rs')) {
        src += fs.readFileSync(p, 'utf-8');
      }
    }
  };
  walk(crateDir);
  return src;
}

function rustLocAndModules(crateDir) {
  let loc = 0;
  const modules = [];
  const walk = dir => {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) {
        // tests/ (integration tests) are exempt from the size policy
        if (e.name === 'target' || e.name === 'node_modules' || e.name === 'tests') continue;
        walk(p);
      } else if (e.name.endsWith('.rs')) {
        loc += (fs.readFileSync(p, 'utf-8').match(/\n/g) || []).length;
        if (!['lib.rs', 'main.rs', 'build.rs'].includes(e.name)) modules.push(path.relative(crateDir, p));
      }
    }
  };
  walk(crateDir);
  return { loc, modules };
}

function lintCrate(root, crateDir) {
  const crateRel = path.relative(REPO_ROOT, crateDir);
  const meta = metadataAllow(crateDir);
  const rel = path.relative(root.dir, crateDir);
  const parts = rel.split(path.sep);

  // 1. LAYOUT: exactly L0/L1/crate
  if (parts.length !== 3) {
    report(crateRel, 'layout', `crate is at depth ${parts.length}, expected <root>/<L0>/<L1>/<crate>`, meta);
    return; // chain/naming are meaningless at the wrong depth
  }

  // 2. CHAIN: each dir continues its parent dir's name (minus first segment)
  let chain = root.chain;
  let parentName = path.basename(root.dir);
  for (const part of parts) {
    const expectPrefix = chainPrefix(parentName);
    if (!part.startsWith(expectPrefix)) {
      report(crateRel, 'chain', `'${part}' does not start with '${expectPrefix}' (chain so far: ${chain})`, meta);
      return;
    }
    chain = chain + '.' + part.slice(expectPrefix.length);
    parentName = part;
  }

  // 3. NAMING: package == chain with dots -> underscores; no y5_ prefix
  const expected = chain.replace(/\./g, '_');
  const actual = packageName(crateDir);
  if (!actual) {
    report(crateRel, 'naming', 'no [package] name found');
  } else if (actual !== expected) {
    report(crateRel, 'naming', `package '${actual}', expected '${expected}'`, meta);
  }

  // 4. SIZE: 30..100 LOC, single module besides lib/main/build
  const { loc, modules } = rustLocAndModules(crateDir);
  if (loc > 100) report(crateRel, 'size', `${loc} LOC of .rs (policy: 30~100)`, meta);
  else if (loc < 30) warnings.push(`[advice:size] ${crateRel}: ${loc} LOC (<30 — consider merging)`);
  if (modules.length > 1) report(crateRel, 'size', `${modules.length} module files besides lib/main/build: ${modules.join(', ')}`, meta);

  // 5. FLAT: lib.rs + the single module sit next to Cargo.toml — no src/ dir,
  //    no .rs files in subdirectories (tests/ exempt).
  if (fs.existsSync(path.join(crateDir, 'src'))) {
    report(crateRel, 'flat', 'has a src/ dir — crates are flat (lib.rs next to Cargo.toml)');
  }
  const nested = modules.filter(m => m.includes(path.sep));
  if (nested.length > 0) {
    report(crateRel, 'flat', `.rs files in subdirectories: ${nested.join(', ')}`, meta);
  }

  // 6. DEPS: deps are workspace=true only — no path/version/feature deps in crates.
  //    Standalone workspaces (installer, dev-tools) are intentionally outside the
  //    link graph (no generated block), so they keep self-contained path deps —
  //    same exclusion the generator applies. Everything else must be workspace=true.
  const STANDALONE = /(^|[\/\\])compositor\.installer|[\/\\]developer\.tool[\/\\]/;
  if (!STANDALONE.test(crateRel)) {
    const deps = depViolations(crateDir);
    if (deps.length > 0) {
      report(crateRel, 'deps', `non-workspace dependency(s): ${deps.join(', ')}`, meta);
    }
  }

  // 7. WORLD-ID: the rim must resolve the focused world via accessors (camera(),
  //    canvas(), select(), ... — see document/WORLD_DELEGATION.md), never name a
  //    world by a literal id. A qualified `MAIN_WORLD` reference outside the
  //    WorldManager + loader construction is the delegation breach. Allowlisted
  //    crates are the documented-deferred sites (per-world background/iced) +
  //    legitimate world construction; the list must shrink to 0.
  if (/lock_system_base::base::MAIN_WORLD/.test(rustSource(crateDir))) {
    report(crateRel, 'world-id', 'literal MAIN_WORLD — resolve the focused world via an Orchestrator focus accessor', meta);
  }
}

// --names: emit JSON [{dir, current, expected}] for every member crate whose
// package name differs from the chain-derived expected name.
function emitNames() {
  const out = [];
  for (const root of discoverRoots()) {
    for (const crateDir of memberDirs(root.dir)) {
      const parts = path.relative(root.dir, crateDir).split(path.sep);
      if (parts.length !== 3) continue;
      let chain = root.chain;
      let parentName = path.basename(root.dir);
      let ok = true;
      for (const part of parts) {
        const prefix = chainPrefix(parentName);
        if (!part.startsWith(prefix)) { ok = false; break; }
        chain = chain + '.' + part.slice(prefix.length);
        parentName = part;
      }
      if (!ok) continue;
      const expected = chain.replace(/\./g, '_');
      const current = packageName(crateDir);
      if (current && current !== expected) {
        out.push({ dir: path.relative(REPO_ROOT, crateDir), current, expected });
      }
    }
  }
  console.log(JSON.stringify(out, null, 1));
}

function main() {
  if (process.argv.includes('--names')) return emitNames();
  const roots = discoverRoots();
  let total = 0;
  for (const root of roots) {
    for (const crateDir of memberDirs(root.dir)) {
      total += 1;
      lintCrate(root, crateDir);
    }
  }

  for (const w of warnings) console.log('  warn  ' + w);
  for (const f of failures) console.error('  FAIL  ' + f);
  console.log(`workspace.lint: ${total} crates across ${roots.length} workspace roots — ${failures.length} failure(s), ${warnings.length} warning(s)`);
  if (failures.length > 0) process.exit(1);
}

main();
