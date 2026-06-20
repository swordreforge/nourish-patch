use smithay::desktop::Window;
use uuid::Uuid;
use compositor_support_smithay_state_xdg_activation_dispatch::wire::ActivationDetails;

pub enum WindowLifecycleEvent {
    InitialMap(Window),
    // Resize(Window),
    Destroyed(Uuid, Option<ActivationDetails>),
    /// (Un)fullscreen request for a window. `true` = enter fullscreen.
    Fullscreen(Window, bool),
}