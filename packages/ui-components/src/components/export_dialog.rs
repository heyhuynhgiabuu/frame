//! Export dialog for configuring export settings

use iced::{
    widget::{button, column, container, pick_list, row, slider, text, Space},
    Alignment, Element, Length,
};
use std::path::PathBuf;

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// MP4 with H.264
    Mp4,
    /// GIF animation
    Gif,
    /// WebM with VP9
    WebM,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportFormat::Mp4 => write!(f, "MP4 (H.264)"),
            ExportFormat::Gif => write!(f, "GIF"),
            ExportFormat::WebM => write!(f, "WebM (VP9)"),
        }
    }
}

impl Default for ExportFormat {
    fn default() -> Self {
        Self::Mp4
    }
}

/// Quality preset for export
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityPreset {
    /// Low quality, small file
    Low,
    /// Medium quality
    Medium,
    /// High quality
    High,
    /// Maximum quality, large file
    Maximum,
}

impl std::fmt::Display for QualityPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualityPreset::Low => write!(f, "Low (smaller file)"),
            QualityPreset::Medium => write!(f, "Medium"),
            QualityPreset::High => write!(f, "High"),
            QualityPreset::Maximum => write!(f, "Maximum (larger file)"),
        }
    }
}

impl Default for QualityPreset {
    fn default() -> Self {
        Self::High
    }
}

impl QualityPreset {
    /// Get CRF value for H.264 encoding
    pub fn crf_value(&self) -> u32 {
        match self {
            QualityPreset::Low => 28,
            QualityPreset::Medium => 23,
            QualityPreset::High => 20,
            QualityPreset::Maximum => 17,
        }
    }

    /// Get bitrate for encoding
    pub fn bitrate_mbps(&self) -> u64 {
        match self {
            QualityPreset::Low => 2_000_000,      // 2 Mbps
            QualityPreset::Medium => 6_000_000,   // 6 Mbps
            QualityPreset::High => 12_000_000,    // 12 Mbps
            QualityPreset::Maximum => 24_000_000, // 24 Mbps
        }
    }
}

/// Resolution options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportResolution {
    /// Original resolution
    Original,
    /// 1080p
    P1080,
    /// 720p
    P720,
    /// 480p
    P480,
}

impl std::fmt::Display for ExportResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportResolution::Original => write!(f, "Original"),
            ExportResolution::P1080 => write!(f, "1080p"),
            ExportResolution::P720 => write!(f, "720p"),
            ExportResolution::P480 => write!(f, "480p"),
        }
    }
}

impl Default for ExportResolution {
    fn default() -> Self {
        Self::Original
    }
}

/// Export configuration
#[derive(Debug, Clone)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub quality: QualityPreset,
    pub resolution: ExportResolution,
    pub destination: PathBuf,
    pub filename: String,
    pub include_audio: bool,
    pub fps: u32,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::default(),
            quality: QualityPreset::default(),
            resolution: ExportResolution::default(),
            destination: PathBuf::from("."),
            filename: "recording".to_string(),
            include_audio: true,
            fps: 30,
        }
    }
}

impl ExportConfig {
    /// Get the full output path
    pub fn output_path(&self) -> PathBuf {
        let extension = match self.format {
            ExportFormat::Mp4 => "mp4",
            ExportFormat::Gif => "gif",
            ExportFormat::WebM => "webm",
        };
        self.destination
            .join(format!("{}.{}.", self.filename, extension))
    }
}

/// Messages for export dialog
#[derive(Debug, Clone)]
pub enum ExportDialogMessage {
    FormatChanged(ExportFormat),
    QualityChanged(QualityPreset),
    ResolutionChanged(ExportResolution),
    DestinationChanged(PathBuf),
    FilenameChanged(String),
    IncludeAudioChanged(bool),
    FpsChanged(u32),
    StartExport,
    CancelExport,
}

/// Export dialog component
pub struct ExportDialog {
    pub config: ExportConfig,
    pub is_open: bool,
}

