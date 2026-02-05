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
        AppState::Idle => idle_view(app),
        AppState::Recording => {
            let elapsed = app.start_time.map(|t| t.elapsed());
            let frame_count = app.frame_count;
            recording_view(elapsed, frame_count, app)
        }
        AppState::Previewing { project_id, .. } => {
            preview_view(project_id, app.timeline.as_ref(), app)
        }
        AppState::ExportConfiguring { project_id, path } => {
            export_dialog_view(project_id, path, &app.export_dialog)
        }
        AppState::Exporting {
            project_id,
            progress,
        } => exporting_view(project_id, *progress, app),
        AppState::RegionSelecting => region_selecting_view(app),
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

fn idle_view(app: &FrameApp) -> Element<'_, Message> {
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

    // Main recording button
    let record_button = button(
        text("Start Recording")
            .size(18)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .padding([12, 24])
    .style(iced::theme::Button::Primary)
    .on_press(Message::StartRecording);

    // Phase 5: Region selection button
    let region_button = button(
        text("Select Region")
            .size(14)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .padding([10, 20])
    .style(iced::theme::Button::Secondary)
    .on_press(Message::StartRegionSelection);

    // Phase 5: Webcam toggle button
    let webcam_status = if app.webcam_config.enabled {
        "Webcam: On"
    } else {
        "Webcam: Off"
    };
    let webcam_button = button(
        text(webcam_status)
            .size(14)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .padding([10, 20])
    .style(iced::theme::Button::Secondary)
    .on_press(Message::ToggleWebcamSettings);

    // Phase 5: Settings button
    let settings_button = button(
        text("Effects")
            .size(14)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .padding([10, 20])
    .style(iced::theme::Button::Secondary)
    .on_press(Message::ToggleSettingsPanel);

    let main_buttons = row![record_button, Space::with_width(12), region_button].spacing(8);

    let config_buttons = row![webcam_button, Space::with_width(12), settings_button].spacing(8);

    let mut content = column![
        title,
        subtitle,
        Space::with_height(40),
        main_buttons,
        Space::with_height(16),
        config_buttons,
    ]
    .spacing(12)
    .align_items(Alignment::Center);

    // Phase 5: Show webcam settings if enabled
    if app.show_webcam_settings {
        let webcam_view = app
            .webcam_settings
            .view()
            .map(Message::WebcamSettingsMessage);
        content = content.push(Space::with_height(20));
        content = content.push(webcam_view);
    }

    // Phase 5: Show settings panel if enabled
    if app.show_settings_panel {
        let settings_view = app.settings_panel.view().map(Message::SettingsMessage);
        content = content.push(Space::with_height(20));
        content = content.push(settings_view);
    }

    content.into()
}

fn recording_view(
    elapsed: Option<std::time::Duration>,
    frame_count: u64,
    app: &FrameApp,
) -> Element<'_, Message> {
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

    // Phase 5: Webcam toggle during recording
    let webcam_status = if app.webcam_config.enabled {
        "Webcam: On"
    } else {
        "Webcam: Off"
    };
    let webcam_toggle = button(
        text(webcam_status)
            .size(14)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .padding([8, 16])
    .style(iced::theme::Button::Secondary)
    .on_press(Message::ToggleWebcamSettings);

    let mut content = column![
        recording_indicator,
        Space::with_height(20),
        timer,
        stats,
        Space::with_height(20),
        stop_button,
        Space::with_height(12),
        webcam_toggle,
    ]
    .spacing(12)
    .align_items(Alignment::Center);

    // Phase 5: Show webcam settings if enabled
    if app.show_webcam_settings {
        let webcam_view = app
            .webcam_settings
            .view()
            .map(Message::WebcamSettingsMessage);
        content = content.push(Space::with_height(16));
        content = content.push(webcam_view);
    }

    content.into()
}

fn preview_view<'a>(
    project_id: &'a str,
    timeline: Option<&'a Timeline>,
    app: &'a FrameApp,
) -> Element<'a, Message> {
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

    // Phase 5: Shadow/inset effect preview indicator
    let effects_text = format!(
        "Effects: Shadow={}, Inset={}",
        app.settings_panel.config().shadow.enabled,
        app.settings_panel.config().inset.enabled
    );
    let effects_label =
        text(effects_text)
            .size(12)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.5, 0.5, 0.5,
            )));
    content = content.push(Space::with_height(12));
    content = content.push(effects_label);

    content.into()
}

fn export_dialog_view<'a>(
    _project_id: &'a str,
    _path: &'a std::path::PathBuf,
    export_dialog: &'a frame_ui::export_dialog::ExportDialog,
) -> Element<'a, Message> {
    export_dialog.view(Message::ExportDialogMessage)
}

fn region_selecting_view(app: &FrameApp) -> Element<'_, Message> {
    let title = text("Select Recording Region")
        .size(32)
        .style(iced::theme::Text::Color(iced::Color::WHITE));

    let instructions = text(
        "Drag to select a region of your screen to record.\n\
         Press Enter to confirm or Escape to cancel.",
    )
    .size(14)
    .style(iced::theme::Text::Color(iced::Color::from_rgb(
        0.6, 0.6, 0.6,
    )));

    let cancel_button = button(
        text("Cancel")
            .size(14)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .padding([10, 20])
    .style(iced::theme::Button::Secondary)
    .on_press(Message::CancelRegionSelection);

    let region_canvas = app.region_selector.view().map(Message::RegionSelector);

    column![
        title,
        instructions,
        Space::with_height(20),
        region_canvas,
        Space::with_height(20),
        cancel_button,
    ]
    .spacing(12)
    .align_items(Alignment::Center)
    .into()
}

fn exporting_view<'a>(
    _project_id: &'a str,
    progress: f32,
    app: &'a FrameApp,
) -> Element<'a, Message> {
    // If export is complete (100%), show completion UI with clipboard option
    if progress >= 1.0 {
        let title = text("Export Complete!")
            .size(32)
            .style(iced::theme::Text::Color(iced::Color::WHITE));

        let message = text("Your recording has been exported successfully.")
            .size(14)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.6, 0.6, 0.6,
            )));

        let mut content = column![
            title,
            Space::with_height(10),
            message,
            Space::with_height(30),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        // Phase 5: Clipboard integration - copy file path
        if let Some(ref path) = app.last_exported_path {
            let copy_button = button(
                text("Copy File Path")
                    .size(14)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .padding([10, 20])
            .style(iced::theme::Button::Primary)
            .on_press(Message::CopyToClipboard(path.clone()));

            let new_recording_button = button(
                text("New Recording")
                    .size(14)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .padding([10, 20])
            .style(iced::theme::Button::Secondary)
            .on_press(Message::StartRecording);

            let buttons = row![copy_button, Space::with_width(12), new_recording_button].spacing(8);
            content = content.push(buttons);
        } else {
            let done_button = button(
                text("Done")
                    .size(14)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .padding([10, 20])
            .style(iced::theme::Button::Primary)
            .on_press(Message::ExportComplete);
            content = content.push(done_button);
        }

        return content.into();
    }

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
