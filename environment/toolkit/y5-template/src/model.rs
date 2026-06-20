//! Pure logic for the y5 template tool: workspace/level validation, the
//! derived-variable computation, template discovery, placeholder extraction,
//! and substitution. Kept free of interactive I/O so it can be unit-tested.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The placeholder delimiters: `$${name}$$`.
pub const OPEN: &str = "$${";
pub const CLOSE: &str = "}$$";

/// Result of resolving + validating a selection directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// Absolute path of the workspace root (dir holding the qualifying Cargo.toml).
    pub workspace_root: PathBuf,
    /// The workspace root's own directory name, e.g. "compositor.ui".
    pub workspace_name: String,
    /// L0 own-segment, e.g. "member.1".
    pub l0: String,
    /// L1 own-segment, e.g. "example".
    pub l1: String,
    /// The L1 directory itself (the selection).
    pub l1_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    NoWorkspaceRoot,
    NotTwoLevelsDeep { levels: usize },
    BadL0Prefix { expected_prefix: String, found: String },
    BadL1Prefix { expected_prefix: String, found: String },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::NoWorkspaceRoot => write!(
                f,
                "no enclosing Cargo workspace with a two-level glob members entry \
                 (e.g. members = [\"member.1/*/*\"]) was found above the selection"
            ),
            ResolveError::NotTwoLevelsDeep { levels } => write!(
                f,
                "the selected directory must be exactly 2 levels below the workspace \
                 root (L0/L1); it is {levels} level(s) below"
            ),
            ResolveError::BadL0Prefix { expected_prefix, found } => write!(
                f,
                "L0 directory `{found}` must be prefixed with `{expected_prefix}` \
                 (chain-prefix convention)"
            ),
            ResolveError::BadL1Prefix { expected_prefix, found } => write!(
                f,
                "L1 directory `{found}` must be prefixed with `{expected_prefix}` \
                 (chain-prefix convention)"
            ),
        }
    }
}

/// The last dot-delimited segment of a name. "compositor.ui" -> "ui";
/// "member.1" -> "1"? No — segments are split on '.', and the chain uses the
/// WHOLE own-segment of a level which itself may contain dots (e.g. the L0
/// own-segment is "member.1"). So we cannot just split on '.'.
///
/// The convention is positional, not dot-counting: a child's name is
/// `{parent_tail}.{child_tail}` where `{parent_tail}` is the part of the
/// parent's name AFTER its own inherited prefix. We compute tails by walking
/// down from the root, peeling the known prefix at each step.
fn tail_after_prefix<'a>(name: &'a str, prefix: &str) -> Option<&'a str> {
    // child name must be exactly `{prefix}.{tail}`
    let with_dot = format!("{prefix}.");
    name.strip_prefix(&with_dot).filter(|t| !t.is_empty())
}

