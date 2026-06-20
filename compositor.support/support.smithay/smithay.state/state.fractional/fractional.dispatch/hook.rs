use smithay::desktop::{Space, Window};
use smithay::wayland::seat::WaylandFocus;
use compositor_support_smithay_dispatch_state_base::fractional_base;
use compositor_support_smithay_dispatch_state_base::fractional_base::emit_to_surfaces;

pub fn hook(
    fractional: &mut fractional_base::Fractional,
    space: &Space<Window>,
    zoom: f64,
) -> Option<f64> {
    let Some(tick_updated_scale) = fractional.tick(zoom) else {
        return None;
    };

    let surfaces: Vec<_> = space.elements().filter_map(|w| w.wl_surface()).collect();

    let surfaces = surfaces.iter().map(|s| &**s);
    emit_to_surfaces(tick_updated_scale, surfaces);

    Some(tick_updated_scale)
}
