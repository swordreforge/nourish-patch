use smithay::desktop::Space;
use compositor_support_smithay_dispatch_state_base::state::DispatchWire;
use compositor_support_smithay_state_space_base::state::SpaceState;

pub fn new<I: DispatchWire>() -> SpaceState {
    return SpaceState {
        state: Space::default(),
    }
}
