//! Webcam settings UI for configuring webcam overlay
//!
//! Provides UI for camera selection, position, shape, size, and styling.

use frame_core::effects::{
    Color as CoreColor, WebcamOverlayConfig, WebcamPosition, WebcamShape, WebcamSize,
};
use iced::widget::{button, checkbox, column, container, pick_list, row, slider, text, Space};
use iced::{Alignment, Element, Length, Theme};

/// Messages from the webcam settings panel
#[derive(Debug, Clone)]
pub enum WebcamSettingsMessage {
    /// Enable/disable webcam overlay
    EnabledChanged(bool),
    /// Camera device selection changed
    CameraDeviceChanged(String),
    /// Position selection changed
    PositionChanged(WebcamPosition),
    /// Shape selection changed
    ShapeChanged(WebcamShape),
    /// Size selection changed
    SizeChanged(WebcamSize),
    /// Border color selection changed
    BorderColorChanged(ColorOption),
    /// Border width changed
    BorderWidthChanged(f32),
    /// Confirm and apply settings
    Confirm,
    /// Cancel and discard changes
    Cancel,
}

/// Position options for UI display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebcamPositionOption {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl std::fmt::Display for WebcamPositionOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebcamPositionOption::TopLeft => write!(f, "Top Left"),
            WebcamPositionOption::TopRight => write!(f, "Top Right"),
            WebcamPositionOption::BottomLeft => write!(f, "Bottom Left"),
            WebcamPositionOption::BottomRight => write!(f, "Bottom Right"),
        }
    }
}

impl From<WebcamPosition> for WebcamPositionOption {
    fn from(pos: WebcamPosition) -> Self {
        match pos {
            WebcamPosition::TopLeft => WebcamPositionOption::TopLeft,
            WebcamPosition::TopRight => WebcamPositionOption::TopRight,
            WebcamPosition::BottomLeft => WebcamPositionOption::BottomLeft,
            WebcamPosition::BottomRight => WebcamPositionOption::BottomRight,
        }
    }
}

impl From<WebcamPositionOption> for WebcamPosition {
    fn from(pos: WebcamPositionOption) -> Self {
        match pos {
            WebcamPositionOption::TopLeft => WebcamPosition::TopLeft,
            WebcamPositionOption::TopRight => WebcamPosition::TopRight,
            WebcamPositionOption::BottomLeft => WebcamPosition::BottomLeft,
            WebcamPositionOption::BottomRight => WebcamPosition::BottomRight,
        }
    }
}

/// Shape options for UI display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeOption {
    Circle,
    RoundedRect,
}

impl std::fmt::Display for ShapeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShapeOption::Circle => write!(f, "Circle"),
            ShapeOption::RoundedRect => write!(f, "Rounded Rectangle"),
        }
    }
}

impl From<WebcamShape> for ShapeOption {
    fn from(shape: WebcamShape) -> Self {
        match shape {
            WebcamShape::Circle => ShapeOption::Circle,
            WebcamShape::RoundedRect | WebcamShape::Rectangle => ShapeOption::RoundedRect,
        }
    }
}

impl From<ShapeOption> for WebcamShape {
    fn from(shape: ShapeOption) -> Self {
        match shape {
            ShapeOption::Circle => WebcamShape::Circle,
            ShapeOption::RoundedRect => WebcamShape::RoundedRect,
        }
    }
}

/// Size options for UI display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeOption {
    Small,
    Medium,
    Large,
}

impl std::fmt::Display for SizeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SizeOption::Small => write!(f, "Small"),
            SizeOption::Medium => write!(f, "Medium"),
            SizeOption::Large => write!(f, "Large"),
        }
    }
}

impl From<WebcamSize> for SizeOption {
    fn from(size: WebcamSize) -> Self {
        match size {
            WebcamSize::Small => SizeOption::Small,
            WebcamSize::Medium => SizeOption::Medium,
            WebcamSize::Large => SizeOption::Large,
            WebcamSize::Custom(_) => SizeOption::Medium, // Default to medium for custom
        }
    }
}

impl From<SizeOption> for WebcamSize {
    fn from(size: SizeOption) -> Self {
        match size {
            SizeOption::Small => WebcamSize::Small,
            SizeOption::Medium => WebcamSize::Medium,
            SizeOption::Large => WebcamSize::Large,
        }
    }
}

/// Color preset options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorOption {
    White,
    Black,
    Red,
    Green,
    Blue,
    Yellow,
    Cyan,
    Magenta,
}

