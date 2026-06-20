use compositor_y5_group_protocol_base::protocol::GroupBufferMessage;
use compositor_y5_graphic_capture_session::message::CaptureMessage;
use compositor_y5_launcher_protocol_message::message::LauncherMessage;
use compositor_y5_lock_interface_surface::message::LockMessage;
use compositor_y5_placeholder_protocol_base::message::PlaceholderMessage;
use compositor_y5_picker_surface_view::PickerSurfaceMessage;

#[derive(Debug)]
pub struct SurfaceMessage {
    // SurfaceID // <-- by HandlerID is available just in case its needed
    pub message: SurfaceMessageType
}

#[derive(Debug)]
pub enum SurfaceMessageType {
    Group(GroupBufferMessage),
    Placeholder(PlaceholderMessage),
    Launcher(LauncherMessage),
    LockScreen(LockMessage),
    Capture(CaptureMessage),
    Picker(PickerSurfaceMessage),
}