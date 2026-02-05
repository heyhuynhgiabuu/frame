//! Export dialog for configuring export settings

use frame_core::effects::aspect_ratio::AspectRatio;
use frame_core::export_preset::{ExportPreset, PresetManager};
use iced::{
    widget::{button, column, container, pick_list, row, slider, text, text_input, Space},
    Alignment, Element, Length,
};
use std::path::PathBuf;

/// Messages for export preset selector
#[derive(Debug, Clone)]
pub enum ExportPresetMessage {
    /// User selected a preset from the dropdown
    PresetSelected(PresetOption),
    /// User clicked Save as Preset button
    SaveAsPreset,
    /// User clicked Delete button on a custom preset
    DeletePreset(String),
    /// User changed the name of a custom preset
    PresetNameChanged(String),
    /// User confirmed editing preset name
    ConfirmPresetNameEdit,
    /// User cancelled editing preset name
    CancelPresetNameEdit,
}

/// A selectable preset option for the dropdown
#[derive(Debug, Clone, PartialEq)]
pub struct PresetOption {
    pub id: String,
    pub name: String,
    pub is_built_in: bool,
}

impl std::fmt::Display for PresetOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let icon = if self.is_built_in { "★ " } else { "  " };
        write!(f, "{}{}", icon, self.name)
    }
}

/// Export preset selector widget for managing and selecting presets
pub struct ExportPresetSelector {
    preset_manager: PresetManager,
    selected_preset_id: Option<String>,
    editing_preset_id: Option<String>,
    editing_name: String,
}

impl Default for ExportPresetSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl ExportPresetSelector {
    /// Create a new export preset selector
    pub fn new() -> Self {
        Self {
            preset_manager: PresetManager::new(),
            selected_preset_id: None,
            editing_preset_id: None,
            editing_name: String::new(),
        }
    }

    /// Load presets from disk
    pub async fn load_presets(&mut self) -> Result<(), frame_core::error::FrameError> {
        self.preset_manager.load_presets().await?;
        // Select first preset if none selected
        if self.selected_preset_id.is_none() {
            if let Some(first) = self.get_all_options().first() {
                self.selected_preset_id = Some(first.id.clone());
            }
        }
        Ok(())
    }

    /// Get the currently selected preset
    pub fn selected_preset(&self) -> Option<&ExportPreset> {
        self.selected_preset_id
            .as_ref()
            .and_then(|id| self.preset_manager.get_preset(id))
    }

    /// Get the selected preset ID
    pub fn selected_preset_id(&self) -> Option<&str> {
        self.selected_preset_id.as_deref()
    }

    /// Get all preset options sorted: built-ins first, then custom
    pub fn get_all_options(&self) -> Vec<PresetOption> {
        let mut options: Vec<PresetOption> = Vec::new();

        // Add built-in presets first
        for preset in self.preset_manager.get_builtin_presets() {
            options.push(PresetOption {
                id: preset.id.clone(),
                name: preset.name.clone(),
                is_built_in: true,
            });
        }

        // Add separator placeholder if there are user presets
        let user_presets = self.preset_manager.get_user_presets();
        if !user_presets.is_empty() {
            options.push(PresetOption {
                id: "__separator__".to_string(),
                name: "--- Custom Presets ---".to_string(),
                is_built_in: false,
            });
        }

        // Add user presets
        for preset in user_presets {
            options.push(PresetOption {
                id: preset.id.clone(),
                name: preset.name.clone(),
                is_built_in: false,
            });
        }

        options
    }

    /// Get the selected preset option
    fn get_selected_option(&self) -> Option<PresetOption> {
        self.selected_preset_id.as_ref().and_then(|id| {
            self.preset_manager.get_preset(id).map(|p| PresetOption {
                id: p.id.clone(),
                name: p.name.clone(),
                is_built_in: p.is_built_in,
            })
        })
    }

