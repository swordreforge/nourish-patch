//! The picker's entry fade: a black overlay that clears over `FADE_SECS` (a
//! fade-in, in place of the lock screen's morph).

use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::{Id, Kind};
use smithay::backend::renderer::utils::CommitCounter;
use smithay::utils::{Physical, Point, Rectangle, Size};
use compositor_orchestration_core_state_base::Loop;
use compositor_y5_picker_system_base::base::{PICKER_MUT, PICKER_WORLD};

/// A full-screen black overlay whose alpha ramps 1 → 0 over `FADE_SECS`, or None
/// once the fade has cleared / the picker isn't active.
pub fn overlay(state: &mut Loop, size: Size<i32, Physical>) -> Option<SolidColorRenderElement> {
    let secs = state
        .inner
        .worlds
        .get_mut(PICKER_WORLD)
        .storage_mut()
        .get_mut(&PICKER_MUT)
        .active
        .as_ref()
        .map(|a| a.time.elapsed().as_secs_f64())?;
    let p = (secs / compositor_y5_picker_three_constant::FADE_SECS as f64).clamp(0.0, 1.0) as f32;
    (p < 1.0).then(|| {
        SolidColorRenderElement::new(
            Id::new(),
            Rectangle::new(Point::from((0, 0)), size),
            CommitCounter::default(),
            [0.0, 0.0, 0.0, 1.0 - p],
            Kind::Unspecified,
        )
    })
}
