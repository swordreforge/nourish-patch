//! Renders the whole settings panel from plain data (so the `IcedUi` owner stays
//! tiny): right-edge full-height panel over a near-solid full-screen backdrop,
//! with the tab content in a height-filling scroll area and the mode Keep/Revert
//! confirm bar PINNED to the panel bottom (always visible, even mid-scroll).
use compositor_developer_environment_config_base::base::Environment;
use compositor_developer_environment_keybinding_base::base::KeyRow;
use compositor_configurator_hardware_gpu_base::base::RenderDevice;
use compositor_orchestration_driver_output_base::base::ModeInfo;
use compositor_support_iced_core_engine_base::Renderer;
use compositor_y5_audio_controller_interface::interface::AudioState;
use compositor_configurator_network_backend_base::base::WifiSnapshot;
use compositor_configurator_bluetooth_backend_base::base::BtSnapshot;
use compositor_configurator_settings_surface_cursor::cursor;
use compositor_configurator_settings_surface_display::display;
use compositor_configurator_settings_surface_environment::environment;
use compositor_configurator_settings_surface_keys::keys;
use compositor_configurator_audio_tab_base::base as audio_tab;
use compositor_configurator_network_tab_base::base as network_tab;
use compositor_configurator_bluetooth_tab_base::base as bluetooth_tab;
use compositor_configurator_settings_surface_message::message::{SettingsMessage, Tab};
use compositor_configurator_settings_surface_style::style;
use iced_core::alignment::Horizontal;
use iced_core::{Element, Length, Theme};
use iced_widget::{button, column, container, row, text};

type El<'a> = Element<'a, SettingsMessage, Theme, Renderer>;

fn confirm<'a>(p: ModeInfo) -> El<'a> {
    row![
        text("Keep new display mode?").width(Length::Fill),
        button(text("Keep")).on_press(SettingsMessage::Keep(p)).style(style::accent),
        button(text("Revert")).on_press(SettingsMessage::Revert).style(style::action),
    ]
    .spacing(8)
    .into()
}

#[allow(clippy::too_many_arguments)]
pub fn render<'a>(
    tab: Tab,
    dirty: bool,
    cursor_sensitivity: f32,
    natural: bool,
    env: &'a Environment,
    modes: &'a [ModeInfo],
    current: Option<ModeInfo>,
    picked: Option<ModeInfo>,
    confirming: bool,
    keys: &'a [KeyRow],
    audio: &'a AudioState,
    wifi: &'a WifiSnapshot,
    bt: &'a BtSnapshot,
    wifi_selected: Option<&'a str>,
    wifi_password: &'a str,
    render_devices: &'a [RenderDevice],
) -> El<'a> {
    let tabbtn = |label: &'static str, t: Tab| {
        button(text(label)).on_press(SettingsMessage::Tab(t)).style(style::tab(tab == t))
    };
    let header = row![
        text("Settings").size(24).width(Length::Fill),
        button(text("✕")).on_press(SettingsMessage::Close).style(style::action),
    ];
    let bar = column![
        row![tabbtn("Display", Tab::Display), tabbtn("Cursor", Tab::Cursor), tabbtn("Keys", Tab::Keys), tabbtn("System", Tab::Settings)].spacing(6),
        row![tabbtn("Audio", Tab::Audio), tabbtn("Wi-Fi", Tab::Wifi), tabbtn("Bluetooth", Tab::Bluetooth)].spacing(6),
    ]
    .spacing(6);
    let content: El<'a> = match tab {
        Tab::Display => display::build(modes, current),
        Tab::Cursor => cursor::build(cursor_sensitivity, natural),
        Tab::Keys => keys::build(keys),
        Tab::Audio => audio_tab::build(audio),
        Tab::Wifi => network_tab::build(wifi, wifi_selected, wifi_password),
        Tab::Bluetooth => bluetooth_tab::build(bt),
        Tab::Settings => environment::build(env, render_devices),
    };
    let mut col = column![header, bar].spacing(12).padding(18);
    if dirty {
        col = col.push(text("⚠  Some settings changed — reboot to apply.").size(13));
    }
    // Content fills the remaining height; the confirm bar then pins to the bottom.
    col = col.push(container(content).width(Length::Fill).height(Length::Fill));
    if confirming {
        if let Some(p) = picked {
            col = col.push(confirm(p));
        }
    }
    let panel = container(col).width(Length::Fixed(460.0)).height(Length::Fill).style(style::panel);
    container(panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Right)
        .style(style::backdrop)
        .into()
}
