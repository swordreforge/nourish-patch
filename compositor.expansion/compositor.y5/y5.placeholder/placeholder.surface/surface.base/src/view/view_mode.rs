//! View mode: centered icon + title + app_id + Launch / Edit buttons.
//!
//! Visual goals:
//! - Whole content centered both axes inside the surface.
//! - Icon sits inside a soft "glassy" circular backdrop with a subtle
//!   highlight border and outer glow — approximating glassmorphism
//!   without backdrop blur (which iced doesn't support natively).
//! - Buttons use the accent color, rounded corners, slightly raised.

use std::path::PathBuf;

use compositor_support_library_i18n_base_core::t;
use iced_core::{alignment, Alignment, Background, Border, Color, ContentFit, Element, Length, Padding, Shadow, Theme, Vector};
use iced_widget::{button, column, container, image, row, svg, text, Space};
use compositor_introspection_extraction_window_base::attributes::{DisplayName, IconPath};
use compositor_support_iced_core_engine_base::Renderer;

use crate::message::PlaceholderMessage;
use crate::style;
use crate::ui::PlaceholderUi;

const ICON_PX: f32 = 96.0;
/// Outer "glass" backdrop is a bit larger than the icon itself.
const ICON_BACKDROP_PX: f32 = 132.0;

pub fn render(ui: &PlaceholderUi) -> Element<'_, PlaceholderMessage, Theme, Renderer> {
    let plan = ui.shown_plan();

    let icon = match plan.current::<IconPath>() {
        Some(path) => render_icon(path),
        None => fallback_glyph(),
    };
    let icon_with_backdrop = icon;
    // let icon_with_backdrop = backdrop(icon);

    let display_name = plan
        .current::<DisplayName>()
        .unwrap_or_else(|| "Unknown".to_string());
    let app_id = plan.application_data.meta.meta.app_id.clone();

    let title =
        text(display_name)
            .size(style::TEXT_SIZE_TITLE)
            .style(|_| iced_widget::text::Style {
                color: Some(style::TEXT),
            });

    let app_id_line: Element<'_, _, _, _> = match app_id {
        Some(s) => text(s)
            .size(style::TEXT_SIZE_HINT)
            .style(|_| iced_widget::text::Style {
                color: Some(style::TEXT_DIM),
            })
            .into(),
        None => text("").size(style::TEXT_SIZE_HINT).into(),
    };

    let edit_btn =
        button(
            text(t!("Edit"))
                .size(style::TEXT_SIZE_BODY)
                .style(|_| iced_widget::text::Style {
                    color: Some(style::TEXT),
                }),
        )
        .padding(style::PAD_MEDIUM)
        .on_press(PlaceholderMessage::EnterSettings)
        .style(button_secondary);
    
    let dismiss_btn =
        button(
            text(t!("Dismiss"))
                .size(style::TEXT_SIZE_BODY)
                .style(|_| iced_widget::text::Style {
                    color: Some(style::TEXT),
                }),
        )
        .padding(style::PAD_MEDIUM)
        .on_press(PlaceholderMessage::DismissClicked)
        .style(button_secondary);

    let launch_btn =
        button(
            text(t!("Launch"))
                .size(style::TEXT_SIZE_BODY)
                .style(|_| iced_widget::text::Style {
                    color: Some(Color::WHITE),
                }),
        )
        .padding(style::PAD_MEDIUM)
        .on_press(PlaceholderMessage::LaunchClicked)
        .style(button_primary);

    let buttons = row![edit_btn, launch_btn, dismiss_btn]
        .spacing(12)
        .align_y(Alignment::Center);



    let body = column![
        icon_with_backdrop,
        title,
        app_id_line,
        buttons,
    ]
    .spacing(16)
    .align_x(Alignment::Center);

    container(body)
        .padding(Padding::new(32.0).top(128))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_y(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
}

// ── Icon rendering ──────────────────────────────────────────────────

/// Wrap the icon in a soft circular "glassy" backdrop.
fn backdrop<'a>(
    inner: Element<'a, PlaceholderMessage, Theme, Renderer>,
) -> Element<'a, PlaceholderMessage, Theme, Renderer> {
    container(inner)
        .width(Length::Fixed(ICON_BACKDROP_PX))
        .height(Length::Fixed(ICON_BACKDROP_PX))
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .style(|_| container::Style {
            background: Some(Background::Color(style::ICON_BG)),
            border: Border {
                color: style::ICON_HIGHLIGHT,
                width: 1.0,
                radius: (ICON_BACKDROP_PX / 2.0).into(),
            },
            // Outer accent glow gives the "lit from behind" feeling.
            shadow: Shadow {
                color: style::GLOW,
                offset: Vector::new(0.0, 0.0),
                blur_radius: 28.0,
            },
            text_color: Some(style::TEXT),
            snap: true,
        })
        .into()
}

fn render_icon<'a>(path: PathBuf) -> Element<'a, PlaceholderMessage, Theme, Renderer> {
    let is_svg = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("svg"))
        .unwrap_or(false);

    if is_svg {
        svg(svg::Handle::from_path(path))
            .width(Length::Fixed(ICON_PX))
            .height(Length::Fixed(ICON_PX))
            .content_fit(ContentFit::Contain)
            .into()
    } else {
        image(image::Handle::from_path(path))
            .width(Length::Fixed(ICON_PX))
            .height(Length::Fixed(ICON_PX))
            .content_fit(ContentFit::Contain)
            .into()
    }
}

fn fallback_glyph<'a>() -> Element<'a, PlaceholderMessage, Theme, Renderer> {
    container(
        text("?")
            .size(ICON_PX / 2.0)
            .style(|_| iced_widget::text::Style {
                color: Some(style::TEXT_HINT),
            }),
    )
    .width(Length::Fixed(ICON_PX))
    .height(Length::Fixed(ICON_PX))
    .align_x(alignment::Horizontal::Center)
    .align_y(alignment::Vertical::Center)
    .into()
}

// ── Button styles ───────────────────────────────────────────────────

fn button_primary(
    _theme: &Theme,
    status: iced_widget::button::Status,
) -> iced_widget::button::Style {
    use iced_widget::button::Status;
    let bg = match status {
        Status::Hovered => Color {
            r: 0.50,
            g: 0.72,
            b: 0.98,
            a: 1.0,
        },
        Status::Pressed => Color {
            r: 0.30,
            g: 0.55,
            b: 0.88,
            a: 1.0,
        },
        _ => style::ACCENT,
    };
    iced_widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            color: style::BORDER_BRIGHT,
            width: 0.0,
            radius: style::RADIUS_MEDIUM.into(),
        },
        shadow: Shadow {
            color: style::GLOW,
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        snap: true,
    }
}

fn button_secondary(
    _theme: &Theme,
    status: iced_widget::button::Status,
) -> iced_widget::button::Style {
    use iced_widget::button::Status;
    let bg = match status {
        Status::Hovered => Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.10,
        },
        Status::Pressed => Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.16,
        },
        _ => Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.05,
        },
    };
    iced_widget::button::Style {
        background: Some(Background::Color(bg)),
        text_color: style::TEXT,
        border: Border {
            color: style::BORDER_BRIGHT,
            width: 1.0,
            radius: style::RADIUS_MEDIUM.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}
