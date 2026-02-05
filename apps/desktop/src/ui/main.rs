//! Main UI views for different app states

use crate::app::{AppState, FrameApp, Message};
use frame_ui::timeline::Timeline;
use iced::{
    widget::{button, column, container, progress_bar, row, text, Space},
    Alignment, Element, Length,
};

pub fn main_view(app: &FrameApp) -> Element<'_, Message> {
    let content: Element<Message> = match &app.state {
        AppState::CheckingPermissions => checking_permissions_view(),
        AppState::PermissionRequired { screen, microphone } => {
            permission_required_view(*screen, *microphone)
        }
        AppState::Idle => idle_view(),
        AppState::Recording => {
            let elapsed = app.start_time.map(|t| t.elapsed());
            let frame_count = app.frame_count;
            recording_view(elapsed, frame_count)
        }
        AppState::Previewing { project_id, .. } => preview_view(project_id, app.timeline.as_ref()),
        AppState::ExportConfiguring { project_id, path } => {
            export_dialog_view(project_id, path, &app.export_dialog)
        }
        AppState::Exporting {
            project_id,
            progress,
        } => exporting_view(project_id, *progress),
        AppState::Error(msg) => error_view(msg),
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20)
        .into()
}

fn checking_permissions_view() -> Element<'static, Message> {
    let title = text("Frame")
        .size(48)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(
            0.9, 0.9, 0.9,
        )));

    let status = text("Checking permissions...")
        .size(16)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(
            0.6, 0.6, 0.6,
        )));

    column![title, status,]
        .spacing(12)
        .align_items(Alignment::Center)
        .into()
}

fn permission_required_view(
    screen_needed: bool,
    microphone_needed: bool,
) -> Element<'static, Message> {
    let title = text("Permissions Required")
        .size(32)
        .style(iced::theme::Text::Color(iced::Color::WHITE));

    let description = text(
        "Frame needs permission to record your screen and audio.\n\
         Please grant the required permissions to continue.",
    )
    .size(14)
    .style(iced::theme::Text::Color(iced::Color::from_rgb(
        0.6, 0.6, 0.6,
    )));

    let mut buttons = column![].spacing(12);

    if screen_needed {
        buttons = buttons.push(
            button(
                text("Grant Screen Recording Permission")
                    .size(14)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .padding([12, 24])
            .style(iced::theme::Button::Primary)
            .on_press(Message::RequestScreenPermission),
        );
    }

    if microphone_needed {
        buttons = buttons.push(
            button(
                text("Grant Microphone Permission")
                    .size(14)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .padding([12, 24])
            .style(iced::theme::Button::Primary)
            .on_press(Message::RequestMicrophonePermission),
        );
    }

    column![title, description, Space::with_height(30), buttons,]
        .spacing(12)
        .align_items(Alignment::Center)
        .into()
}

fn idle_view() -> Element<'static, Message> {
    let title = text("Frame")
        .size(48)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(
            0.9, 0.9, 0.9,
        )));

    let subtitle = text("Beautiful screen recordings for developers")
        .size(16)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(
            0.6, 0.6, 0.6,
        )));

    let record_button = button(
        text("Start Recording")
            .size(18)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .padding([12, 24])
    .style(iced::theme::Button::Primary)
    .on_press(Message::StartRecording);

    column![title, subtitle, Space::with_height(40), record_button,]
        .spacing(12)
        .align_items(Alignment::Center)
        .into()
}