impl ColorOption {
    /// Convert to frame-core Color
    pub fn to_core_color(&self) -> CoreColor {
        match self {
            ColorOption::White => CoreColor::WHITE,
            ColorOption::Black => CoreColor::BLACK,
            ColorOption::Red => CoreColor::rgb(1.0, 0.0, 0.0),
            ColorOption::Green => CoreColor::rgb(0.0, 1.0, 0.0),
            ColorOption::Blue => CoreColor::rgb(0.0, 0.0, 1.0),
            ColorOption::Yellow => CoreColor::rgb(1.0, 1.0, 0.0),
            ColorOption::Cyan => CoreColor::rgb(0.0, 1.0, 1.0),
            ColorOption::Magenta => CoreColor::rgb(1.0, 0.0, 1.0),
        }
    }

    /// Get display color for the button
    pub fn to_iced_color(&self) -> iced::Color {
        match self {
            ColorOption::White => iced::Color::WHITE,
            ColorOption::Black => iced::Color::BLACK,
            ColorOption::Red => iced::Color::from_rgb(1.0, 0.0, 0.0),
            ColorOption::Green => iced::Color::from_rgb(0.0, 1.0, 0.0),
            ColorOption::Blue => iced::Color::from_rgb(0.0, 0.0, 1.0),
            ColorOption::Yellow => iced::Color::from_rgb(1.0, 1.0, 0.0),
            ColorOption::Cyan => iced::Color::from_rgb(0.0, 1.0, 1.0),
            ColorOption::Magenta => iced::Color::from_rgb(1.0, 0.0, 1.0),
        }
    }
}

impl From<CoreColor> for ColorOption {
    fn from(color: CoreColor) -> Self {
        // Compare with known colors (with some tolerance for float comparison)
        const EPSILON: f32 = 0.01;

        if (color.r - 1.0).abs() < EPSILON
            && (color.g - 1.0).abs() < EPSILON
            && (color.b - 1.0).abs() < EPSILON
        {
            ColorOption::White
        } else if (color.r).abs() < EPSILON
            && (color.g).abs() < EPSILON
            && (color.b).abs() < EPSILON
        {
            ColorOption::Black
        } else if (color.r - 1.0).abs() < EPSILON
            && (color.g).abs() < EPSILON
            && (color.b).abs() < EPSILON
        {
            ColorOption::Red
        } else if (color.r).abs() < EPSILON
            && (color.g - 1.0).abs() < EPSILON
            && (color.b).abs() < EPSILON
        {
            ColorOption::Green
        } else if (color.r).abs() < EPSILON
            && (color.g).abs() < EPSILON
            && (color.b - 1.0).abs() < EPSILON
        {
            ColorOption::Blue
        } else if (color.r - 1.0).abs() < EPSILON
            && (color.g - 1.0).abs() < EPSILON
            && (color.b).abs() < EPSILON
        {
            ColorOption::Yellow
        } else if (color.r).abs() < EPSILON
            && (color.g - 1.0).abs() < EPSILON
            && (color.b - 1.0).abs() < EPSILON
        {
            ColorOption::Cyan
        } else if (color.r - 1.0).abs() < EPSILON
            && (color.g).abs() < EPSILON
            && (color.b - 1.0).abs() < EPSILON
        {
            ColorOption::Magenta
        } else {
            ColorOption::White // Default
        }
    }
}

/// Webcam settings widget
pub struct WebcamSettings {
    config: WebcamOverlayConfig,
    /// List of available camera devices (names as strings)
    pub available_cameras: Vec<String>,
    /// Currently selected camera device
    pub selected_camera: Option<String>,
}

impl Default for WebcamSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl WebcamSettings {
    /// Create a new webcam settings panel with default config
    pub fn new() -> Self {
        Self {
            config: WebcamOverlayConfig::default(),
            available_cameras: Vec::new(),
            selected_camera: None,
        }
    }

    /// Create with existing config
    pub fn with_config(config: WebcamOverlayConfig) -> Self {
        Self {
            config,
            available_cameras: Vec::new(),
            selected_camera: None,
        }
    }

    /// Set the list of available cameras
    pub fn set_available_cameras(&mut self, cameras: Vec<String>) {
        self.available_cameras = cameras;
        // Set default selection if none exists and cameras are available
        if self.selected_camera.is_none() && !self.available_cameras.is_empty() {
            self.selected_camera = Some(self.available_cameras[0].clone());
        }
    }

    /// Get the current config
    pub fn config(&self) -> &WebcamOverlayConfig {
        &self.config
    }

