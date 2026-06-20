use compositor_y5_canvas_input_state::state::CanvasGrab;

// Selection -> SelectSystem (SELECT); grouping -> GroupSystem (GROUP). Canvas is
// being decomposed into per-system slots; only the input Grab remains here.
pub struct CanvasState {
    pub Grab: CanvasGrab,
    /// Pan-in-progress flag: set when a canvas pan starts, cleared on release;
    /// canvas motion gates camera panning on it. Lives here (with the grab) not
    /// in the camera, so the canvas owner can flip it synchronously.
    pub position_updating: bool,
}

impl CanvasState {
    pub fn new() -> Self {
        Self {
            Grab: CanvasGrab::None,
            position_updating: false,
        }
    }
}
