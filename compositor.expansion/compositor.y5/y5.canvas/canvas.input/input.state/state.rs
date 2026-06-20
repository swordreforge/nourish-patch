use smithay::desktop::Window;
use smithay::utils::{Logical, Point, Rectangle};
use uuid::Uuid;

pub enum CanvasGrab {
    None,

    Target(TargetOption),
    Active(ActiveOption),
}

pub enum ActiveOption {
    Moving {
        candidates: ActiveTransformCandidate,
        start_cursor: Point<f64, Logical>,
        Anchor: Anchor,
    },
    Scaling {
        candidates: ActiveTransformCandidate,
        start_cursor: Point<f64, Logical>,
        Anchor: Anchor,
    },
    SelectBox {
        start_cursor: Point<f64, Logical>,
        current_cursor: Point<f64, Logical>,
        start_selection: Vec<Uuid>,
    },

    Hand,
}

pub enum ActiveTransformCandidate {
    Window(Vec<(Window, Rectangle<i32, Logical>)>),
    Placeholder(Uuid, Rectangle<i32, Logical>),
}
pub enum TargetOption {
    Scale,
    Move,
    Select { Append: bool },
}

pub struct Anchor {
    pub Horizontal: bool,
    pub Vertical: bool,
}
