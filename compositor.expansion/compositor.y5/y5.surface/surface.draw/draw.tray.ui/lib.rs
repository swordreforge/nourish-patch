pub mod ui {
    use iced_core::{Color, Element, Theme};
    use iced_widget::{button, container, image, row, text, tooltip};
    use compositor_support_iced_core_engine_base::{IcedUi, Renderer};

    #[derive(Debug, Clone, Default)]
    pub struct TrayItem {
        pub service: String,
        pub icon_name: Option<String>,
        pub icon_pixmap: Option<Vec<u8>>,
        pub tooltip: String,
    }

    pub struct TrayUi { pub items: Vec<TrayItem> }

    #[derive(Debug, Clone)]
    pub enum TrayMessage { Sync, Activate(String) }

    impl IcedUi for TrayUi {
        type Message = TrayMessage;
        fn update(&mut self, _: TrayMessage) {}
        fn view(&self) -> Element<'_, TrayMessage, Theme, Renderer> {
            if self.items.is_empty() { return row![].into(); }
            let icons = self.items.iter().map(|item| {
                let svc = item.service.clone();
                let icon = if let Some(ref data) = item.icon_pixmap {
                    image(image::Handle::from_rgba(16,16,data.clone())).width(24).height(24)
                } else {
                    let name = item.icon_name.as_deref().unwrap_or("application-x-executable");
                    image(image::Handle::from_path(
                        format!("/usr/share/icons/hicolor/24x24/apps/{name}.png"))
                    ).width(24).height(24)
                };
                let btn = button(icon).padding(2).on_press(TrayMessage::Activate(svc));
                if item.tooltip.is_empty() { btn.into() }
                else { tooltip(btn, text(item.tooltip.clone()), tooltip::Position::Top).into() }
            }).collect::<Vec<Element<'_, _, _, _>>>();
            container(row(icons).spacing(4).padding([6,10]))
                .style(|_| container::Style {
                    background: Some(iced_core::Background::Color(Color::from_rgba(
                        0.125, 0.106, 0.078, 0.92))),
                    border: iced_core::Border { radius: 10.0.into(), ..Default::default() },
                    ..Default::default()
                }).into()
        }
    }
}