impl ExportDialog {
    pub fn new() -> Self {
        Self {
            config: ExportConfig::default(),
            is_open: false,
        }
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn update(&mut self, message: ExportDialogMessage) {
        match message {
            ExportDialogMessage::FormatChanged(format) => self.config.format = format,
            ExportDialogMessage::QualityChanged(quality) => self.config.quality = quality,
            ExportDialogMessage::ResolutionChanged(resolution) => {
                self.config.resolution = resolution
            }
            ExportDialogMessage::DestinationChanged(dest) => self.config.destination = dest,
            ExportDialogMessage::FilenameChanged(name) => self.config.filename = name,
            ExportDialogMessage::IncludeAudioChanged(include) => {
                self.config.include_audio = include
            }
            ExportDialogMessage::FpsChanged(fps) => self.config.fps = fps,
            ExportDialogMessage::StartExport | ExportDialogMessage::CancelExport => {}
        }
    }

    pub fn view<'a, Message: Clone + 'a>(
        &'a self,
        on_message: impl Fn(ExportDialogMessage) -> Message + Clone + 'a,
    ) -> Element<'a, Message> {
        let title = text("Export Recording")
            .size(24)
            .style(iced::theme::Text::Color(iced::Color::WHITE));

        // Format selection
        let format_label = text("Format:").size(14);
        let on_message_clone = on_message.clone();
        let format_pick_list = pick_list(
            vec![ExportFormat::Mp4, ExportFormat::Gif, ExportFormat::WebM],
            Some(self.config.format),
            move |format| on_message_clone(ExportDialogMessage::FormatChanged(format)),
        )
        .width(Length::Fixed(200.0));

        // Quality selection
        let quality_label = text("Quality:").size(14);
        let on_message_clone = on_message.clone();
        let quality_pick_list = pick_list(
            vec![
                QualityPreset::Low,
                QualityPreset::Medium,
                QualityPreset::High,
                QualityPreset::Maximum,
            ],
            Some(self.config.quality),
            move |quality| on_message_clone(ExportDialogMessage::QualityChanged(quality)),
        )
        .width(Length::Fixed(200.0));

        // Resolution selection
        let resolution_label = text("Resolution:").size(14);
        let on_message_clone = on_message.clone();
        let resolution_pick_list = pick_list(
            vec![
                ExportResolution::Original,
                ExportResolution::P1080,
                ExportResolution::P720,
                ExportResolution::P480,
            ],
            Some(self.config.resolution),
            move |resolution| on_message_clone(ExportDialogMessage::ResolutionChanged(resolution)),
        )
        .width(Length::Fixed(200.0));

        // FPS slider
        let fps_label = text(format!("Frame Rate: {} fps", self.config.fps)).size(14);
        let on_message_clone = on_message.clone();
        let fps_slider = slider(15..=60, self.config.fps, move |fps| {
            on_message_clone(ExportDialogMessage::FpsChanged(fps))
        })
        .width(Length::Fixed(200.0));

        // Filename input (simplified - would use proper text input)
        let filename_label = text("Filename:").size(14);
        let filename_text = text(&self.config.filename).size(14);

        // Output path preview
        let output_label = text("Output:").size(14);
        let output_path = text(self.config.output_path().to_string_lossy()).size(12);

        // Buttons
        let on_message_clone = on_message.clone();
        let export_button = button(
            text("Export")
                .size(14)
                .horizontal_alignment(iced::alignment::Horizontal::Center),
        )
        .padding([10, 20])
        .style(iced::theme::Button::Primary)
        .on_press(on_message_clone(ExportDialogMessage::StartExport));

        let on_message_clone = on_message.clone();
        let cancel_button = button(
            text("Cancel")
                .size(14)
                .horizontal_alignment(iced::alignment::Horizontal::Center),
        )
        .padding([10, 20])
        .style(iced::theme::Button::Secondary)
        .on_press(on_message_clone(ExportDialogMessage::CancelExport));

        let buttons = row![export_button, Space::with_width(12), cancel_button].spacing(8);

        let content = column![
            title,
            Space::with_height(20),
            format_label,
            format_pick_list,
            Space::with_height(12),
            quality_label,
            quality_pick_list,
            Space::with_height(12),
            resolution_label,
            resolution_pick_list,
            Space::with_height(12),
            fps_label,
            fps_slider,
            Space::with_height(12),
            filename_label,
            filename_text,
            Space::with_height(12),
            output_label,
            output_path,
            Space::with_height(30),
            buttons,
        ]
        .spacing(4)
        .align_items(Alignment::Start);

        container(content)
            .width(Length::Fixed(400.0))
            .padding(20)
            .style(iced::theme::Container::Box)
            .into()
    }
}

impl Default for ExportDialog {
    fn default() -> Self {
        Self::new()
    }
}
