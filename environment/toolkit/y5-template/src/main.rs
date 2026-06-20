//! y5-template — scaffold a new L2 directory from a y5.template.
//!
//! Because Zed task variables only expose the active *editor* state (never the
//! project-panel selection), the target directory is chosen via an interactive
//! picker rather than received from Zed. Invoked from a Zed task:
//!
//!     y5-template                       # scans the worktree, shows a picker
//!     y5-template --scan "$ZED_WORKTREE_ROOT"
//!     y5-template --dir <L1_DIR>        # fast path: skip picker if it validates
//!     y5-template --template <NAME>     # choose template (default: "default")
//!     y5-template --list                # list discovered targets and exit
//!
//! Flow:
//!   1. Discover all valid L1 target directories under the scan root.
//!   2. Pick one (fuzzy picker; auto-used if --dir validates or only one found).
//!   3. Pick the template (default: "default", or --template NAME).
//!   4. Prompt for variables (Name always first and required; auto vars filled).
//!   5. Create `{L1}.{Name}/` (abort if it exists) and materialize the template,
//!      substituting in file/dir NAMES and file CONTENTS.

mod model;
mod picker;

use std::collections::BTreeSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use model::*;

const DEFAULT_TEMPLATE: &str = "default";
const SCAN_MAX_DEPTH: usize = 8;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut dir: Option<String> = None;
    let mut scan: Option<String> = None;
    let mut template: Option<String> = None;
    let mut list_only = false;
    let mut open = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                i += 1;
                dir = args.get(i).cloned();
            }
            "--scan" => {
                i += 1;
                scan = args.get(i).cloned();
            }
            "--template" => {
                i += 1;
                template = args.get(i).cloned();
            }
            "--list" => list_only = true,
            "--open" => open = true,
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("y5-template: unknown argument: {other}");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }

    // Determine the scan root: explicit --scan, else $ZED_WORKTREE_ROOT, else cwd.
    let scan_root = scan
        .map(PathBuf::from)
        .or_else(|| std::env::var("ZED_WORKTREE_ROOT").ok().map(PathBuf::from))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    match run(&scan_root, dir.as_deref(), template.as_deref(), list_only, open) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("\n\x1b[31m✗ {msg}\x1b[0m");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "y5-template — scaffold a new directory from a y5.template\n\n\
         USAGE:\n  \
         y5-template [--scan <ROOT>] [--template <NAME>]\n  \
         y5-template --dir <L1_DIR> [--template <NAME>]\n  \
         y5-template [--scan <ROOT>] --list\n\n\
         Targets are valid L1 directories: \
         <workspace_root>/<root_tail>.<L0>/<L0>.<L1>\n\
         The scan root defaults to $ZED_WORKTREE_ROOT, then the cwd.\n"
    );
}

