use compositor_introspection_extraction_window_meta_proc_read::proc::extract_meta_for_pid;
use compositor_introspection_extraction_window_meta_types::types::{Meta, MetaNode};
use std::collections::HashSet;
use std::fs;

/// Reasonable defaults for tree-walking; callers can override.
pub const DEFAULT_CHILD_DEPTH: usize = 16;
pub const DEFAULT_PARENT_STEPS: usize = 8;

/// Re-extract a MetaNode given only a PID, preserving Wayland-side fields
/// from a previously-captured node. Used by background samplers that have a
/// PID but no live `Window`/`WlSurface`; app_id/title rarely change for a
/// pinned window, so they're copied forward from the prior extraction.
pub fn refresh_meta_from_pid(pid: u32, previous: &MetaNode) -> Option<MetaNode> {
    let mut fresh = extract_full_tree(pid)?;
    // Preserve Wayland-derived fields. They aren't in `/proc`.
    fresh.meta.app_id = previous.meta.app_id.clone();
    fresh.meta.title = previous.meta.title.clone();
    fresh.meta.uid = previous.meta.uid;
    fresh.meta.gid = previous.meta.gid;
    Some(fresh)
}

/// Read `/proc/<pid>/task/<pid>/children` into a list of child PIDs.
fn read_children(pid: u32) -> Vec<u32> {
    let path = format!("/proc/{pid}/task/{pid}/children");
    fs::read_to_string(&path)
        .ok()
        .map(|s| s.split_whitespace().filter_map(|n| n.parse::<u32>().ok()).collect())
        .unwrap_or_default()
}

/// Read the parent PID from `/proc/<pid>/stat`. `comm` is parenthesized and
/// can contain spaces or ')', so split from the LAST ')' to be safe.
fn read_ppid(pid: u32) -> Option<u32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let last_paren = stat.rfind(')')?;
    let after = &stat[last_paren + 1..];
    let fields: Vec<&str> = after.split_whitespace().collect();
    // fields[0] = state, fields[1] = ppid
    fields.get(1).and_then(|s| s.parse().ok())
}

/// Build a MetaNode rooted at `root_pid`, walking children down to `max_depth`.
/// The result's `parent` is None; use [`walk_parents`] to fill it.
pub fn extract_tree(root_pid: u32, max_depth: usize) -> Option<MetaNode> {
    extract_tree_inner(root_pid, max_depth, &mut HashSet::new())
}

fn extract_tree_inner(pid: u32, depth_remaining: usize, visited: &mut HashSet<u32>) -> Option<MetaNode> {
    if depth_remaining == 0 || !visited.insert(pid) {
        return None;
    }
    let meta = extract_meta_for_pid(pid)?;
    let children = read_children(pid)
        .into_iter()
        .filter_map(|child_pid| extract_tree_inner(child_pid, depth_remaining - 1, visited))
        .collect();
    Some(MetaNode { meta, parent: None, children })
}

/// Walk parents upward from the given Meta's pid. Stops at pid 0/1, at
/// max_steps, or when a /proc read fails. Returns the immediate parent node
/// (with its own parent chained via Box).
pub fn walk_parents(meta: &Meta, max_steps: usize) -> Option<MetaNode> {
    let root_pid = meta.pid?;
    walk_parents_inner(root_pid, max_steps)
}

fn walk_parents_inner(pid: u32, max_steps: usize) -> Option<MetaNode> {
    if max_steps == 0 {
        return None;
    }
    let parent_pid = read_ppid(pid)?;
    if matches!(parent_pid, 0 | 1) {
        return None;
    }
    let parent_meta = extract_meta_for_pid(parent_pid)?;
    let grandparent = walk_parents_inner(parent_pid, max_steps - 1);
    Some(MetaNode {
        meta: parent_meta,
        parent: grandparent.map(Box::new),
        children: Vec::new(),
    })
}

/// Given just a PID, build a MetaNode with children and parents populated.
pub fn extract_full_tree(root_pid: u32) -> Option<MetaNode> {
    let mut node = extract_tree(root_pid, DEFAULT_CHILD_DEPTH)?;
    node.parent = walk_parents(&node.meta, DEFAULT_PARENT_STEPS).map(Box::new);
    Some(node)
}
