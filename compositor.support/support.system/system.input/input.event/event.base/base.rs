/// What a receiver does with an input event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputFlow {
    /// The event was handled; traversal stops.
    Consume,
    /// Not interested; the bus continues to the next receiver.
    Pass,
}

/// Kernel-level input event. Platform-free on purpose: the orchestration layer
/// translates smithay/libinput/winit events into this shape before the bus
/// traversal, so systems never see backend types.
#[derive(Clone, Debug)]
pub enum InputEvent {
    Keyboard {
        /// Raw keycode (evdev).
        code: u32,
        pressed: bool,
        /// Active modifier bits (shift/ctrl/alt/logo packed by the driver).
        modifiers: u32,
    },
    PointerMotion {
        /// Position in the world's storage space (the post-constraint normalized
        /// world point — what `pointer.motion` is given as its location).
        x: f64,
        y: f64,
        /// Physical screen-space cursor position (the rim's `raw_pos` /
        /// `position_screen` accumulator). Carried separately from the world
        /// point because the canvas-pan delta (`screen - position_previous`)
        /// must run in physical space, while `pointer.motion`/transforms use the
        /// world point above.
        screen_x: f64,
        screen_y: f64,
        delta_x: f64,
        delta_y: f64,
    },
    PointerButton {
        /// Raw button code (evdev, e.g. BTN_LEFT).
        button: u32,
        pressed: bool,
        x: f64,
        y: f64,
    },
    PointerAxis {
        horizontal: f64,
        vertical: f64,
        x: f64,
        y: f64,
    },
}