fn run(
    scan_root: &Path,
    dir: Option<&str>,
    template: Option<&str>,
    list_only: bool,
    open: bool,
) -> Result<(), String> {
    // 1. Discover candidate targets.
    let scan_root = scan_root
        .canonicalize()
        .map_err(|e| format!("cannot access scan root `{}`: {e}", scan_root.display()))?;
    let targets = discover_l1_targets(&scan_root, SCAN_MAX_DEPTH);

    if list_only {
        if targets.is_empty() {
            return Err(format!("no valid L1 targets found under {}", scan_root.display()));
        }
        println!("Valid L1 targets under {}:", scan_root.display());
        for t in &targets {
            println!("  {}   ({})", target_label(t), t.l1_dir.display());
        }
        return Ok(());
    }

    // 2. Resolve the selection.
    let resolved = if let Some(d) = dir {
        // Fast path: if --dir validates, use it directly (skip picker).
        let p = PathBuf::from(d)
            .canonicalize()
            .map_err(|e| format!("cannot access --dir `{d}`: {e}"))?;
        match resolve(&p) {
            Ok(r) => r,
            Err(e) => {
                let mut msg = e.to_string();
                msg.push_str(&format!("\n  (--dir was: {})", p.display()));
                return Err(msg);
            }
        }
    } else {
        // We may have zero targets but still want to allow `+ L0/L1` to
        // bootstrap a fresh workspace. Require *some* workspace root: first
        // search downward (like discovery), then upward from scan_root or
        // ZED_FILE, in that order.
        if targets.is_empty() {
            let any_root = find_workspace_root_down(&scan_root, SCAN_MAX_DEPTH)
                .or_else(|| find_workspace_root(&scan_root))
                .or_else(|| std::env::var("ZED_FILE").ok().filter(|s| !s.is_empty())
                    .and_then(|f| find_workspace_root(&PathBuf::from(f))));
            if any_root.is_none() {
                return Err(format!(
                    "no valid L1 target directories and no qualifying workspace root \
                     found under {}.\n  A workspace root needs a `[workspace] members` \
                     entry with a two-level glob (e.g. `member.1/*/*`).",
                    scan_root.display()
                ));
            }
        }
        // If we were invoked while editing a file inside a valid L1 (via
        // ZED_FILE), pin that L1 as the FIRST picker entry — preselected,
        // visually marked. Falls through to the regular list otherwise.
        let current_l1 = std::env::var("ZED_FILE")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .and_then(|p| find_l1_ancestor(&p));

        // Build the (possibly-reordered) target list. The pinned entry, if
        // present, goes first; we remove its duplicate from the rest so the
        // picker doesn't show the same L1 twice.
        let mut ordered: Vec<&Resolved> = Vec::with_capacity(targets.len() + 1);
        if let Some(ref pinned) = current_l1 {
            ordered.push(pinned);
            for t in &targets {
                if t.l1_dir != pinned.l1_dir {
                    ordered.push(t);
                }
            }
        } else {
            ordered.extend(targets.iter());
        }

        let items: Vec<picker::Item> = ordered
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let pinned = current_l1.is_some() && i == 0;
                let label = if pinned {
                    format!("★ {} (current)", target_label(t))
                } else {
                    target_label(t)
                };
                picker::Item::new(label, t.l1_dir.display().to_string())
            })
            .collect();

        // Pick a workspace root for the `+ L0/L1` command. Order: pinned
        // current L1's, then first target's, then nearest qualifying root
        // (downward, then upward).
        let cmd_workspace_root: PathBuf = current_l1
            .as_ref()
            .map(|r| r.workspace_root.clone())
            .or_else(|| ordered.first().map(|t| t.workspace_root.clone()))
            .or_else(|| find_workspace_root_down(&scan_root, SCAN_MAX_DEPTH))
            .or_else(|| find_workspace_root(&scan_root))
            .or_else(|| std::env::var("ZED_FILE").ok().filter(|s| !s.is_empty())
                .and_then(|f| find_workspace_root(&PathBuf::from(f))))
            .ok_or_else(|| "no workspace root found for `+` command".to_string())?;
        let cmd_workspace_name: String = cmd_workspace_root
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let cmd_root_tail: String = cmd_workspace_name
            .rsplit('.')
            .next()
            .unwrap_or(&cmd_workspace_name)
            .to_string();

        // Closure: when the query contains a `/`, suspend filtering and
        // return ONE synthetic entry — either a selectable `+ create …`
        // action, or a non-selectable error explaining what's wrong.
        let make_extras = |query: &str| -> Vec<picker::Item> {
            if !query.contains('/') {
                return Vec::new();
            }
            match parse_path_spec(query) {
                Ok((l0, l1)) => {
                    let l0_dir_name = format!("{cmd_root_tail}.{l0}");
                    let l1_dir_name = format!("{l0}.{l1}");
                    let l0_dir = cmd_workspace_root.join(&l0_dir_name);
                    let l1_dir = l0_dir.join(&l1_dir_name);
                    let detail = format!("{}", l1_dir.display());
                    let label = format!("+ create  {l0_dir_name}/{l1_dir_name}");
                    vec![picker::Item::new(label, detail)]
                }
                Err(msg) => {
                    vec![picker::Item::info(format!("✗ {msg}"), String::new())]
                }
            }
        };

        let pick_result = picker::pick("Select target directory", &items, &make_extras)
            .map_err(|e| format!("picker error: {e}"))?;
        let pr = match pick_result {
            Some(pr) => pr,
            None => return Err("cancelled".into()),
        };
        match pr.kind {
            picker::PickKind::Base(idx) => ordered[idx].clone(),
            picker::PickKind::Extra(_) => {
                // Re-parse the query (the extras vector isn't kept around).
                let (l0, l1) = parse_path_spec(&pr.query).map_err(|e| e.to_string())?;
                let l0_dir = cmd_workspace_root.join(format!("{cmd_root_tail}.{l0}"));
                let l1_dir = l0_dir.join(format!("{l0}.{l1}"));
                std::fs::create_dir_all(&l1_dir).map_err(|e| {
                    format!("could not create {}: {e}", l1_dir.display())
                })?;
                println!("\x1b[36my5-template\x1b[0m  created path: {}", l1_dir.display());
                // Now resolve the freshly-created L1 just like any other.
                resolve(&l1_dir.canonicalize().unwrap_or(l1_dir))
                    .map_err(|e| format!("internal: created path failed to validate: {e}"))?
            }
        }
    };

    proceed(&resolved, template, open)
}