/// Find the nearest ancestor directory (inclusive of `start`'s parents) that
/// contains a Cargo.toml whose [workspace] members has a two-level glob entry
/// like `something/*/*`. Returns that directory.
pub fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start.to_path_buf());
    while let Some(d) = dir {
        let cargo = d.join("Cargo.toml");
        if cargo.is_file() {
            if let Ok(text) = fs::read_to_string(&cargo) {
                if has_two_level_glob_members(&text) {
                    return Some(d);
                }
            }
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    None
}

/// True if the Cargo.toml text has a [workspace] members array containing an
/// entry with at least two glob `*` path components (e.g. "member.1/*/*").
pub fn has_two_level_glob_members(text: &str) -> bool {
    for member in workspace_members(text) {
        let stars = member.split('/').filter(|c| *c == "*" || *c == "**").count();
        if stars >= 2 {
            return true;
        }
    }
    false
}

/// Extract the string entries of [workspace] members (single- or multi-line),
/// stripping `#` comments. Minimal scanner (same spirit as the LSP tool).
pub fn workspace_members(text: &str) -> Vec<String> {
    // Find [workspace] section body.
    let mut in_ws = false;
    let mut body = String::new();
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            if in_ws {
                break;
            }
            in_ws = t.trim_matches(['[', ']'].as_ref()).trim() == "workspace";
            continue;
        }
        if in_ws {
            body.push_str(line);
            body.push('\n');
        }
    }
    if body.is_empty() {
        return Vec::new();
    }
    // Find `members = [ ... ]`.
    let Some(idx) = body.find("members") else {
        return Vec::new();
    };
    let after = &body[idx..];
    let Some(open) = after.find('[') else {
        return Vec::new();
    };
    let Some(close) = after[open..].find(']') else {
        return Vec::new();
    };
    let arr = &after[open + 1..open + close];

    let mut out = Vec::new();
    for raw in arr.lines() {
        let line = match raw.find('#') {
            Some(h) => &raw[..h],
            None => raw,
        };
        let mut rest = line;
        while let Some(q) = rest.find('"') {
            let tail = &rest[q + 1..];
            if let Some(q2) = tail.find('"') {
                out.push(tail[..q2].to_string());
                rest = &tail[q2 + 1..];
            } else {
                break;
            }
        }
    }
    out
}

/// Resolve and validate a selection directory against the chain-prefix rules.
pub fn resolve(selection: &Path) -> Result<Resolved, ResolveError> {
    let root = find_workspace_root(selection).ok_or(ResolveError::NoWorkspaceRoot)?;

    // Compute the chain of directory names from root (exclusive) down to the
    // selection (inclusive).
    let rel = selection.strip_prefix(&root).map_err(|_| ResolveError::NoWorkspaceRoot)?;
    let components: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    if components.len() != 2 {
        return Err(ResolveError::NotTwoLevelsDeep { levels: components.len() });
    }

    let workspace_name = root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // Root's own tail = the last dot-segment of the workspace name
    // (e.g. "compositor.ui" -> "ui"). This is what L0 must chain off.
    let root_tail = workspace_name.rsplit('.').next().unwrap_or(&workspace_name);

    let l0_name = &components[0];
    let l0 = tail_after_prefix(l0_name, root_tail).ok_or_else(|| ResolveError::BadL0Prefix {
        expected_prefix: format!("{root_tail}."),
        found: l0_name.clone(),
    })?;

    let l1_name = &components[1];
    let l1 = tail_after_prefix(l1_name, l0).ok_or_else(|| ResolveError::BadL1Prefix {
        expected_prefix: format!("{l0}."),
        found: l1_name.clone(),
    })?;

    Ok(Resolved {
        workspace_root: root.clone(),
        workspace_name,
        l0: l0.to_string(),
        l1: l1.to_string(),
        l1_dir: selection.to_path_buf(),
    })
}

/// Discover every valid L1 target directory at or beneath `scan_root`.
///
/// Strategy: find all directories under `scan_root` that are qualifying
/// workspace roots (Cargo.toml with a two-level glob members entry). For each
/// such root, enumerate directories exactly 2 levels below it and keep those
/// that pass the chain-prefix validation. Returns the resolved selections,
/// sorted by their display path.
///
/// `max_depth` bounds how deep we look for workspace roots, to keep scans
/// fast on large trees.
pub fn discover_l1_targets(scan_root: &Path, max_depth: usize) -> Vec<Resolved> {
    use walkdir::WalkDir;

    // 1. Collect qualifying workspace roots.
    let mut roots: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(scan_root)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_dir() {
            continue;
        }
        let cargo = entry.path().join("Cargo.toml");
        if cargo.is_file() {
            if let Ok(text) = fs::read_to_string(&cargo) {
                if has_two_level_glob_members(&text) {
                    roots.push(entry.path().to_path_buf());
                }
            }
        }
    }

    // 2. For each root, enumerate dirs exactly 2 levels deep and validate.
    let mut out: Vec<Resolved> = Vec::new();
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    for root in &roots {
        for l0_entry in read_subdirs(root) {
            for l1_entry in read_subdirs(&l0_entry) {
                // resolve() re-validates the chain-prefix rules and rediscovers
                // the (nearest) workspace root, which will be `root` here.
                if let Ok(resolved) = resolve(&l1_entry) {
                    let key = l1_entry.canonicalize().unwrap_or_else(|_| l1_entry.clone());
                    if seen.insert(key) {
                        out.push(resolved);
                    }
                }
            }
        }
    }

    out.sort_by(|a, b| a.l1_dir.cmp(&b.l1_dir));
    out
}

