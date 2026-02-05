use crate::app::Message;
use iced::{
    widget::{button, row, text},
    Alignment, Element, Length,
};

pub fn toolbar_view() -> Element<'static, Message> {
    row![
        button(text("Settings").size(12))
            .padding([6, 12])
            .style(iced::theme::Button::Text),
        iced::widget::Space::with_width(Length::Fill),
        button(text("Help").size(12))
            .padding([6, 12])
            .style(iced::theme::Button::Text),
    ]
    .spacing(8)
    .align_items(Alignment::Center)
    .into()
}
