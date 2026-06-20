use compositor_support_action_canvas_layout_base::layout::{self, MinSize, Rect};
use compositor_support_action_canvas_layout_ordered::ordered;
use smithay::desktop::Window;
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::utils::{Logical, Point, Rectangle, Size};
use compositor_orchestration_core_state_base::Loop;
use compositor_y5_window_lifecycle_interface::interface::TransformUpdate;

pub fn commit(_loop: &mut Loop, flags: layout::LayoutFlags) {
    commit_all(_loop, &[flags])
}

pub fn commit_all(_loop: &mut Loop, flags: &[layout::LayoutFlags]) {
    let windows: Vec<(Window, Rect)> = _loop.inner.select()
        
        .Selection
        .iter()
        .filter_map(|a| {
            let win = a.as_ref().clone();
            let geom = _loop.inner.space_state().state.element_geometry(a);
            if (geom.is_none()) {
                return None;
            }
            match geom {
                None => {
                    return None;
                }
                Some(geom) => {
                    return Some((
                        win,
                        Rect {
                            w: geom.size.w as f64,
                            x: geom.loc.x as f64,
                            y: geom.loc.y as f64,
                            h: geom.size.h as f64,
                        },
                    ));
                }
            }
        })
        .collect();

    let primary = _loop.inner.select()
        
        .Primary
        .as_ref()
        .map(|a| a.as_ref().clone());

    let primary: Option<(Window, Rect)> = primary.and_then(|primary| {
        let geom = _loop.inner.space_state().state.element_geometry(&primary);

        if geom.is_none() {
            return None;
        }
        let geom = geom.unwrap();
        return Some((
            primary.clone(),
            Rect {
                w: geom.size.w as f64,
                x: geom.loc.x as f64,
                y: geom.loc.y as f64,
                h: geom.size.h as f64,
            },
        ));
    });

    // It must operate on wworld size, or scale min size.
    let min_size = MinSize { w: 50.0, h: 50.0 };

    let result = ordered::layout_ordered(windows, primary, flags, min_size);
    // Expected result to align 2 the top of the 2 windows without any resize
    result.iter().for_each(|(window, rect)| {
        let window = window.clone();
        commit_result(_loop, window, rect.clone())
    });
}

fn commit_result(_loop: &mut Loop, window: Window, rect: Rect) {
    // CHECK: In some cases- where position is not updated, this cannot fail. The round() call should not change the results
    // this applies to size as well.

    let update = TransformUpdate {
        size: Some(Size::new(rect.w.round() as i32, rect.h.round() as i32)),
        position: Some(Point::new(rect.x.round() as i32, rect.y.round() as i32)),
    };
    compositor_y5_window_lifecycle_interface::interface::reform(_loop, window, update);
}