/// Immediate subdirectories of `dir` (non-recursive), skipping hidden ones
/// and the template host folder.
fn read_subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name == "y5.template" || name == "target" {
                    continue;
                }
                out.push(e.path());
            }
        }
    }
    out.sort();
    out
}

/// Search DOWNWARD from `scan_root` for the first directory whose Cargo.toml
/// qualifies as a workspace root (two-level glob members entry). Bounded by
/// `max_depth`. Returns the first match in walk order.
pub fn find_workspace_root_down(scan_root: &Path, max_depth: usize) -> Option<PathBuf> {
    use walkdir::WalkDir;
    for entry in WalkDir::new(scan_root)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_dir() {
            continue;
        }
        let cargo = entry.path().join("Cargo.toml");
        if cargo.is_file() {
            if let Ok(text) = fs::read_to_string(&cargo) {
                if has_two_level_glob_members(&text) {
                    return Some(entry.path().to_path_buf());
                }
            }
        }
    }
    None
}

/// Walk up from `file` looking for the first ancestor directory that
/// resolves as a valid L1 target. Returns the resolved L1, or None if no
/// ancestor qualifies (i.e. the file isn't inside an actual L1 crate).
///
/// This powers the "current L1" pinned entry in the picker when invoked
/// while editing a file inside an L2 (or deeper).
pub fn find_l1_ancestor(file: &Path) -> Option<Resolved> {
    let mut dir = if file.is_dir() {
        Some(file.to_path_buf())
    } else {
        file.parent().map(|p| p.to_path_buf())
    };
    while let Some(d) = dir {
        if let Ok(r) = resolve(&d) {
            return Some(r);
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    None
}

/// A short, friendly label for a resolved L1 target, for the picker:
/// "<workspace_name> › <L0> › <L1>".
pub fn target_label(r: &Resolved) -> String {
    format!("{} › {} › {}", r.workspace_name, r.l0, r.l1)
}

/// After creating `dest`, pick the file to open (when --open is requested).
/// Tries, in order: src/<module>.rs, <module>.rs, src/lib.rs, lib.rs.
/// `module` is the fully-qualified module name (the Name, dots → `_`).
/// Returns the first path that exists, or None.
pub fn open_target(dest: &Path, module: &str) -> Option<PathBuf> {
    let candidates = [
        dest.join("src").join(format!("{module}.rs")),
        dest.join(format!("{module}.rs")),
        dest.join("src").join("lib.rs"),
        dest.join("lib.rs"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

/// Convert a dotted name to an identifier-safe form (dots -> underscores).
pub fn dots_to_underscores(s: &str) -> String {
    s.replace('.', "_")
}

/// Compute the automatic variables for a resolved selection plus a Name input.
/// `name` is the raw user input (may contain dots).
pub fn auto_variables(resolved: &Resolved, name: &str) -> Vec<(String, String)> {
    // fully_qualified_crate_name = workspace_name + each own-segment + Name,
    // joined as a dotted path, then dots -> underscores.
    let dotted = format!(
        "{}.{}.{}.{}",
        resolved.workspace_name, resolved.l0, resolved.l1, name
    );
    let fq_crate = dots_to_underscores(&dotted);
    let fq_module = dots_to_underscores(name);

    vec![
        ("workspace_name".into(), resolved.workspace_name.clone()),
        ("L0".into(), resolved.l0.clone()),
        ("L1".into(), resolved.l1.clone()),
        ("Name".into(), name.to_string()),
        ("fully_qualified_crate_name".into(), fq_crate),
        ("fully_qualified_module_name".into(), fq_module),
    ]
}

/// The directory name to create inside the selection: `{L1}.{Name}`.
pub fn created_dir_name(resolved: &Resolved, name: &str) -> String {
    format!("{}.{}", resolved.l1, name)
}

// ─── Template discovery ─────────────────────────────────────────────────

/// Locate `y5.template` host folders: at the workspace root and one dir above.
pub fn template_hosts(workspace_root: &Path) -> Vec<PathBuf> {
    let mut hosts = Vec::new();
    let here = workspace_root.join("y5.template");
    if here.is_dir() {
        hosts.push(here);
    }
    if let Some(parent) = workspace_root.parent() {
        let up = parent.join("y5.template");
        if up.is_dir() {
            hosts.push(up);
        }
    }
    hosts
}

/// List template names (immediate subdirectories) across all hosts. If the
/// same name appears in multiple hosts, the first host (workspace root) wins.
pub fn list_templates(hosts: &[PathBuf]) -> Vec<(String, PathBuf)> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();
    for host in hosts {
        if let Ok(entries) = fs::read_dir(host) {
            let mut names: Vec<_> = entries
                .flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| (e.file_name().to_string_lossy().to_string(), e.path()))
                .collect();
            names.sort_by(|a, b| a.0.cmp(&b.0));
            for (name, path) in names {
                if seen.insert(name.clone()) {
                    out.push((name, path));
                }
            }
        }
    }
    out
}

// ─── Placeholder extraction & substitution ──────────────────────────────

/// Find all `$${var}$$` placeholder names in a string.
pub fn extract_placeholders(s: &str, out: &mut BTreeSet<String>) {
    let mut rest = s;
    while let Some(o) = rest.find(OPEN) {
        let after = &rest[o + OPEN.len()..];
        if let Some(c) = after.find(CLOSE) {
            let name = after[..c].trim();
            if !name.is_empty() {
                out.insert(name.to_string());
            }
            rest = &after[c + CLOSE.len()..];
        } else {
            break;
        }
    }
}

/// Substitute every `$${var}$$` using the provided lookup. Unknown variables
/// are left intact (callers should ensure all are filled first).
pub fn substitute(s: &str, vars: &[(String, String)]) -> String {
    let mut result = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(o) = rest.find(OPEN) {
        result.push_str(&rest[..o]);
        let after = &rest[o + OPEN.len()..];
        if let Some(c) = after.find(CLOSE) {
            let name = after[..c].trim();
            if let Some((_, val)) = vars.iter().find(|(k, _)| k == name) {
                result.push_str(val);
            } else {
                // leave intact
                result.push_str(OPEN);
                result.push_str(&after[..c]);
                result.push_str(CLOSE);
            }
            rest = &after[c + CLOSE.len()..];
        } else {
            result.push_str(OPEN);
            rest = after;
        }
    }
    result.push_str(rest);
    result
}

/// Scan a template directory, returning the set of all placeholder names used
/// in any file's CONTENTS or in any file/dir NAME.
pub fn template_variables(template_dir: &Path) -> BTreeSet<String> {
    use walkdir::WalkDir;
    let mut vars = BTreeSet::new();
    for entry in WalkDir::new(template_dir).into_iter().flatten() {
        // names (relative component) may contain placeholders
        if let Some(name) = entry.path().file_name() {
            extract_placeholders(&name.to_string_lossy(), &mut vars);
        }
        if entry.file_type().is_file() {
            if let Ok(text) = fs::read_to_string(entry.path()) {
                extract_placeholders(&text, &mut vars);
            }
        }
    }
    vars
}
