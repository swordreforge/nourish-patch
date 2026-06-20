use iced_core::{
    Event as IcedEvent, Point,
    keyboard::{self, Key, Location, Modifiers as IcedMods, key::Named},
    mouse::{self, Button, ScrollDelta},
};
use smithay_client_toolkit::seat::{
    keyboard::{KeyEvent, Keysym, Modifiers as SctkMods},
    pointer::{AxisScroll, PointerEvent, PointerEventKind},
};

/// Translate one sctk PointerEvent into zero or more iced events.
/// Pointer position from sctk is already in surface-local logical coords
/// (the wl_pointer protocol gives surface coordinates).
pub fn translate_pointer(event: &PointerEvent) -> Vec<IcedEvent> {
    let position = Point::new(event.position.0 as f32, event.position.1 as f32);

    match event.kind {
        PointerEventKind::Enter { .. } => vec![
            IcedEvent::Mouse(mouse::Event::CursorEntered),
            IcedEvent::Mouse(mouse::Event::CursorMoved { position }),
        ],
        PointerEventKind::Leave { .. } => {
            vec![IcedEvent::Mouse(mouse::Event::CursorLeft)]
        }
        PointerEventKind::Motion { .. } => {
            vec![IcedEvent::Mouse(mouse::Event::CursorMoved { position })]
        }
        PointerEventKind::Press { button, .. } => translate_button(button)
            .map(|b| IcedEvent::Mouse(mouse::Event::ButtonPressed(b)))
            .into_iter()
            .collect(),
        PointerEventKind::Release { button, .. } => translate_button(button)
            .map(|b| IcedEvent::Mouse(mouse::Event::ButtonReleased(b)))
            .into_iter()
            .collect(),
        PointerEventKind::Axis {
            horizontal,
            vertical,
            ..
        } => translate_axis(horizontal, vertical)
            .map(|d| IcedEvent::Mouse(mouse::Event::WheelScrolled { delta: d }))
            .into_iter()
            .collect(),
    }
}

/// Linux input-event-codes button numbers → iced mouse buttons.
fn translate_button(linux_code: u32) -> Option<Button> {
    // From <linux/input-event-codes.h>
    match linux_code {
        0x110 => Some(Button::Left),    // BTN_LEFT
        0x111 => Some(Button::Right),   // BTN_RIGHT
        0x112 => Some(Button::Middle),  // BTN_MIDDLE
        0x113 => Some(Button::Back),    // BTN_SIDE
        0x114 => Some(Button::Forward), // BTN_EXTRA
        other => Some(Button::Other(other as u16)),
    }
}

/// Map sctk axis scroll (continuous + discrete) to an iced ScrollDelta.
/// Prefer discrete (mouse wheels) when present; fall back to continuous
/// (touchpads) as pixel scrolling.
fn translate_axis(h: AxisScroll, v: AxisScroll) -> Option<ScrollDelta> {
    if h.discrete != 0 || v.discrete != 0 {
        // Wayland discrete is reversed sign vs iced; check on a wheel.
        Some(ScrollDelta::Lines {
            x: -h.discrete as f32,
            y: -v.discrete as f32,
        })
    } else if h.absolute != 0.0 || v.absolute != 0.0 {
        Some(ScrollDelta::Pixels {
            x: -h.absolute as f32,
            y: -v.absolute as f32,
        })
    } else {
        None
    }
}

/// Translate a sctk keyboard event to an iced keyboard event.
pub fn translate_key(
    event: &KeyEvent,
    sctk_mods: SctkMods,
    pressed: bool,
    is_repeat: bool,
) -> Option<IcedEvent> {
    let key = keysym_to_iced_key(event.keysym, event.utf8.as_deref())?;
    let modifiers = translate_modifiers(sctk_mods);

    let text = if pressed {
        event.utf8.as_ref().and_then(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.as_str().into())
            }
        })
    } else {
        None
    };

    let logical_key = key.clone();
    let physical_key =
        keyboard::key::Physical::Unidentified(keyboard::key::NativeCode::Xkb(event.raw_code));

    Some(if pressed {
        IcedEvent::Keyboard(keyboard::Event::KeyPressed {
            key,
            modified_key: logical_key.clone(),
            physical_key,
            location: Location::Standard,
            modifiers,
            text,
            repeat: is_repeat,
        })
    } else {
        IcedEvent::Keyboard(keyboard::Event::KeyReleased {
            key,
            modified_key: logical_key,
            physical_key,
            location: Location::Standard,
            modifiers,
        })
    })
}

pub fn translate_modifiers(m: SctkMods) -> IcedMods {
    let mut out = IcedMods::empty();
    if m.shift {
        out |= IcedMods::SHIFT;
    }
    if m.ctrl {
        out |= IcedMods::CTRL;
    }
    if m.alt {
        out |= IcedMods::ALT;
    }
    if m.logo {
        out |= IcedMods::LOGO;
    }
    out
}