    /// Apply a preset's settings to export config
    pub fn apply_preset_to_config(&self, config: &mut ExportConfig) {
        if let Some(preset) = self.selected_preset() {
            // Map export preset format to export dialog format
            config.format = match preset.format {
                frame_core::export_preset::ExportOutputFormat::Mp4 => ExportFormat::Mp4,
                frame_core::export_preset::ExportOutputFormat::Mov => ExportFormat::Mp4, // Map to MP4
                frame_core::export_preset::ExportOutputFormat::Webm => ExportFormat::WebM,
            };

            // Map quality preset
            config.quality = match preset.quality {
                frame_core::export_preset::QualityPreset::Low => QualityPreset::Low,
                frame_core::export_preset::QualityPreset::Medium => QualityPreset::Medium,
                frame_core::export_preset::QualityPreset::High => QualityPreset::High,
                frame_core::export_preset::QualityPreset::Lossless => QualityPreset::Maximum,
                frame_core::export_preset::QualityPreset::Custom => QualityPreset::High,
            };

            config.fps = preset.fps;
            // Note: resolution and aspect_ratio would need additional mapping
        }
    }

    /// Handle messages
    pub fn update(&mut self, message: ExportPresetMessage) -> Option<ExportPreset> {
        match message {
            ExportPresetMessage::PresetSelected(option) => {
                if option.id != "__separator__" {
                    self.selected_preset_id = Some(option.id);
                    self.selected_preset().cloned()
                } else {
                    None
                }
            }
            ExportPresetMessage::SaveAsPreset => {
                // This should be handled by the parent to get current config
                None
            }
            ExportPresetMessage::DeletePreset(id) => {
                // This returns a future, parent should handle async delete
                if !self.preset_manager.is_built_in_preset(&id) {
                    // Note: Actual deletion should be done by parent with async
                    None
                } else {
                    None
                }
            }
            ExportPresetMessage::PresetNameChanged(name) => {
                self.editing_name = name;
                None
            }
            ExportPresetMessage::ConfirmPresetNameEdit => {
                // Parent should handle the actual rename
                self.editing_preset_id = None;
                None
            }
            ExportPresetMessage::CancelPresetNameEdit => {
                self.editing_preset_id = None;
                self.editing_name.clear();
                None
            }
        }
    }

    /// Start editing a preset name
    pub fn start_editing_name(&mut self, preset_id: &str) {
        if let Some(preset) = self.preset_manager.get_preset(preset_id) {
            if !preset.is_built_in {
                self.editing_preset_id = Some(preset_id.to_string());
                self.editing_name = preset.name.clone();
            }
        }
    }

    /// Check if currently editing a preset name
    pub fn is_editing_name(&self) -> bool {
        self.editing_preset_id.is_some()
    }

    /// Get the preset manager for async operations
    pub fn preset_manager(&self) -> &PresetManager {
        &self.preset_manager
    }

    /// Get mutable preset manager for async operations
    pub fn preset_manager_mut(&mut self) -> &mut PresetManager {
        &mut self.preset_manager
    }

    /// Set the selected preset by ID
    pub fn set_selected_preset(&mut self, preset_id: String) {
        self.selected_preset_id = Some(preset_id);
    }