    /// Set the config
    pub fn set_config(&mut self, config: WebcamOverlayConfig) {
        self.config = config;
    }

    /// Get the selected camera
    pub fn selected_camera(&self) -> Option<&String> {
        self.selected_camera.as_ref()
    }

    /// Handle a settings message, returns true if Confirm was pressed
    pub fn update(&mut self, message: WebcamSettingsMessage) -> bool {
        match message {
            WebcamSettingsMessage::EnabledChanged(enabled) => {
                self.config.enabled = enabled;
            }
            WebcamSettingsMessage::CameraDeviceChanged(device) => {
                self.selected_camera = Some(device);
            }
            WebcamSettingsMessage::PositionChanged(position) => {
                self.config.position = position;
            }
            WebcamSettingsMessage::ShapeChanged(shape) => {
                self.config.shape = shape;
            }
            WebcamSettingsMessage::SizeChanged(size) => {
                self.config.size = size;
            }
            WebcamSettingsMessage::BorderColorChanged(color) => {
                self.config.border_color = color.to_core_color();
            }
            WebcamSettingsMessage::BorderWidthChanged(width) => {
                self.config.border_width = width as u32;
            }
            WebcamSettingsMessage::Confirm => {
                return true;
            }
            WebcamSettingsMessage::Cancel => {
                // Reset to default config on cancel
                self.config = WebcamOverlayConfig::default();
                return false;
            }
        }
        false
    }

    /// Build the view
    pub fn view(&self) -> Element<'_, WebcamSettingsMessage> {
        let title = text("Webcam Settings")
            .size(24)
            .style(iced::theme::Text::Color(iced::Color::WHITE));

        // Enable/disable toggle
        let enabled = checkbox("Enable webcam overlay", self.config.enabled)
            .on_toggle(WebcamSettingsMessage::EnabledChanged);

        // Camera device dropdown
        let camera_section = self.camera_section();

        // Preview placeholder
        let preview_section = self.preview_section();

        // Position selector
        let position_section = self.position_section();

        // Shape toggle
        let shape_section = self.shape_section();

        // Size slider
        let size_section = self.size_section();

        // Border color picker
        let color_section = self.color_section();

        // Border width slider
        let border_width_section = self.border_width_section();

        // Action buttons
        let action_row = row![
            button("Cancel")
                .on_press(WebcamSettingsMessage::Cancel)
                .style(iced::theme::Button::Secondary),
            Space::with_width(10),
            button("Confirm")
                .on_press(WebcamSettingsMessage::Confirm)
                .style(iced::theme::Button::Primary),
        ]
        .spacing(10);