/// Parse a `+ L0/L1` path-spec from the picker query. Accepts exactly two
/// non-empty segments separated by a single `/`. Returns (L0, L1).
fn parse_path_spec(s: &str) -> Result<(String, String), String> {
    let trimmed = s.trim();
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() != 2 {
        return Err(format!(
            "path-spec must be exactly `L0/L1` ({} segment(s) given)",
            parts.len()
        ));
    }
    let l0 = parts[0].trim();
    let l1 = parts[1].trim();
    if l0.is_empty() || l1 .is_empty() {
        return Err("both L0 and L1 segments must be non-empty".into());
    }
    // Disallow path-traversal and shell-ish chars to be safe.
    for seg in [l0, l1] {
        if seg.contains('/') || seg.contains('\\') || seg.starts_with('.') {
            return Err(format!("invalid segment `{seg}`"));
        }
    }
    Ok((l0.to_string(), l1.to_string()))
}

fn proceed(resolved: &Resolved, template: Option<&str>, open: bool) -> Result<(), String> {
    println!("\x1b[36my5-template\x1b[0m");
    println!("  workspace : {}", resolved.workspace_name);
    println!("  L0 / L1   : {} / {}", resolved.l0, resolved.l1);
    println!("  selection : {}", resolved.l1_dir.display());

    // Locate templates.
    let hosts = template_hosts(&resolved.workspace_root);
    if hosts.is_empty() {
        return Err(format!(
            "no `y5.template` folder found at the workspace root ({}) or one directory above it",
            resolved.workspace_root.display()
        ));
    }
    let templates = list_templates(&hosts);
    if templates.is_empty() {
        return Err("no templates found inside any `y5.template` folder".into());
    }

    // Choose template: explicit --template, else "default", else (if exactly
    // one) that one, else error asking for --template.
    let wanted = template.unwrap_or(DEFAULT_TEMPLATE);
    let chosen = templates
        .iter()
        .find(|(n, _)| n == wanted)
        .or_else(|| {
            if template.is_none() && templates.len() == 1 {
                templates.first()
            } else {
                None
            }
        });
    let (tpl_name, tpl_dir) = match chosen {
        Some(t) => t,
        None => {
            let names: Vec<&str> = templates.iter().map(|(n, _)| n.as_str()).collect();
            return Err(format!(
                "template `{wanted}` not found. Available: {}",
                names.join(", ")
            ));
        }
    };
    println!("  template  : {tpl_name}");

    // 3. Determine variables. Auto vars are computed PER NAME; everything else
    //    the template declares is prompted once upfront. Name is required.
    let declared = template_variables(tpl_dir);

    // Prompt Name first (required). Comma-separated input becomes a batch.
    let raw = prompt_required("Name (comma-separated for batch)")?;
    let names: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if names.is_empty() {
        return Err("no names provided".into());
    }
    // Reject obvious duplicates in the same batch (would collide on disk).
    let mut dedup_check: BTreeSet<&str> = BTreeSet::new();
    for n in &names {
        if !dedup_check.insert(n.as_str()) {
            return Err(format!("duplicate name `{n}` in the batch"));
        }
    }

    // Prompt for any declared variable that isn't auto-provided and isn't Name.
    // Asked ONCE, shared across the whole batch.
    let auto_keys: BTreeSet<String> = auto_variables(resolved, "x")
        .into_iter()
        .map(|(k, _)| k)
        .collect();
    let mut extra: Vec<String> = declared
        .into_iter()
        .filter(|v| v != "Name" && !auto_keys.contains(v))
        .collect();
    extra.sort();
    let mut shared_extras: Vec<(String, String)> = Vec::with_capacity(extra.len());
    if !extra.is_empty() && names.len() > 1 {
        println!(
            "  ({} extra var{} prompted ONCE for the whole batch)",
            extra.len(),
            if extra.len() == 1 { "" } else { "s" }
        );
    }
    for var in extra {
        let val = prompt_optional(&var)?;
        shared_extras.push((var, val));
    }

    // 4. ATOMIC PRE-CHECK: compute every destination and verify none exist
    //    before writing anything. Mirrors the single-create guarantee.
    let mut plans: Vec<(String, PathBuf, Vec<(String, String)>)> = Vec::with_capacity(names.len());
    for n in &names {
        let dest_name = created_dir_name(resolved, n);
        let dest = resolved.l1_dir.join(&dest_name);
        if dest.exists() {
            return Err(format!(
                "destination already exists: {} (batch aborted, nothing was written)",
                dest.display()
            ));
        }
        let mut vars = auto_variables(resolved, n);
        vars.extend(shared_extras.iter().cloned());
        plans.push((n.clone(), dest, vars));
    }

    // 5. Materialize each. If any one fails, the rest are skipped (its temp
    //    staging dir is cleaned up by materialize itself), but earlier ones in
    //    the batch stay on disk — the pre-check already eliminated the most
    //    likely failure (existing dest), so reaching here and failing is rare.
    let multi = plans.len() > 1;
    let mut last_dest_and_module: Option<(PathBuf, String)> = None;
    for (i, (n, dest, vars)) in plans.iter().enumerate() {
        let created = materialize(tpl_dir, dest, vars)
            .map_err(|e| format!("failed while writing `{}`: {e}", dest.display()))?;
        if multi {
            println!(
                "\x1b[32m✓ [{}/{}] created {} ({} file(s))\x1b[0m",
                i + 1,
                plans.len(),
                dest.display(),
                created
            );
        } else {
            println!(
                "\n\x1b[32m✓ created {} ({} file(s))\x1b[0m",
                dest.display(),
                created
            );
        }
        let module = lookup(vars, "fully_qualified_module_name");
        println!("  fully_qualified_crate_name  = {}", lookup(vars, "fully_qualified_crate_name"));
        println!("  fully_qualified_module_name = {module}");
        last_dest_and_module = Some((dest.clone(), module));
        // Suppress unused-name warning for n; surface it on errors only.
        let _ = n;
    }

    // 6. With --open, open only the LAST created crate's file.
    if open {
        if let Some((dest, module)) = last_dest_and_module {
            match open_target(&dest, &module) {
                Some(file) => open_in_zed(&file),
                None => eprintln!(
                    "  (--open: no <module>.rs or lib.rs found under {} to open)",
                    dest.display()
                ),
            }
        }
    }
    Ok(())
}

