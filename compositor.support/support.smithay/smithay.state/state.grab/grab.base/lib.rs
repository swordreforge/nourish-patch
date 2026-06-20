pub mod movement {
    pub mod state {
        pub use compositor_support_smithay_state_grab_move_state::GrabMovement;
    }
    // The PointerGrab impl lives with GrabMovement in grab.move.state.
    pub mod wire {}
}

pub mod resize {
    pub mod state {
        pub use compositor_support_smithay_state_grab_resize_state::GrabResize;
        pub use compositor_support_smithay_state_grab_resize_surface::{
            ResizeEdge, ResizeSurfaceState,
        };
    }
    pub mod dispatch {
        pub use compositor_support_smithay_state_grab_resize_commit::handle_commit;
    }
    // The PointerGrab impl lives with GrabResize in grab.resize.state.
    pub mod wire {}
}