    /// Build the preset selector view
    pub fn view(&self) -> Element<'_, ExportPresetMessage> {
        let label = text("Export Preset:")
            .size(14)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.7, 0.7, 0.7,
            )));

        // Get all preset options
        let options = self.get_all_options();
        let selected = self.get_selected_option();

        // Preset dropdown - filter out separator from selectable options
        let selectable_options: Vec<_> = options
            .into_iter()
            .filter(|opt| opt.id != "__separator__")
            .collect();

        let preset_picker = pick_list(
            selectable_options,
            selected,
            ExportPresetMessage::PresetSelected,
        )
        .width(Length::Fixed(280.0));

        // Action buttons row
        let save_button = button(
            text("Save as Preset")
                .size(12)
                .horizontal_alignment(iced::alignment::Horizontal::Center),
        )
        .padding([6, 12])
        .style(iced::theme::Button::Primary)
        .on_press(ExportPresetMessage::SaveAsPreset);

        // Delete button (only for custom presets)
        let delete_button = if let Some(preset) = self.selected_preset() {
            if !preset.is_built_in {
                Some(
                    button(
                        text("Delete")
                            .size(12)
                            .horizontal_alignment(iced::alignment::Horizontal::Center),
                    )
                    .padding([6, 12])
                    .style(iced::theme::Button::Destructive)
                    .on_press(ExportPresetMessage::DeletePreset(preset.id.clone())),
                )
            } else {
                None
            }
        } else {
            None
        };

        let mut buttons_row = row![save_button];
        if let Some(delete) = delete_button {
            buttons_row = buttons_row.push(Space::with_width(8)).push(delete);
        }

        // Edit name UI (if editing)
        let edit_ui = if let Some(editing_id) = &self.editing_preset_id {
            if self.preset_manager.get_preset(editing_id).is_some() {
                let name_input = text_input("Preset name", &self.editing_name)
                    .width(Length::Fixed(180.0))
                    .on_input(ExportPresetMessage::PresetNameChanged);

                let confirm_btn = button(text("✓").size(12))
                    .padding([6, 10])
                    .style(iced::theme::Button::Positive)
                    .on_press(ExportPresetMessage::ConfirmPresetNameEdit);

                let cancel_btn = button(text("✕").size(12))
                    .padding([6, 10])
                    .style(iced::theme::Button::Destructive)
                    .on_press(ExportPresetMessage::CancelPresetNameEdit);

                Some(
                    row![
                        name_input,
                        Space::with_width(8),
                        confirm_btn,
                        Space::with_width(4),
                        cancel_btn
                    ]
                    .align_items(Alignment::Center),
                )
            } else {
                None
            }
        } else {
            None
        };

        // Legend
        let legend = text("★ = Built-in preset")
            .size(11)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.5, 0.5, 0.5,
            )));

        let mut content = column![label, Space::with_height(4), preset_picker,];

        if let Some(edit_ui) = edit_ui {
            content = content.push(Space::with_height(8)).push(edit_ui);
        }

        content = content
            .push(Space::with_height(8))
            .push(buttons_row)
            .push(Space::with_height(4))
            .push(legend);

        content.spacing(4).into()
    }
}

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportFormat {
    /// MP4 with H.264
    #[default]
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

/// Quality preset for export
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QualityPreset {
    /// Low quality, small file
    Low,
    /// Medium quality
    Medium,
    /// High quality
    #[default]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportResolution {
    /// Original resolution
    #[default]
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

/// Aspect ratio preset options for the picker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AspectRatioPreset {
    /// Use original recording aspect ratio
    #[default]
    Original,
    /// 16:9 landscape (HD video)
    Horizontal16x9,
    /// 9:16 portrait (mobile/TikTok)
    Vertical9x16,
    /// 1:1 square (Instagram)
    Square,
    /// 4:3 standard (classic displays)
    Standard4x3,
    /// 21:9 ultrawide (cinemascope)
    Ultrawide21x9,
    /// Custom ratio with user-defined dimensions
    Custom,
}

impl std::fmt::Display for AspectRatioPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AspectRatioPreset::Original => write!(f, "Original"),
            AspectRatioPreset::Horizontal16x9 => write!(f, "16:9 (Landscape)"),
            AspectRatioPreset::Vertical9x16 => write!(f, "9:16 (Portrait)"),
            AspectRatioPreset::Square => write!(f, "1:1 (Square)"),
            AspectRatioPreset::Standard4x3 => write!(f, "4:3 (Standard)"),
            AspectRatioPreset::Ultrawide21x9 => write!(f, "21:9 (Ultrawide)"),
            AspectRatioPreset::Custom => write!(f, "Custom..."),
        }
    }
}

impl From<AspectRatio> for AspectRatioPreset {
    fn from(ratio: AspectRatio) -> Self {
        match ratio {
            AspectRatio::Horizontal16x9 => AspectRatioPreset::Horizontal16x9,
            AspectRatio::Vertical9x16 => AspectRatioPreset::Vertical9x16,
            AspectRatio::Square => AspectRatioPreset::Square,
            AspectRatio::Standard4x3 => AspectRatioPreset::Standard4x3,
            AspectRatio::Ultrawide21x9 => AspectRatioPreset::Ultrawide21x9,
            AspectRatio::Cinema185 => AspectRatioPreset::Horizontal16x9, // Close enough fallback
            AspectRatio::Cinema239 => AspectRatioPreset::Ultrawide21x9,  // Close enough fallback
            AspectRatio::Custom(_, _) => AspectRatioPreset::Custom,
        }
    }
}

