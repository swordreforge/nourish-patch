pub use compositor_support_library_input_keyboard_action::shortcut;

pub mod keyboard {
    pub mod key {
        pub use compositor_support_library_input_keyboard_enum::*;
    }
    pub mod combo {
        pub use compositor_support_library_input_keyboard_combo::*;
    }
    pub mod handler {
        pub use compositor_support_library_input_keyboard_action::ShortcutHandler;
    }
}
