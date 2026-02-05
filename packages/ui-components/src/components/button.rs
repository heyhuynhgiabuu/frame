use iced::widget::button;

pub fn primary_button<'a, Message: Clone>(label: &'a str) -> button::Button<'a, Message> {
    button(label).style(iced::theme::Button::Primary)
}

pub fn secondary_button<'a, Message: Clone>(label: &'a str) -> button::Button<'a, Message> {
    button(label).style(iced::theme::Button::Secondary)
}