impl From<AspectRatioPreset> for AspectRatio {
    fn from(preset: AspectRatioPreset) -> Self {
        match preset {
            AspectRatioPreset::Original => AspectRatio::default(), // Will be replaced with actual original
            AspectRatioPreset::Horizontal16x9 => AspectRatio::Horizontal16x9,
            AspectRatioPreset::Vertical9x16 => AspectRatio::Vertical9x16,
            AspectRatioPreset::Square => AspectRatio::Square,
            AspectRatioPreset::Standard4x3 => AspectRatio::Standard4x3,
            AspectRatioPreset::Ultrawide21x9 => AspectRatio::Ultrawide21x9,
            AspectRatioPreset::Custom => AspectRatio::Custom(16, 9), // Default custom values
        }
    }
}

/// Messages from the aspect ratio selector
#[derive(Debug, Clone)]
pub enum AspectRatioMessage {
    /// User selected a preset from the dropdown
    PresetChanged(AspectRatioPreset),
    /// User changed the custom width input
    CustomWidthChanged(String),
    /// User changed the custom height input
    CustomHeightChanged(String),
}

/// Aspect ratio selector widget with preview
///
/// Provides a dropdown for preset ratios and visual preview.
/// Remembers the last used ratio via stored config.
pub struct AspectRatioSelector {
    /// The currently selected preset
    preset: AspectRatioPreset,
    /// The actual aspect ratio (either from preset or custom)
    aspect_ratio: AspectRatio,
    /// Custom width input value (as string for text input)
    custom_width: String,
    /// Custom height input value (as string for text input)
    custom_height: String,
    /// Original aspect ratio from source recording (for "Original" preset)
    original_ratio: Option<AspectRatio>,
}

impl Default for AspectRatioSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl AspectRatioSelector {
    /// Create a new aspect ratio selector with default values
    pub fn new() -> Self {
        Self {
            preset: AspectRatioPreset::Original,
            aspect_ratio: AspectRatio::default(),
            custom_width: "16".to_string(),
            custom_height: "9".to_string(),
            original_ratio: None,
        }
    }

    /// Create with a specific aspect ratio
    pub fn with_aspect_ratio(ratio: AspectRatio) -> Self {
        let preset = AspectRatioPreset::from(ratio);
        let (custom_width, custom_height) = match ratio {
            AspectRatio::Custom(w, h) => (w.to_string(), h.to_string()),
            _ => ("16".to_string(), "9".to_string()),
        };

        Self {
            preset,
            aspect_ratio: ratio,
            custom_width,
            custom_height,
            original_ratio: None,
        }
    }

    /// Set the original recording aspect ratio (for "Original" preset)
    pub fn set_original_ratio(&mut self, ratio: AspectRatio) {
        self.original_ratio = Some(ratio);
        // If currently set to Original, update the aspect_ratio
        if self.preset == AspectRatioPreset::Original {
            self.aspect_ratio = ratio;
        }
    }

    /// Get the current aspect ratio
    pub fn aspect_ratio(&self) -> AspectRatio {
        self.aspect_ratio
    }

    /// Get the current preset
    pub fn preset(&self) -> AspectRatioPreset {
        self.preset
    }

