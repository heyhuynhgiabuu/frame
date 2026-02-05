use iced::widget::text_input as iced_text_input;

pub fn text_input<'a, Message: Clone>(
    placeholder: &'a str,
    value: &'a str,
) -> iced_text_input::TextInput<'a, Message> {
    iced_text_input::TextInput::new(placeholder, value)
}