fn recording_view(
    elapsed: Option<std::time::Duration>,
    frame_count: u64,
) -> Element<'static, Message> {
    let duration = elapsed
        .map(format_duration)
        .unwrap_or_else(|| "00:00:00".to_string());

    let recording_indicator = row![
        Space::with_width(8),
        container(Space::with_width(12))
            .style(iced::theme::Container::Custom(Box::new(RecordingDot)))
            .width(12)
            .height(12),
        text("Recording").size(14),
    ]
    .spacing(8)
    .align_items(Alignment::Center);

    let timer = text(duration)
        .size(64)
        .style(iced::theme::Text::Color(iced::Color::WHITE));

    let stats = text(format!("{} frames captured", frame_count))
        .size(12)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(
            0.5, 0.5, 0.5,
        )));

    let stop_button = button(
        text("Stop Recording")
            .size(16)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .padding([10, 20])
    .style(iced::theme::Button::Destructive)
    .on_press(Message::StopRecording);

    column![
        recording_indicator,
        Space::with_height(20),
        timer,
        stats,
        Space::with_height(20),
        stop_button,
    ]
    .spacing(12)
    .align_items(Alignment::Center)
    .into()
}

fn preview_view<'a>(project_id: &'a str, timeline: Option<&'a Timeline>) -> Element<'a, Message> {
    let title = text("Recording Complete")
        .size(32)
        .style(iced::theme::Text::Color(iced::Color::WHITE));

    let info =
        text(format!("Project ID: {}", project_id))
            .size(14)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.6, 0.6, 0.6,
            )));

    let actions = row![
        button(text("Export").size(14))
            .padding([10, 20])
            .style(iced::theme::Button::Primary)
            .on_press(Message::ExportProject(project_id.to_string())),
        Space::with_width(12),
        button(text("New Recording").size(14))
            .padding([10, 20])
            .style(iced::theme::Button::Secondary)
            .on_press(Message::StartRecording),
    ]
    .spacing(8);

    // Build main content
    let mut content = column![title, info, Space::with_height(20), actions,]
        .spacing(12)
        .align_items(Alignment::Center);

    // Add timeline if available
    if let Some(timeline) = timeline {
        let timeline_widget = timeline.view().map(|msg| match msg {
            frame_ui::timeline::TimelineMessage::PositionChanged(pos) => {
                Message::TimelinePositionChanged(pos)
            }
            _ => Message::TimelinePositionChanged(timeline.position()),
        });
        content = content.push(Space::with_height(20));
        content = content.push(timeline_widget);
    }

    content.into()
}

fn export_dialog_view<'a>(
    _project_id: &'a str,
    _path: &'a std::path::PathBuf,
    export_dialog: &'a frame_ui::export_dialog::ExportDialog,
) -> Element<'a, Message> {
    export_dialog.view(Message::ExportDialogMessage)
}

fn exporting_view(_project_id: &str, progress: f32) -> Element<'static, Message> {
    let title = text("Exporting...")
        .size(32)
        .style(iced::theme::Text::Color(iced::Color::WHITE));

    let progress_bar_widget = progress_bar(0.0..=1.0, progress)
        .width(Length::Fixed(300.0))
        .height(Length::Fixed(8.0));

    let percentage =
        text(format!("{:.0}%", progress * 100.0))
            .size(14)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.6, 0.6, 0.6,
            )));

    column![
        title,
        Space::with_height(30),
        progress_bar_widget,
        percentage,
    ]
    .spacing(12)
    .align_items(Alignment::Center)
    .into()
}

fn error_view(error: &str) -> Element<'static, Message> {
    let title = text("Error")
        .size(32)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(
            1.0, 0.3, 0.3,
        )));

    let message = text(error)
        .size(14)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(
            0.8, 0.8, 0.8,
        )));

    let retry_button = button(
        text("Try Again")
            .size(14)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .padding([10, 20])
    .style(iced::theme::Button::Primary)
    .on_press(Message::CheckPermissions);

    column![title, message, Space::with_height(30), retry_button,]
        .spacing(12)
        .align_items(Alignment::Center)
        .into()
}

// Custom style for recording indicator
struct RecordingDot;

impl iced::widget::container::StyleSheet for RecordingDot {
    type Style = iced::Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                1.0, 0.2, 0.2,
            ))),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    }
}

/// Format duration as HH:MM:SS
fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}