        let content = column![
            title,
            Space::with_height(20),
            enabled,
            Space::with_height(15),
            camera_section,
            Space::with_height(15),
            preview_section,
            Space::with_height(15),
            position_section,
            Space::with_height(15),
            shape_section,
            Space::with_height(15),
            size_section,
            Space::with_height(15),
            color_section,
            Space::with_height(15),
            border_width_section,
            Space::with_height(20),
            action_row,
        ]
        .spacing(10)
        .padding(20)
        .width(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn camera_section(&self) -> Element<'_, WebcamSettingsMessage> {
        let header = text("Camera Device")
            .size(18)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.8, 0.8, 0.8,
            )));

        if self.available_cameras.is_empty() {
            let no_cameras = text("No cameras detected")
                .size(14)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.6, 0.6, 0.6,
                )));

            return column![header, Space::with_height(5), no_cameras]
                .spacing(5)
                .into();
        }

        let camera_dropdown = pick_list(
            self.available_cameras.clone(),
            self.selected_camera.clone(),
            WebcamSettingsMessage::CameraDeviceChanged,
        )
        .width(Length::Fill);

        column![header, Space::with_height(5), camera_dropdown]
            .spacing(5)
            .into()
    }

    fn preview_section(&self) -> Element<'_, WebcamSettingsMessage> {
        let header =
            text("Preview")
                .size(18)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.8, 0.8, 0.8,
                )));

        // Preview placeholder - rectangle showing where webcam would appear
        let preview_size = self.config.size.dimension() as f32;
        let preview = container(
            text("Camera Preview")
                .size(12)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.5, 0.5, 0.5,
                ))),
        )
        .width(Length::Fixed(preview_size))
        .height(Length::Fixed(preview_size * 0.75)) // 4:3 aspect ratio
        .style(iced::theme::Container::Box);

        column![header, Space::with_height(5), preview]
            .spacing(5)
            .into()
    }

    fn position_section(&self) -> Element<'_, WebcamSettingsMessage> {
        let header =
            text("Position")
                .size(18)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.8, 0.8, 0.8,
                )));

        // 4 corner position buttons
        let tl_button = self.position_button("TL", WebcamPosition::TopLeft);
        let tr_button = self.position_button("TR", WebcamPosition::TopRight);
        let bl_button = self.position_button("BL", WebcamPosition::BottomLeft);
        let br_button = self.position_button("BR", WebcamPosition::BottomRight);

        let top_row = row![tl_button, Space::with_width(10), tr_button].spacing(5);
        let bottom_row = row![bl_button, Space::with_width(10), br_button].spacing(5);

        column![header, Space::with_height(5), top_row, bottom_row]
            .spacing(5)
            .into()
    }

    fn position_button(
        &self,
        label: &str,
        position: WebcamPosition,
    ) -> Element<'_, WebcamSettingsMessage> {
        let is_selected = self.config.position == position;

        button(text(label).size(14))
            .on_press(WebcamSettingsMessage::PositionChanged(position))
            .style(if is_selected {
                iced::theme::Button::Primary
            } else {
                iced::theme::Button::Secondary
            })
            .into()
    }

    fn shape_section(&self) -> Element<'_, WebcamSettingsMessage> {
        let header = text("Shape")
            .size(18)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.8, 0.8, 0.8,
            )));

        let shape_label =
            text("Shape:")
                .size(14)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.7, 0.7, 0.7,
                )));

        let shape_options = vec![ShapeOption::Circle, ShapeOption::RoundedRect];
        let shape_dropdown = pick_list(
            shape_options,
            Some(ShapeOption::from(self.config.shape)),
            |opt| WebcamSettingsMessage::ShapeChanged(opt.into()),
        );

        let shape_row =
            row![shape_label, Space::with_width(10), shape_dropdown].align_items(Alignment::Center);

        column![header, Space::with_height(5), shape_row]
            .spacing(5)
            .into()
    }

    fn size_section(&self) -> Element<'_, WebcamSettingsMessage> {
        let header = text("Size")
            .size(18)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.8, 0.8, 0.8,
            )));

        let size_label =
            text("Size:")
                .size(14)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.7, 0.7, 0.7,
                )));

        let size_options = vec![SizeOption::Small, SizeOption::Medium, SizeOption::Large];
        let size_dropdown = pick_list(
            size_options,
            Some(SizeOption::from(self.config.size)),
            |opt| WebcamSettingsMessage::SizeChanged(opt.into()),
        );

        let size_row =
            row![size_label, Space::with_width(10), size_dropdown].align_items(Alignment::Center);

        column![header, Space::with_height(5), size_row]
            .spacing(5)
            .into()
    }

    fn color_section(&self) -> Element<'_, WebcamSettingsMessage> {
        let header = text("Border Color")
            .size(18)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.8, 0.8, 0.8,
            )));

        let color_options = vec![
            ColorOption::White,
            ColorOption::Black,
            ColorOption::Red,
            ColorOption::Green,
            ColorOption::Blue,
            ColorOption::Yellow,
            ColorOption::Cyan,
            ColorOption::Magenta,
        ];

        let current_color = ColorOption::from(self.config.border_color);

        let mut color_row = row![];
        for color in color_options {
            let is_selected = current_color == color;
            let color_button = button(Space::with_width(20))
                .on_press(WebcamSettingsMessage::BorderColorChanged(color))
                .style(iced::theme::Button::Custom(Box::new(ColorButtonStyle {
                    color: color.to_iced_color(),
                    is_selected,
                })));
            color_row = color_row.push(color_button).push(Space::with_width(5));
        }

        column![header, Space::with_height(5), color_row]
            .spacing(5)
            .into()
    }

    fn border_width_section(&self) -> Element<'_, WebcamSettingsMessage> {
        let width_label = text(format!("Border width: {}px", self.config.border_width))
            .size(14)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.7, 0.7, 0.7,
            )));

        let width_slider = slider(0.0..=10.0, self.config.border_width as f32, |v| {
            WebcamSettingsMessage::BorderWidthChanged(v)
        })
        .step(1.0);

        column![width_label, width_slider].spacing(5).into()
    }

    /// Get the current config (convenience method for confirm action)
    pub fn get_config(&self) -> WebcamOverlayConfig {
        self.config.clone()
    }
}

/// Custom button style for color picker buttons
struct ColorButtonStyle {
    color: iced::Color,
    is_selected: bool,
}

