use smithay::reexports::wayland_server::DisplayHandle;
use smithay::wayland::input_method::InputMethodManagerState;
use smithay::wayland::text_input::TextInputManagerState;

pub struct TextInput {
    pub input_method_manager_state: InputMethodManagerState,
    pub text_input_manager_state: TextInputManagerState,
}
