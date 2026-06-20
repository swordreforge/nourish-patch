use compositor_y5_window_interface_record::window::LoopWindow;
use smithay::desktop::Window;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Resolve a "join the selection into one group" request. Pure: takes the
/// selection set, the primary window, and the current window->group map, and
/// returns the target group + the windows to add — or None if the selection
/// spans conflicting groups (or is empty). Lifted out of the rim so the
/// GroupSystem can call it from its buffer.
pub fn resolve_join(
    selection: &[Arc<Window>],
    primary: &Option<Arc<Window>>,
    group_window: &HashMap<Uuid, Arc<Uuid>>,
) -> Option<(Uuid, Vec<Uuid>)> {
    let candidates: Vec<Uuid> = selection.iter().filter_map(|w| w.uuid()).collect();
    if candidates.is_empty() {
        return None;
    }

    let mut group: Option<Arc<Uuid>> = None;
    let mut group_by_primary = false;

    if let Some(primary) = primary
        && let Some(primary) = primary.uuid()
        && let Some(primary_group) = group_window.get(&primary).map(|w| w.as_ref().clone())
    {
        group = Some(Arc::new(primary_group));
        group_by_primary = true;
    }

    let mut add: Vec<Uuid> = vec![];
    for window in &candidates {
        let Some(window_group) = group_window.get(window) else {
            add.push(*window);
            continue;
        };
        if group_by_primary {
            if window_group != &group.clone().expect("group set when group_by_primary") {
                add.push(*window);
            }
            continue;
        }
        if let Some(g) = &group
            && g != window_group
        {
            return None;
        }
        group = Some(window_group.clone());
    }

    let group = group?;
    Some((group.as_ref().clone(), add))
}
