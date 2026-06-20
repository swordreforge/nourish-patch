use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::desktop::{Space, Window};
use smithay::utils::{Logical, Point, Rectangle};
use smithay::wayland::dmabuf::{DmabufGlobal, ImportNotifier};
use smithay::wayland::shell::xdg::ToplevelSurface;
use compositor_support_smithay_dispatch_state_base::state::Dispatch;
use compositor_support_smithay_state_space_base::state::SpaceState;

pub trait WireTrait {
    /// Driverdata: the window `Space` hosted by the active spatial world.
    /// Smithay handlers (neutral `Wire`) reach the space ONLY through here —
    /// they must not touch worlds directly; orchestration thin-routes to its
    /// space-host slice (document/ARCHITECTURE.md → "Window tracking").
    fn host_space(&self) -> &SpaceState;
    fn host_space_mut(&mut self) -> &mut SpaceState;
    fn initialize_surface_data(&mut self, window: Window);
    fn destroy_surface_data(&mut self, surface: ToplevelSurface);
    /// Warp the pointer to a world-space point. The handler reads its own
    /// hosted space internally (it owns it now), so no space is passed in.
    fn apply_pointer(&mut self, storage_point: Point<f64, Logical>);
    fn place_window(&mut self, window: Window, geometry: Rectangle<i32, Logical>);
    /// A client asked to (un)fullscreen `window`. The actual sizing/placement is
    /// deferred to the Loop-level lifecycle hook, since it needs concrete state
    /// (group bounds) unavailable behind the generic `WireTrait` boundary.
    fn fullscreen_request(&mut self, window: Window, fullscreen: bool);
    fn dmabuf_import(
        &mut self,
        dispatch: &mut Dispatch,
        _global: &DmabufGlobal,
        _dmabuf: Dmabuf,
        notifier: ImportNotifier,
    ) -> Option<(Dmabuf, ImportNotifier)>;
}
