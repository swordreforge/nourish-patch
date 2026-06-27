//! The Keys tab: every compositor shortcut, each rebindable. Type a combo string
//! (e.g. "Super+K") in the field; Reset restores the built-in default. Edits
//! forward to the handler, which parses + persists to keybinding.json.
use compositor_developer_environment_keybinding_base::base::KeyRow;
use compositor_support_iced_core_engine_base::Renderer;
use compositor_configurator_settings_surface_message::message::SettingsMessage;
use compositor_configurator_settings_surface_style::style;
use iced_core::{Element, Length, Theme};
use iced_widget::{button, row, scrollable, text, text_input, Column};

pub fn build<'a>(keys: &'a [KeyRow]) -> Element<'a, SettingsMessage, Theme, Renderer> {
    let mut rows: Vec<Element<'a, SettingsMessage, Theme, Renderer>> = vec![
        text("Keyboard shortcuts").size(18).into(),
        text("Type a combo like \"Super+Shift+K\". Off disables; Reset restores the default.").size(12).into(),
    ];
    let mut fixed_header = false;
    for k in keys {
        if k.editable {
            let id = k.id.clone();
            // Empty override = disabled. Mark the label and let Off (set empty) /
            // Reset (restore default) toggle it; typing a combo also re-enables.
            let label = if k.combo.is_empty() {
                format!("{}  (off)", k.label)
            } else {
                k.label.clone()
            };
            rows.push(
                row![
                    text(label).width(Length::Fill),
                    text_input(&k.default, &k.combo)
                        .width(Length::Fixed(160.0))
                        .on_input(move |s| SettingsMessage::Rebind(id.clone(), s)),
                    button(text("Off")).on_press(SettingsMessage::Rebind(k.id.clone(), String::new())).style(style::action),
                    button(text("Reset")).on_press(SettingsMessage::ResetBind(k.id.clone())).style(style::action),
                ]
                .spacing(6)
                .into(),
            );
        } else {
            if !fixed_header {
                rows.push(text("Built-in (not rebindable)").size(14).into());
                fixed_header = true;
            }
            // Read-only: label + the held combo, no field/reset.
            rows.push(
                row![text(k.label.clone()).width(Length::Fill), text(k.combo.clone())].spacing(8).into(),
            );
        }
    }
    scrollable(Column::with_children(rows).spacing(6).padding(4)).into()
}
