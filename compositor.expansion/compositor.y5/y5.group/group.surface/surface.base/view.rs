//! Root view for the group surface.
//!
//! The surface is sized by the compositor from the bounding box of its
//! contained windows, padded 125px on every side plus an extra 125px on top
//! (250px total). Those windows are composited *on top* of this surface, so
//! the only place the surface is actually visible and clickable is the padding
//! bands. We use the generous 250px top band to host the group header (fold
//! arrow + editable name).
//!
//! The layout is identical whether expanded or collapsed: the header is pinned
//! to the top-left. When collapsed the compositor simply hands back a smaller
//! 500x250 surface anchored at the same origin, so the header does not move.
use iced_core::{Alignment, Background, Border, Element, Length, Padding, Shadow, Theme, Vector};
use iced_widget::{Space, column, container, mouse_area, row, text};
use compositor_support_iced_core_engine_base::Renderer;

use crate::message::GroupMessage;
use crate::mode::Mode;
use crate::style;
use crate::ui::GroupUi;

/// Inset of the header from the surface's top-left. Generous left padding so
/// the name sits comfortably away from the edge; kept identical across modes
/// so the header stays put when collapsing.
const HEADER_PAD: Padding = Padding {
    top: 28.0,
    right: 28.0,
    bottom: 0.0,
    left: 64.0,
};

/// Top-level view. Same layout for both modes; only the fold glyph differs.
pub fn root_view(ui: &GroupUi) -> Element<'_, GroupMessage, Theme, Renderer> {
    let body = column![header(ui), Space::new().height(Length::Fill)]
        .width(Length::Fill)
        .height(Length::Fill);

    panel(body.into()).padding(HEADER_PAD).into()
}

/// The fold arrow + name, laid out as a row pinned to the top-left.
fn header(ui: &GroupUi) -> Element<'_, GroupMessage, Theme, Renderer> {
    row![fold_arrow(ui), name_field(ui)]
        .spacing(14.0)
        .align_y(Alignment::Center)
        .into()
}

/// The fold toggle. Down-chevron when expanded (click to collapse), right-
/// chevron when collapsed (click to expand).
fn fold_arrow(ui: &GroupUi) -> Element<'_, GroupMessage, Theme, Renderer> {
    let (glyph, msg) = match ui.mode {
        Mode::Show => ("\u{25BE}", GroupMessage::Collapse), // ▾
        Mode::Collapse => ("\u{25B8}", GroupMessage::Show), // ▸
    };

    let arrow = text(glyph)
        .size(style::TEXT_SIZE_TITLE)
        .color(style::TEXT_DIM);

    mouse_area(
        container(arrow)
            .padding(style::PAD_SMALL)
            .style(|_theme| container::Style {
                background: Some(Background::Color(style::ICON_BG)),
                text_color: Some(style::TEXT),
                border: Border {
                    color: style::BORDER,
                    width: 1.0,
                    radius: style::RADIUS_SMALL.into(),
                },
                shadow: Shadow::default(),
                snap: true,
            }),
    )
    .on_press(msg)
    .into()
}

/// The name. A clickable label that turns into an editable field (with a
/// caret) while editing.
fn name_field(ui: &GroupUi) -> Element<'_, GroupMessage, Theme, Renderer> {
    if ui.editing {
        // Render the draft plus a caret. Editing is driven by keyboard events
        // routed through `event_process`, not a focused text_input.
        let shown = format!("{}\u{2502}", ui.draft); // draft + │ caret
        let field = text(shown).size(style::TEXT_SIZE_GROUP).color(style::TEXT);

        container(field)
            .padding(style::PAD_SMALL)
            .style(|_theme| container::Style {
                background: Some(Background::Color(style::PANEL_BG)),
                text_color: Some(style::TEXT),
                border: Border {
                    color: style::ACCENT,
                    width: 1.0,
                    radius: style::RADIUS_SMALL.into(),
                },
                shadow: Shadow {
                    color: style::GLOW,
                    offset: Vector::new(0.0, 0.0),
                    blur_radius: 8.0,
                },
                snap: true,
            })
            .into()
    } else {
        let label = if ui.name.trim().is_empty() {
            "Untitled group".to_string()
        } else {
            ui.name.clone()
        };

        let dim = ui.name.trim().is_empty();
        let t = text(label).size(style::TEXT_SIZE_GROUP).color(if dim {
            style::TEXT_HINT
        } else {
            style::TEXT
        });

        mouse_area(container(t).padding(style::PAD_SMALL))
            .on_press(GroupMessage::StartEdit)
            .into()
    }
}

/// Shared floating-panel container: semi-transparent dark background, rounded
/// corners, soft drop shadow.
fn panel<'a>(
    content: Element<'a, GroupMessage, Theme, Renderer>,
) -> container::Container<'a, GroupMessage, Theme, Renderer> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Background::Color(style::SURFACE_BG)),
            text_color: Some(style::TEXT),
            border: Border {
                color: style::BORDER,
                width: 1.0,
                radius: style::RADIUS_LARGE.into(),
            },
            shadow: Shadow {
                color: style::SHADOW,
                offset: Vector::new(0.0, 12.0),
                blur_radius: 32.0,
            },
            snap: true,
        })
}