impl iced::widget::button::StyleSheet for ColorButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: Some(iced::Background::Color(self.color)),
            border: iced::Border {
                color: if self.is_selected {
                    iced::Color::from_rgb(0.5, 0.8, 1.0) // Highlight color for selected
                } else {
                    iced::Color::from_rgb(0.3, 0.3, 0.3)
                },
                width: if self.is_selected { 3.0 } else { 1.0 },
                radius: 4.0.into(),
            },
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let mut active = self.active(style);
        active.border.width = 2.0;
        active.border.color = iced::Color::from_rgb(0.7, 0.7, 0.7);
        active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webcam_settings_update() {
        let mut settings = WebcamSettings::new();

        // Test enabled toggle
        settings.update(WebcamSettingsMessage::EnabledChanged(false));
        assert!(!settings.config().enabled);

        // Test position change
        settings.update(WebcamSettingsMessage::PositionChanged(
            WebcamPosition::TopLeft,
        ));
        assert_eq!(settings.config().position, WebcamPosition::TopLeft);

        // Test shape change
        settings.update(WebcamSettingsMessage::ShapeChanged(WebcamShape::Circle));
        assert_eq!(settings.config().shape, WebcamShape::Circle);

        // Test size change
        settings.update(WebcamSettingsMessage::SizeChanged(WebcamSize::Large));
        assert_eq!(settings.config().size, WebcamSize::Large);

        // Test border width
        settings.update(WebcamSettingsMessage::BorderWidthChanged(5.0));
        assert_eq!(settings.config().border_width, 5);
    }

    #[test]
    fn test_webcam_settings_confirm() {
        let mut settings = WebcamSettings::new();

        // Modify some settings
        settings.update(WebcamSettingsMessage::EnabledChanged(false));
        settings.update(WebcamSettingsMessage::PositionChanged(
            WebcamPosition::TopRight,
        ));

        // Confirm should return true
        assert!(settings.update(WebcamSettingsMessage::Confirm));

        // Config should remain as modified
        assert!(!settings.config().enabled);
        assert_eq!(settings.config().position, WebcamPosition::TopRight);
    }

    #[test]
    fn test_webcam_settings_cancel() {
        let mut settings = WebcamSettings::new();
        let _original_config = settings.config().clone();

        // Modify some settings
        settings.update(WebcamSettingsMessage::EnabledChanged(false));
        settings.update(WebcamSettingsMessage::PositionChanged(
            WebcamPosition::TopRight,
        ));

        // Cancel should reset to defaults
        settings.update(WebcamSettingsMessage::Cancel);

        // Config should be reset to defaults
        assert!(settings.config().enabled); // Default is true
        assert_eq!(settings.config().position, WebcamPosition::BottomRight); // Default
    }

    #[test]
    fn test_color_option_conversions() {
        // Test color round-trip
        let original = CoreColor::rgb(1.0, 0.0, 0.0);
        let option = ColorOption::from(original);
        assert!(matches!(option, ColorOption::Red));

        // Test white
        let white = CoreColor::WHITE;
        let white_option = ColorOption::from(white);
        assert!(matches!(white_option, ColorOption::White));
    }

    #[test]
    fn test_option_conversions() {
        // Position round-trip
        let original = WebcamPosition::TopRight;
        let option = WebcamPositionOption::from(original);
        let back: WebcamPosition = option.into();
        assert_eq!(original, back);

        // Shape round-trip
        let original = WebcamShape::Circle;
        let option = ShapeOption::from(original);
        let back: WebcamShape = option.into();
        assert_eq!(original, back);

        // Size round-trip
        let original = WebcamSize::Medium;
        let option = SizeOption::from(original);
        let back: WebcamSize = option.into();
        assert_eq!(original, back);
    }

    #[test]
    fn test_camera_management() {
        let mut settings = WebcamSettings::new();

        // Initially no cameras
        assert!(settings.available_cameras.is_empty());
        assert!(settings.selected_camera().is_none());

        // Set available cameras
        let cameras = vec!["Built-in Camera".to_string(), "External Webcam".to_string()];
        settings.set_available_cameras(cameras.clone());

        assert_eq!(settings.available_cameras.len(), 2);
        assert_eq!(
            settings.selected_camera(),
            Some(&"Built-in Camera".to_string())
        );

        // Select a different camera
        settings.update(WebcamSettingsMessage::CameraDeviceChanged(
            "External Webcam".to_string(),
        ));
        assert_eq!(
            settings.selected_camera(),
            Some(&"External Webcam".to_string())
        );
    }
}