    /// Handle aspect ratio messages
    pub fn update(&mut self, message: AspectRatioMessage) {
        match message {
            AspectRatioMessage::PresetChanged(preset) => {
                self.preset = preset;
                self.aspect_ratio = match preset {
                    AspectRatioPreset::Original => self.original_ratio.unwrap_or_default(),
                    AspectRatioPreset::Custom => {
                        // Parse current custom values
                        let w = self.custom_width.parse().unwrap_or(16);
                        let h = self.custom_height.parse().unwrap_or(9);
                        AspectRatio::Custom(w, h)
                    }
                    _ => AspectRatio::from(preset),
                };
            }
            AspectRatioMessage::CustomWidthChanged(value) => {
                self.custom_width = value;
                // Update aspect ratio if in custom mode
                if self.preset == AspectRatioPreset::Custom {
                    if let (Ok(w), Ok(h)) = (
                        self.custom_width.parse::<u32>(),
                        self.custom_height.parse::<u32>(),
                    ) {
                        if h > 0 {
                            self.aspect_ratio = AspectRatio::Custom(w, h);
                        }
                    }
                }
            }
            AspectRatioMessage::CustomHeightChanged(value) => {
                self.custom_height = value;
                // Update aspect ratio if in custom mode
                if self.preset == AspectRatioPreset::Custom {
                    if let (Ok(w), Ok(h)) = (
                        self.custom_width.parse::<u32>(),
                        self.custom_height.parse::<u32>(),
                    ) {
                        if h > 0 {
                            self.aspect_ratio = AspectRatio::Custom(w, h);
                        }
                    }
                }
            }
        }
    }

    /// Build the aspect ratio selector view
    pub fn view(&self) -> Element<'_, AspectRatioMessage> {
        let label = text("Aspect Ratio:")
            .size(14)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.7, 0.7, 0.7,
            )));

        // Preset dropdown
        let preset_options = vec![
            AspectRatioPreset::Original,
            AspectRatioPreset::Horizontal16x9,
            AspectRatioPreset::Vertical9x16,
            AspectRatioPreset::Square,
            AspectRatioPreset::Standard4x3,
            AspectRatioPreset::Ultrawide21x9,
            AspectRatioPreset::Custom,
        ];

        let preset_picker = pick_list(
            preset_options,
            Some(self.preset),
            AspectRatioMessage::PresetChanged,
        )
        .width(Length::Fixed(200.0));

        // Custom input fields (only show when Custom preset is selected)
        let custom_inputs = if self.preset == AspectRatioPreset::Custom {
            let width_input = text_input("Width", &self.custom_width)
                .width(Length::Fixed(80.0))
                .on_input(AspectRatioMessage::CustomWidthChanged);

            let height_input = text_input("Height", &self.custom_height)
                .width(Length::Fixed(80.0))
                .on_input(AspectRatioMessage::CustomHeightChanged);

            let colon = text(":").size(16);

            Some(
                row![
                    width_input,
                    Space::with_width(8),
                    colon,
                    Space::with_width(8),
                    height_input
                ]
                .align_items(Alignment::Center),
            )
        } else {
            None
        };

        // Visual preview box
        let preview = self.preview_box();

        // Ratio display text
        let ratio_text = self.format_ratio_text();
        let ratio_label =
            text(ratio_text)
                .size(12)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.6, 0.6, 0.6,
                )));

        let mut content = column![label, Space::with_height(4), preset_picker,];

        if let Some(inputs) = custom_inputs {
            content = content.push(Space::with_height(8)).push(inputs);
        }

        content = content
            .push(Space::with_height(12))
            .push(preview)
            .push(Space::with_height(4))
            .push(ratio_label);

        content.spacing(4).into()
    }

    /// Create a visual preview box showing the aspect ratio
    fn preview_box(&self) -> Element<'_, AspectRatioMessage> {
        let ratio = self.aspect_ratio.ratio_f32();
        let max_preview_width = 200.0;
        let max_preview_height = 120.0;

        // Calculate preview dimensions that fit within max bounds while maintaining ratio
        let (preview_width, preview_height) = if ratio > max_preview_width / max_preview_height {
            // Wider than container - fit to width
            (max_preview_width, max_preview_width / ratio)
        } else {
            // Taller or square - fit to height
            (max_preview_height * ratio, max_preview_height)
        };

        // Create a styled container as the preview box
        let preview = container(text(""))
            .width(Length::Fixed(preview_width))
            .height(Length::Fixed(preview_height))
            .style(iced::theme::Container::Box);

        // Wrap in a centered container
        container(preview)
            .width(Length::Fixed(max_preview_width))
            .height(Length::Fixed(max_preview_height))
            .center_x()
            .center_y()
            .into()
    }

    /// Format the ratio as a display string
    fn format_ratio_text(&self) -> String {
        match self.aspect_ratio {
            AspectRatio::Horizontal16x9 => "16:9 (1.78)".to_string(),
            AspectRatio::Vertical9x16 => "9:16 (0.56)".to_string(),
            AspectRatio::Square => "1:1 (1.00)".to_string(),
            AspectRatio::Standard4x3 => "4:3 (1.33)".to_string(),
            AspectRatio::Ultrawide21x9 => "21:9 (2.33)".to_string(),
            AspectRatio::Cinema185 => "1.85:1 (1.85)".to_string(),
            AspectRatio::Cinema239 => "2.39:1 (2.39)".to_string(),
            AspectRatio::Custom(w, h) => {
                let ratio = w as f32 / h as f32;
                format!("{}:{} ({:.2})", w, h, ratio)
            }
        }
    }
}

