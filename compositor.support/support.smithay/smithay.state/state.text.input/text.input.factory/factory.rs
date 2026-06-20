use smithay::{
    input::SeatHandler,
    reexports::{
        wayland_protocols::wp::text_input::zv3::server::{
            zwp_text_input_manager_v3::ZwpTextInputManagerV3, zwp_text_input_v3::ZwpTextInputV3,
        },
        wayland_protocols_misc::zwp_input_method_v2::server::{
            zwp_input_method_manager_v2::ZwpInputMethodManagerV2,
            zwp_input_method_v2::ZwpInputMethodV2,
        },
        wayland_server::{Client, Dispatch, DisplayHandle, GlobalDispatch},
    },
    wayland::{
        GlobalData,
        input_method::{
            InputMethodManagerGlobalData, InputMethodManagerState, InputMethodUserData,
        },
        text_input::{TextInputManagerState, TextInputUserData},
    },
};
use compositor_support_smithay_dispatch_state_base::state::DispatchWire;
use compositor_support_smithay_state_text_input_base::state::TextInput;

pub fn new<I: DispatchWire>(display_handle: &DisplayHandle) -> TextInput
where
    I: GlobalDispatch<ZwpTextInputManagerV3, GlobalData>,
    I: Dispatch<ZwpTextInputManagerV3, GlobalData>,
    I: Dispatch<ZwpTextInputV3, TextInputUserData>,
    I: 'static,
    I: GlobalDispatch<ZwpInputMethodManagerV2, InputMethodManagerGlobalData>,
    I: Dispatch<ZwpInputMethodManagerV2, GlobalData>,
    I: Dispatch<ZwpInputMethodV2, InputMethodUserData<I>>,
    I: SeatHandler,
    I: 'static,
{
    let ime_dh = display_handle.clone();
    let ime_bound = Arc::new(AtomicBool::new(false));
    let input_method_manager_state =
        InputMethodManagerState::new::<I, _>(&display_handle, move |client| {
            return filter(client, &ime_bound, &ime_dh);
        });
    let text_input_manager_state = TextInputManagerState::new::<I>(&display_handle);

    TextInput {
        input_method_manager_state,
        text_input_manager_state,
    }
}

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
// The main purpose of the filter function is to disallow malicious applications from becoming IMEs.
// CHECK: This may be incomplete. It should make IMEs connect explicitly to exclusive IME wayland socket.
fn filter(client: &Client, ime_bound: &Arc<AtomicBool>, handle: &DisplayHandle) -> bool {
    let ime_bound_filter = Arc::clone(ime_bound);

    // FIlter by UID.
    // if creds.uid != my_uid {
    //         tracing::warn!(uid = creds.uid, "denied input-method bind: foreign UID");
    //         return false;
    // }

    let exec_filter = filter_1(client, handle);
    if !exec_filter{
        return false
    }

    // compare_exchange: only the first caller flips false→true and wins
    match ime_bound_filter.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(_) => {
            info!("input-method bound set");
            true
        }
        Err(_) => {
            warn!("denied input-method bind: already bound");
            false
        }
    }
}

const ALLOWED_IME_EXES: &[&str] = &[
    "/usr/bin/ibus-daemon",
    "/usr/libexec/ibus-daemon", // some builds put it here
];
fn filter_1(client: &Client, display_handle: &DisplayHandle) -> bool {
    let Ok(creds) = client.get_credentials(display_handle) else {
        return false;
    };

    let exe = std::fs::read_link(format!("/proc/{}/exe", creds.pid)).ok();
    matches!(exe, Some(p) if ALLOWED_IME_EXES.iter().any(|a| p == PathBuf::from(a)))
}
