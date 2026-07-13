//! Active-handler picker for Settings mode.

use compositor_support_library_i18n_base_core::t;
use iced_core::{Alignment, Element, Length, Theme};
use iced_widget::{column, container, pick_list, row, text};
use compositor_support_iced_core_engine_base::Renderer;

use crate::message::PlaceholderMessage;
use crate::style;
use crate::ui::PlaceholderUi;

/// Display option for the pick_list. Includes the explicit "(none)"
/// choice so the user can disable handler synthesis entirely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HandlerChoice {
    pub label: String,
    pub handler: Option<compositor_introspection_extraction_window_base::HandlerId>,
}

impl std::fmt::Display for HandlerChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

pub fn render(ui: &PlaceholderUi) -> Element<'_, PlaceholderMessage, Theme, Renderer> {
    // Build the option set: (none) plus every registered handler.
    let mut options: Vec<HandlerChoice> = vec![HandlerChoice {
        label: "(none)".to_string(),
        handler: None,
    }];
    for (handler, label) in ui.handler_choices() {
        options.push(HandlerChoice {
            label,
            handler: Some(handler),
        });
    }

    let selected = HandlerChoice {
        label: match ui.working.active_handler {
            None => "(none)".to_string(),
            Some(h) => h.to_string(),
        },
        handler: ui.working.active_handler,
    };

    let label = text(t!("Active handler"))
        .size(style::TEXT_SIZE_SECTION)
        .style(|_| iced_widget::text::Style {
            color: Some(style::TEXT),
        });

    let picker = pick_list(Some(selected), options, |choice: &HandlerChoice| {
        choice.label.clone()
    })
    .on_select(|choice: HandlerChoice| PlaceholderMessage::ActiveHandlerChanged(choice.handler))
    .width(Length::Fill);

    let hint = text(t!("Switching the handler changes which synthesizer runs at launch. \
         Each handler's preferences are preserved when switching."))
    .size(style::TEXT_SIZE_HINT)
    .style(|_| iced_widget::text::Style {
        color: Some(style::TEXT_HINT),
    });

    let inner = column![label, picker, hint].spacing(6);

    container(inner)
        .padding(style::PAD_MEDIUM)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced_core::Background::Color(style::PANEL_BG)),
            border: iced_core::Border {
                color: style::BORDER,
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: iced_core::Shadow::default(),
            text_color: Some(style::TEXT),
            snap: true,
        })
        .into()
}