/// Open a path in the running Zed window via the `zed` CLI. Since the path is
/// inside the current project, Zed reuses the existing window. If `zed` isn't
/// on PATH, fall back to printing the path (the create already succeeded).
fn open_in_zed(file: &Path) {
    use std::process::Command;
    match Command::new("zed").arg(file).status() {
        Ok(status) if status.success() => {
            println!("  opened {}", file.display());
        }
        Ok(_) | Err(_) => {
            // Try the alternate Linux binary name, then give up gracefully.
            match Command::new("zeditor").arg(file).status() {
                Ok(s) if s.success() => println!("  opened {}", file.display()),
                _ => println!(
                    "  (could not run `zed`; open it yourself: {})",
                    file.display()
                ),
            }
        }
    }
}

fn lookup(vars: &[(String, String)], key: &str) -> String {
    vars.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()).unwrap_or_default()
}

// ─── Prompting ──────────────────────────────────────────────────────────

fn prompt_required(var: &str) -> Result<String, String> {
    loop {
        let val = read_line(&format!("{var} (required): "))?;
        let val = val.trim().to_string();
        if !val.is_empty() {
            return Ok(val);
        }
        eprintln!("  {var} cannot be empty.");
    }
}

fn prompt_optional(var: &str) -> Result<String, String> {
    let val = read_line(&format!("{var}: "))?;
    Ok(val.trim().to_string())
}

fn read_line(prompt: &str) -> Result<String, String> {
    print!("{prompt}");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut s = String::new();
    let n = io::stdin().read_line(&mut s).map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("input closed (EOF) before all variables were provided".into());
    }
    Ok(s)
}

// ─── Materialization ────────────────────────────────────────────────────

/// Copy the template tree into `dest`, substituting placeholders in both
/// file/dir NAMES and file CONTENTS. Returns the number of files written.
/// Writes into a temp sibling dir first, then renames into place, so a
/// failure doesn't leave a half-written destination.
fn materialize(
    template_dir: &Path,
    dest: &Path,
    vars: &[(String, String)],
) -> io::Result<usize> {
    use walkdir::WalkDir;

    // staging dir: dest + ".y5tmp"
    let staging = dest.with_extension("y5tmp");
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;

    let mut count = 0usize;
    let mut result = (|| -> io::Result<()> {
        for entry in WalkDir::new(template_dir).into_iter().filter_map(|e| e.ok()) {
            let rel = match entry.path().strip_prefix(template_dir) {
                Ok(r) => r,
                Err(_) => continue,
            };
            if rel.as_os_str().is_empty() {
                continue; // the template root itself
            }
            // Substitute placeholders in each path component.
            let mut out_path = staging.to_path_buf();
            for comp in rel.components() {
                let name = comp.as_os_str().to_string_lossy();
                out_path.push(substitute(&name, vars));
            }

            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&out_path)?;
            } else if entry.file_type().is_file() {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                // Substitute in contents. Treat as UTF-8 text; if a file isn't
                // valid UTF-8, copy bytes verbatim (no substitution).
                match std::fs::read_to_string(entry.path()) {
                    Ok(text) => {
                        let rendered = substitute(&text, vars);
                        std::fs::write(&out_path, rendered)?;
                    }
                    Err(_) => {
                        std::fs::copy(entry.path(), &out_path)?;
                    }
                }
                count += 1;
            }
        }
        Ok(())
    })();

    // Commit or clean up.
    if result.is_ok() {
        result = std::fs::rename(&staging, dest);
    }
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&staging);
        result?;
    }
    Ok(count)
}
