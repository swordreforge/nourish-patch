use compositor_configurator_settings_surface_message::message::{SettingsMessage, WallpaperFill};
use compositor_configurator_settings_surface_style::style;
use compositor_configurator_settings_surface_control::control;
use compositor_support_iced_core_engine_base::Renderer;
use iced_core::{Alignment, Element, Length, Theme};
use compositor_support_library_i18n_base_core::t;
use iced_widget::{button, column, container, pick_list, row, text, text_input};

type El<'a> = Element<'a, SettingsMessage, Theme, Renderer>;

fn heading<'a>(title: &'a str, sub: &'a str) -> El<'a> {
    column![text(title).size(13).color(style::ACCENT), text(sub).size(11).color(style::MUTED)].spacing(2).into()
}

fn fill_label(f: WallpaperFill) -> String {
    match f { WallpaperFill::Tile => "Tile", WallpaperFill::Cover => "Cover", WallpaperFill::Fit => "Fit", WallpaperFill::Center => "Center" }.to_string()
}

fn fill_from_label(s: String) -> SettingsMessage {
    SettingsMessage::SetWallpaperFill(match s.as_str() {
        "Cover" => WallpaperFill::Cover, "Fit" => WallpaperFill::Fit, "Center" => WallpaperFill::Center,
        _ => WallpaperFill::Tile,
    })
}

pub fn build<'a>(path: Option<&'a str>, edit_buf: &'a str, fill: WallpaperFill) -> El<'a> {
    let title = column![text(t!("WALLPAPER")).size(16).color(style::ACCENT), text(t!("Set a background image for the active world.")).size(11).color(style::MUTED)].spacing(4);
    let path_input = text_input("/path/to/image.jpg", edit_buf)
        .on_input(SettingsMessage::SetWallpaperPath)
        .width(Length::Fill);
    let clear_btn = if path.is_some() {
        button(text(t!("CLEAR")).size(12)).on_press(SettingsMessage::SetWallpaperPath(String::new())).style(control::action)
    } else {
        button(text(t!("CLEAR")).size(12)).style(control::action)
    };
    let path_row = container(
        row![path_input, clear_btn].align_y(Alignment::Center).spacing(8),
    ).style(style::card).width(Length::Fill).padding(12);
    let desc = match fill {
        WallpaperFill::Tile => t!("Tiles the image across the infinite canvas at native resolution."),
        WallpaperFill::Cover => t!("Scales the image to fill the screen, cropping excess."),
        WallpaperFill::Fit => t!("Scales the image to fit within the screen, with letterbox bars."),
        WallpaperFill::Center => t!("Places the image at native size, centered on screen."),
    };
    let labels: Vec<String> = vec!["Tile", "Cover", "Fit", "Center"].into_iter().map(String::from).collect();
    let fill_picker = pick_list(Some(fill_label(fill)), labels, |s: &String| s.clone())
        .on_select(fill_from_label).width(Length::Fixed(160.0)).style(control::picklist).menu_style(control::menu);
    let fill_row = container(
        row![column![text(t!("FILL MODE")).size(13).color(style::ACCENT), text(desc).size(11).color(style::MUTED)].spacing(2).width(Length::Fill), fill_picker].align_y(Alignment::Center).spacing(10),
    ).style(style::card).width(Length::Fill).padding(12);
    column![title, heading(t!("IMAGE"), t!("The source image file for the wallpaper.")), path_row, heading(t!("FILL MODE"), desc), fill_row].spacing(12).into()
}