/// Map xkb keysym → iced Key. Covers the named keys most overlays care about;
/// printable chars fall through to Key::Character via utf8 text on press.
// fn keysym_to_iced_key(sym: Keysym) -> Option<Key> {
//     use Keysym as K;
//     let named = match sym {
//         K::Return | K::KP_Enter => Named::Enter,
//         K::Escape => Named::Escape,
//         K::BackSpace => Named::Backspace,
//         K::Tab => Named::Tab,
//         K::space => return Some(Key::Character(" ".into())),
//         K::Delete => Named::Delete,
//         K::Insert => Named::Insert,
//         K::Home => Named::Home,
//         K::End => Named::End,
//         K::Page_Up => Named::PageUp,
//         K::Page_Down => Named::PageDown,
//         K::Up => Named::ArrowUp,
//         K::Down => Named::ArrowDown,
//         K::Left => Named::ArrowLeft,
//         K::Right => Named::ArrowRight,
//         K::Shift_L | K::Shift_R => Named::Shift,
//         K::Control_L | K::Control_R => Named::Control,
//         K::Alt_L | K::Alt_R => Named::Alt,
//         K::Super_L | K::Super_R => Named::Super,
//         K::F1  => Named::F1, K::F2 => Named::F2, K::F3 => Named::F3, K::F4 => Named::F4,
//         K::F5  => Named::F5, K::F6 => Named::F6, K::F7 => Named::F7, K::F8 => Named::F8,
//         K::F9  => Named::F9, K::F10 => Named::F10, K::F11 => Named::F11, K::F12 => Named::F12,
//         _ => {
//             // Try printable: convert keysym → utf32 → char → string.
//             let ch = char::from_u32(xkb_keysym_to_utf32(sym))?;
//             if ch == '\0' || ch.is_control() {
//                 return None;
//             }
//             return Some(Key::Character(ch.to_string().into()));
//         }
//     };
//     Some(Key::Named(named))
// }

fn keysym_to_iced_key(sym: Keysym, utf8: Option<&str>) -> Option<Key> {
    use Keysym as K;
    let named = match sym {
        K::Return | K::KP_Enter => Named::Enter,
        K::Escape => Named::Escape,
        K::BackSpace => Named::Backspace,
        K::Tab => Named::Tab,
        K::Delete => Named::Delete,
        K::Insert => Named::Insert,
        K::Home => Named::Home,
        K::End => Named::End,
        K::Page_Up => Named::PageUp,
        K::Page_Down => Named::PageDown,
        K::Up => Named::ArrowUp,
        K::Down => Named::ArrowDown,
        K::Left => Named::ArrowLeft,
        K::Right => Named::ArrowRight,
        K::Shift_L | K::Shift_R => Named::Shift,
        K::Control_L | K::Control_R => Named::Control,
        K::Alt_L | K::Alt_R => Named::Alt,
        K::Super_L | K::Super_R => Named::Super,
        K::F1 => Named::F1,
        K::F2 => Named::F2,
        K::F3 => Named::F3,
        K::F4 => Named::F4,
        K::F5 => Named::F5,
        K::F6 => Named::F6,
        K::F7 => Named::F7,
        K::F8 => Named::F8,
        K::F9 => Named::F9,
        K::F10 => Named::F10,
        K::F11 => Named::F11,
        K::F12 => Named::F12,
        K::space => return Some(Key::Character(" ".into())),
        _ => {
            // For printable keys, use sctk's pre-computed utf8 text.
            let text = utf8?;
            if text.is_empty() {
                return None;
            }
            // Filter out control characters that may sneak in.
            if text.chars().all(|c| c.is_control()) {
                return None;
            }
            return Some(Key::Character(text.into()));
        }
    };
    Some(Key::Named(named))
}

/// xkbcommon's keysym-to-utf32 helper. The Keysym type from sctk wraps the
/// raw u32; this is the standard xkb conversion. Returns 0 for non-printable.
fn xkb_keysym_to_utf32(sym: Keysym) -> u32 {
    // sctk's Keysym derefs/exposes the raw value. Adjust if your version
    // stores it differently.
    let raw: u32 = sym.raw();
    // xkb encodes direct unicode keysyms as 0x01000000 | unicode codepoint.
    if raw & 0xff000000 == 0x01000000 {
        return raw & 0x00ffffff;
    }
    // For named keysyms (a, b, A, ...) xkb has a table. The xkbcommon-sys
    // crate exposes xkb_keysym_to_utf32 directly. Cleanest:
    //
    //     unsafe { xkbcommon_sys::xkb_keysym_to_utf32(raw) }
    //
    // If you don't want a deps on xkbcommon-sys, sctk re-exports xkbcommon
    // bindings through smithay_client_toolkit::seat::keyboard. Use:
    //
    //     event.utf8 (already provided by sctk on press) — preferred.
    //
    // For now, return 0 to indicate "no mapping" and rely on KeyEvent::utf8
    // as the source of character text.
    0
}
