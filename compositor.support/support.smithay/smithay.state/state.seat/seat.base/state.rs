use smithay::backend::session::libseat::LibSeatSession;
use smithay::input::pointer::{CursorImageStatus, PointerHandle};
use smithay::input::{SeatHandler, SeatState};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point};
use smithay::wayland::pointer_constraints::{PointerConstraintsState, with_pointer_constraint};
use smithay::wayland::relative_pointer::RelativePointerManagerState;
use smithay::wayland::seat::WaylandFocus;

pub struct Seat<Handler: SeatHandler> {
    pub state: SeatState<Handler>,
    pub seat: smithay::input::Seat<Handler>,
    pub pointer_status: CursorImageStatus,
    pub relative_pointer_manager_state: RelativePointerManagerState,
    pub pointer_constraints_state: PointerConstraintsState,
    pub unlock_restoration_location: Option<(WlSurface, Point<f64, Logical>)>,
    pub previous_focus: Option<WlSurface>,
    pub libseat: Option<LibSeatSession>,
}

impl<I> Seat<I>
where
    I: SeatHandler + 'static,
    I::KeyboardFocus: WaylandFocus + 'static,
    I: SeatHandler<KeyboardFocus = WlSurface, PointerFocus = WlSurface, TouchFocus = WlSurface>,
{
    pub fn is_keyboard_focused(&self, surface: &WlSurface) -> bool {
        let Some(kb) = self.seat.get_keyboard() else { return false; };
        let Some(kb) = kb.current_focus() else { return false; };
        let Some(kb_surface) = kb.wl_surface() else { return false; };
        kb_surface.as_ref() == surface
    }

    pub fn deactivate_constraint_for(
        &mut self, surface: &WlSurface, pointer: &PointerHandle<I>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        with_pointer_constraint(surface, pointer, |c| {
            if let Some(c) = c { if c.is_active() { c.deactivate(); } }
        });
        if let Some((hint_surface, hint_location)) = self.unlock_restoration_location.take() {
            if &hint_surface == surface {
                return Some((hint_surface, hint_location));
            } else {
                self.unlock_restoration_location = Some((hint_surface, hint_location));
            }
        }
        None
    }

    pub fn reevaluate_pointer_constraints(
        &mut self, pointer: &PointerHandle<I>,
        previous: Option<&WlSurface>, updated: Option<&WlSurface>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        let token = if let Some(old) = previous {
            self.deactivate_constraint_for(old, pointer)
        } else { None };
        if let Some(new_surface) = updated {
            if self.is_keyboard_focused(new_surface) {
                with_pointer_constraint(new_surface, pointer, |c| {
                    if let Some(c) = c { if !c.is_active() { c.activate(); } }
                });
            }
        }
        token
    }

    pub fn abandon_active_constraint(&mut self, pointer: &PointerHandle<I>) {
        let prev_focus = pointer.current_focus();
        if let Some(prev) = prev_focus {
            if let Some(surface) = prev.wl_surface() {
                with_pointer_constraint(&surface, pointer, |c| {
                    if let Some(c) = c { if c.is_active() { c.deactivate(); } }
                });
            }
        }
        self.unlock_restoration_location = None;
    }

    pub fn is_pointer_over(&self, pointer: &PointerHandle<I>, surface: &WlSurface) -> bool {
        let Some(pointer) = pointer.current_focus() else { return false; };
        let Some(pointer) = pointer.wl_surface() else { return false; };
        pointer.as_ref() == surface
    }
}