/// Export configuration
#[derive(Debug, Clone)]
pub struct ExportConfig {
    pub format: ExportFormat,
    pub quality: QualityPreset,
    pub resolution: ExportResolution,
    pub aspect_ratio: AspectRatio,
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
            aspect_ratio: AspectRatio::default(),
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
    AspectRatioChanged(AspectRatioMessage),
    DestinationChanged(PathBuf),
    FilenameChanged(String),
    IncludeAudioChanged(bool),
    FpsChanged(u32),
    PresetChanged(ExportPresetMessage),
    StartExport,
    CancelExport,
}

/// Export dialog component
pub struct ExportDialog {
    pub config: ExportConfig,
    pub aspect_ratio_selector: AspectRatioSelector,
    pub preset_selector: ExportPresetSelector,
    pub is_open: bool,
}

impl ExportDialog {
    pub fn new() -> Self {
        let config = ExportConfig::default();
        let aspect_ratio_selector = AspectRatioSelector::with_aspect_ratio(config.aspect_ratio);
        let preset_selector = ExportPresetSelector::new();

        Self {
            config,
            aspect_ratio_selector,
            preset_selector,
            is_open: false,
        }
    }

    /// Create with existing config
    pub fn with_config(config: ExportConfig) -> Self {
        let aspect_ratio_selector = AspectRatioSelector::with_aspect_ratio(config.aspect_ratio);
        let preset_selector = ExportPresetSelector::new();

        Self {
            config,
            aspect_ratio_selector,
            preset_selector,
            is_open: false,
        }
    }

    /// Set the original recording aspect ratio
    pub fn set_original_aspect_ratio(&mut self, ratio: AspectRatio) {
        self.aspect_ratio_selector.set_original_ratio(ratio);
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
            ExportDialogMessage::AspectRatioChanged(msg) => {
                self.aspect_ratio_selector.update(msg);
                self.config.aspect_ratio = self.aspect_ratio_selector.aspect_ratio();
            }
            ExportDialogMessage::DestinationChanged(dest) => self.config.destination = dest,
            ExportDialogMessage::FilenameChanged(name) => self.config.filename = name,
            ExportDialogMessage::IncludeAudioChanged(include) => {
                self.config.include_audio = include
            }
            ExportDialogMessage::FpsChanged(fps) => self.config.fps = fps,
            ExportDialogMessage::PresetChanged(msg) => {
                if let Some(_preset) = self.preset_selector.update(msg) {
                    // Apply preset settings to config
                    self.preset_selector
                        .apply_preset_to_config(&mut self.config);
                }
            }
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

        // Preset selector
        let on_message_clone = on_message.clone();
        let preset_view = self
            .preset_selector
            .view()
            .map(move |msg| on_message_clone(ExportDialogMessage::PresetChanged(msg)));

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

        // Aspect ratio selector
        let on_message_clone = on_message.clone();
        let aspect_ratio_view = self
            .aspect_ratio_selector
            .view()
            .map(move |msg| on_message_clone(ExportDialogMessage::AspectRatioChanged(msg)));

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
            preset_view,
            Space::with_height(16),
            format_label,
            format_pick_list,
            Space::with_height(12),
            quality_label,
            quality_pick_list,
            Space::with_height(12),
            resolution_label,
            resolution_pick_list,
            Space::with_height(16),
            aspect_ratio_view,
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
