//! Bluetooth tab: power on/off, device list, pair/connect. Scan runs while the
//! settings window is open (started/stopped by the interface lifecycle).
use compositor_configurator_bluetooth_backend_base::base::BtSnapshot;
use compositor_support_iced_core_engine_base::Renderer;
use compositor_configurator_settings_surface_message::message::SettingsMessage;
use compositor_configurator_settings_surface_style::style;
use compositor_configurator_settings_surface_control::control;
use iced_core::{Element, Length, Theme};
use compositor_support_library_i18n_base_core::t;
use iced_widget::{button, row, scrollable, text, Column};

pub fn build<'a>(b: &'a BtSnapshot) -> Element<'a, SettingsMessage, Theme, Renderer> {
    let header = row![
        text(t!("Bluetooth")).size(18).width(Length::Fill),
        button(text(if b.powered { t!("On") } else { t!("Off") })).on_press(SettingsMessage::BtPower(!b.powered)).style(control::action),
    ].spacing(8);
    let mut rows: Vec<Element<'a, SettingsMessage, Theme, Renderer>> = vec![header.into()];
    if !b.available {
        rows.push(text(t!("BlueZ unavailable.")).into());
    }
    if b.discovering {
        rows.push(text(t!("Scanning...")).size(12).into());
    }
    for d in &b.devices {
        let name = if d.name.is_empty() { d.address.clone() } else { d.name.clone() };
        let status = if d.connected { t!(" - connected") } else if d.paired { t!(" - paired") } else { "" };
        let (lbl, msg) = if d.paired {
            (t!("Connect"), SettingsMessage::BtConnect(d.path.clone()))
        } else {
            (t!("Pair"), SettingsMessage::BtPair(d.path.clone()))
        };
        rows.push(
            row![text(format!("{name}{status}")).width(Length::Fill), button(text(lbl)).on_press(msg).style(control::action)]
                .spacing(8)
                .into(),
        );
    }
    scrollable(Column::with_children(rows).spacing(6).padding(4)).into()
}
