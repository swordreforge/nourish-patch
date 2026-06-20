use smithay::desktop::Window;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;
use compositor_y5_window_interface_record::window::LoopWindow;
use compositor_support_system_storage_token_base::base::{Token, TokenMut};

/// The window-selection slot. The token lives beside its type (cycle-free) so the
/// core focus accessor can resolve it without depending on the select SYSTEM
/// crate (which transitively deps core). Owned/mutated by SelectSystem.
pub static SELECT: Token<CanvasSelect> = Token::new();
pub static SELECT_MUT: TokenMut<CanvasSelect> = TokenMut::new(&SELECT);

#[derive(Clone)]
pub struct CanvasSelect {
    pub Selection: Vec<Arc<Window>>,
    pub Primary: Option<Arc<Window>>,
}

pub struct SelectionTarget {
    time_start: Instant,
    pointer_location_start: (f64, f64),
}

impl CanvasSelect {
    pub fn new() -> Self {
        return Self {
            Selection: vec![],
            Primary: None,
        };
    }

    // Clears out the selection
    pub fn clear(&self) -> Self {
        let mut updated = self.clone();
        updated.Primary = None;
        updated.Selection.clear();

        return updated;
    }

    pub fn erase_uuid(&self, uuid: Uuid) -> (Self, bool) {
        let mut updated = self.clone();
        let mut changed = false;

        if let Some(primary) = &self.Primary {
            // CHECK: Map direct UUID
            if primary.uuid().unwrap_or_else(|| abort!("uuid")) == uuid {
                changed = true;
                updated.Primary = None;
            }
        }

        let pos = self
            .Selection
            .iter()
            .position(|a| a.uuid().unwrap_or_else(|| abort!("uuid")) == uuid);
        if let Some(pos) = pos {
            // It is available in the selection vector, remove it and set changed
            changed = true;
            updated.Selection.remove(pos);
        }

        (updated, changed)
    }

    // Default behaviour for set selection
    pub fn set(&self, window: Window) -> Self {
        let mut updated = self.clone();

        let window = Arc::new(window);

        // If the window exist, remove it
        let index = updated
            .Selection
            .iter()
            .position(|s| s.uuid() == window.uuid());

        // Clear selection the list.
        updated.Selection.clear();
        // No primary for single selection.
        updated.Primary = None;

        // The window was there, so dont re-add it
        if let Some(index) = index {
            return updated;
        } else {
            updated.Selection.push(window);
        }

        updated
    }

    pub fn get(&self, window: Window) -> bool {
        let window = Arc::new(window);
        self.Selection.contains(&window)
    }

    pub fn primary(&self, window: Window) -> bool {
        let window = Arc::new(window);
        self.Primary.eq(&Some(window))
    }

    // Default behaviour for appending selection
    pub fn append(&self, window: Window) -> Self {
        let mut updated = self.clone();
        // CHECK: if the window does not have UUID, this should panic. similar for set.

        let window = Arc::new(window);

        let index = updated
            .Selection
            .iter()
            .position(|s| s.uuid() == window.uuid());
        let primary = updated.Primary.eq(&Some(window.clone()));
        if let Some(index) = index {
            if primary {
                // It is primary and selected, deselect it
                updated.Selection.remove(index);

                // There are 2 choices
                // 1. Use the last selected element as primary ( less flexible if we want no primary )
                // 2. Keep primary as none. ( more flexible, if another primary needed, reclick it  )
                updated.Primary = None;
            } else {
                // It is not primary, just set as primary and push it toward the end
                updated.Primary = Some(window.clone());
                updated.Selection.remove(index);
                updated.Selection.push(window.clone());
            }
        } else {
            // The window is not selected:
            // 1. Add it to selection
            // 2. Set as primary
            updated.Selection.push(window.clone());

            // Doesnt affect primary.
            // self.Primary = Some(window.clone());
        }

        // Sanity check for list of size 1, remove primary
        if updated.Selection.len() == 1 {
            updated.Primary = None;
        }

        updated
    }

    pub fn exact(&self, window: Window, flag: bool) -> Self {
        let mut updated = self.clone();

        let window = Arc::new(window);
        let index = updated
            .Selection
            .iter()
            .position(|s| s.uuid() == window.uuid());
        let primary = updated.Primary.eq(&Some(window.clone()));

        if !flag && let Some(index) = index {
            updated.Selection.remove(index);

            if primary {
                updated.Primary = None;
            }
        } else if flag && let None = index {
            updated.Selection.push(window.clone());
        }

        // Sanity check for list of size 1, remove primary
        if updated.Selection.len() == 1 {
            updated.Primary = None;
        }

        updated
    }
}
