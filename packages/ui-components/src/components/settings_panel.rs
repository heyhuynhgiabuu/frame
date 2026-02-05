//! Settings panel for configuring effects
//!
//! Provides UI for configuring zoom, keyboard display, and background effects.

use frame_core::effects::{
    BackgroundStyle, BadgePosition as CoreBadgePosition, Color as CoreColor, EasingFunction,
    EffectsConfig, Padding,
};
use iced::widget::{checkbox, column, container, pick_list, row, slider, text, Space};
use iced::{Alignment, Element, Length};

/// Messages from the settings panel
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    // Zoom settings
    ZoomEnabledChanged(bool),
    ZoomLevelChanged(f32),
    ZoomTransitionChanged(u32),
    ZoomIdleTimeoutChanged(u32),
    ZoomEasingChanged(EasingOption),

    // Keyboard settings
    KeyboardEnabledChanged(bool),
    KeyboardPositionChanged(PositionOption),
    KeyboardFadeChanged(u32),
    KeyboardFontSizeChanged(f32),

    // Background settings
    BackgroundStyleChanged(BackgroundOption),
    BackgroundPaddingChanged(f32),
    BackgroundCornerRadiusChanged(f32),
}

/// Easing options for pick list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasingOption {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOutCubic,
    EaseInOutQuad,
}

impl std::fmt::Display for EasingOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EasingOption::Linear => write!(f, "Linear"),
            EasingOption::EaseIn => write!(f, "Ease In"),
            EasingOption::EaseOut => write!(f, "Ease Out"),
            EasingOption::EaseInOutCubic => write!(f, "Ease In-Out Cubic"),
            EasingOption::EaseInOutQuad => write!(f, "Ease In-Out Quad"),
        }
    }
}

impl From<EasingFunction> for EasingOption {
    fn from(ef: EasingFunction) -> Self {
        match ef {
            EasingFunction::Linear => EasingOption::Linear,
            EasingFunction::EaseIn => EasingOption::EaseIn,
            EasingFunction::EaseOut => EasingOption::EaseOut,
            EasingFunction::EaseInOutCubic => EasingOption::EaseInOutCubic,
            EasingFunction::EaseInOutQuad => EasingOption::EaseInOutQuad,
        }
    }
}

impl From<EasingOption> for EasingFunction {
    fn from(eo: EasingOption) -> Self {
        match eo {
            EasingOption::Linear => EasingFunction::Linear,
            EasingOption::EaseIn => EasingFunction::EaseIn,
            EasingOption::EaseOut => EasingFunction::EaseOut,
            EasingOption::EaseInOutCubic => EasingFunction::EaseInOutCubic,
            EasingOption::EaseInOutQuad => EasingFunction::EaseInOutQuad,
        }
    }
}

/// Position options for pick list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionOption {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

impl std::fmt::Display for PositionOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionOption::TopLeft => write!(f, "Top Left"),
            PositionOption::TopRight => write!(f, "Top Right"),
            PositionOption::BottomLeft => write!(f, "Bottom Left"),
            PositionOption::BottomRight => write!(f, "Bottom Right"),
            PositionOption::Center => write!(f, "Center"),
        }
    }
}

impl From<CoreBadgePosition> for PositionOption {
    fn from(bp: CoreBadgePosition) -> Self {
        match bp {
            CoreBadgePosition::TopLeft => PositionOption::TopLeft,
            CoreBadgePosition::TopRight => PositionOption::TopRight,
            CoreBadgePosition::BottomLeft => PositionOption::BottomLeft,
            CoreBadgePosition::BottomRight => PositionOption::BottomRight,
            CoreBadgePosition::Center => PositionOption::Center,
        }
    }
}

impl From<PositionOption> for CoreBadgePosition {
    fn from(po: PositionOption) -> Self {
        match po {
            PositionOption::TopLeft => CoreBadgePosition::TopLeft,
            PositionOption::TopRight => CoreBadgePosition::TopRight,
            PositionOption::BottomLeft => CoreBadgePosition::BottomLeft,
            PositionOption::BottomRight => CoreBadgePosition::BottomRight,
            PositionOption::Center => CoreBadgePosition::Center,
        }
    }
}

/// Background style options for pick list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackgroundOption {
    #[default]
    Transparent,
    Solid,
    Gradient,
}

impl std::fmt::Display for BackgroundOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackgroundOption::Transparent => write!(f, "Transparent"),
            BackgroundOption::Solid => write!(f, "Solid Color"),
            BackgroundOption::Gradient => write!(f, "Gradient"),
        }
    }
}

impl From<&BackgroundStyle> for BackgroundOption {
    fn from(bs: &BackgroundStyle) -> Self {
        match bs {
            BackgroundStyle::Transparent => BackgroundOption::Transparent,
            BackgroundStyle::Solid(_) => BackgroundOption::Solid,
            BackgroundStyle::Gradient { .. } => BackgroundOption::Gradient,
            BackgroundStyle::Image { .. } => BackgroundOption::Solid, // Fallback
        }
    }
}

/// Settings panel widget
pub struct SettingsPanel {
    config: EffectsConfig,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPanel {
    /// Create a new settings panel with default config
    pub fn new() -> Self {
        Self {
            config: EffectsConfig::default(),
        }
    }

    /// Create with existing config
    pub fn with_config(config: EffectsConfig) -> Self {
        Self { config }
    }

    /// Get the current config
    pub fn config(&self) -> &EffectsConfig {
        &self.config
    }

    /// Set the config
    pub fn set_config(&mut self, config: EffectsConfig) {
        self.config = config;
    }

    /// Handle a settings message, returning updated config
    pub fn update(&mut self, message: SettingsMessage) {
        match message {
            // Zoom settings
            SettingsMessage::ZoomEnabledChanged(enabled) => {
                self.config.zoom.enabled = enabled;
            }
            SettingsMessage::ZoomLevelChanged(level) => {
                self.config.zoom.max_zoom = level;
            }
            SettingsMessage::ZoomTransitionChanged(ms) => {
                self.config.zoom.transition_duration_ms = ms;
            }
            SettingsMessage::ZoomIdleTimeoutChanged(ms) => {
                self.config.zoom.idle_timeout_ms = ms;
            }
            SettingsMessage::ZoomEasingChanged(easing) => {
                self.config.zoom.easing = easing.into();
            }

            // Keyboard settings
            SettingsMessage::KeyboardEnabledChanged(enabled) => {
                self.config.keyboard.enabled = enabled;
            }
            SettingsMessage::KeyboardPositionChanged(pos) => {
                self.config.keyboard.position = pos.into();
            }
            SettingsMessage::KeyboardFadeChanged(ms) => {
                self.config.keyboard.fade_out_duration_ms = ms;
            }
            SettingsMessage::KeyboardFontSizeChanged(size) => {
                self.config.keyboard.font_size = size;
            }

            // Background settings
            SettingsMessage::BackgroundStyleChanged(style) => {
                self.config.background.style = match style {
                    BackgroundOption::Transparent => BackgroundStyle::Transparent,
                    BackgroundOption::Solid => {
                        BackgroundStyle::Solid(CoreColor::rgba_u8(30, 30, 30, 255))
                    }
                    BackgroundOption::Gradient => BackgroundStyle::Gradient {
                        start: CoreColor::rgba_u8(40, 40, 60, 255),
                        end: CoreColor::rgba_u8(20, 20, 40, 255),
                        angle: 135.0,
                    },
                };
            }
            SettingsMessage::BackgroundPaddingChanged(padding) => {
                self.config.background.padding = Padding::all(padding);
            }
            SettingsMessage::BackgroundCornerRadiusChanged(radius) => {
                self.config.background.corner_radius = radius;
            }
        }
    }

    /// Build the view
    pub fn view(&self) -> Element<'_, SettingsMessage> {
        let title = text("Effects Settings")
            .size(24)
            .style(iced::theme::Text::Color(iced::Color::WHITE));

        // Zoom section
        let zoom_section = self.zoom_section();

        // Keyboard section
        let keyboard_section = self.keyboard_section();

        // Background section
        let background_section = self.background_section();

        let content = column![
            title,
            Space::with_height(20),
            zoom_section,
            Space::with_height(20),
            keyboard_section,
            Space::with_height(20),
            background_section,
        ]
        .spacing(10)
        .padding(20)
        .width(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn zoom_section(&self) -> Element<'_, SettingsMessage> {
        let header =
            text("Zoom Effect")
                .size(18)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.8, 0.8, 0.8,
                )));

        let enabled = checkbox("Enable auto-zoom", self.config.zoom.enabled)
            .on_toggle(SettingsMessage::ZoomEnabledChanged);

        let zoom_level_label = text(format!("Max zoom: {:.1}x", self.config.zoom.max_zoom))
            .size(14)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.7, 0.7, 0.7,
            )));
        let zoom_level = slider(1.0..=3.0, self.config.zoom.max_zoom, |v| {
            SettingsMessage::ZoomLevelChanged(v)
        })
        .step(0.1);

        let transition_label = text(format!(
            "Transition: {}ms",
            self.config.zoom.transition_duration_ms
        ))
        .size(14)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(
            0.7, 0.7, 0.7,
        )));
        let transition = slider(
            100.0..=1000.0,
            self.config.zoom.transition_duration_ms as f32,
            |v| SettingsMessage::ZoomTransitionChanged(v as u32),
        )
        .step(50.0);

        let idle_label = text(format!(
            "Idle timeout: {}ms",
            self.config.zoom.idle_timeout_ms
        ))
        .size(14)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(
            0.7, 0.7, 0.7,
        )));
        let idle = slider(
            500.0..=5000.0,
            self.config.zoom.idle_timeout_ms as f32,
            |v| SettingsMessage::ZoomIdleTimeoutChanged(v as u32),
        )
        .step(100.0);

        let easing_label =
            text("Easing:")
                .size(14)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.7, 0.7, 0.7,
                )));
        let easing_options = vec![
            EasingOption::Linear,
            EasingOption::EaseIn,
            EasingOption::EaseOut,
            EasingOption::EaseInOutCubic,
            EasingOption::EaseInOutQuad,
        ];
        let easing = pick_list(
            easing_options,
            Some(EasingOption::from(self.config.zoom.easing)),
            SettingsMessage::ZoomEasingChanged,
        );
        let easing_row =
            row![easing_label, Space::with_width(10), easing].align_items(Alignment::Center);

        column![
            header,
            Space::with_height(10),
            enabled,
            Space::with_height(10),
            zoom_level_label,
            zoom_level,
            transition_label,
            transition,
            idle_label,
            idle,
            easing_row,
        ]
        .spacing(5)
        .into()
    }

    fn keyboard_section(&self) -> Element<'_, SettingsMessage> {
        let header = text("Keyboard Display")
            .size(18)
            .style(iced::theme::Text::Color(iced::Color::from_rgb(
                0.8, 0.8, 0.8,
            )));

        let enabled = checkbox("Show keyboard shortcuts", self.config.keyboard.enabled)
            .on_toggle(SettingsMessage::KeyboardEnabledChanged);

        let position_label =
            text("Position:")
                .size(14)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.7, 0.7, 0.7,
                )));
        let position_options = vec![
            PositionOption::TopLeft,
            PositionOption::TopRight,
            PositionOption::BottomLeft,
            PositionOption::BottomRight,
            PositionOption::Center,
        ];
        let position = pick_list(
            position_options,
            Some(PositionOption::from(self.config.keyboard.position)),
            SettingsMessage::KeyboardPositionChanged,
        );
        let position_row =
            row![position_label, Space::with_width(10), position].align_items(Alignment::Center);

        let fade_label = text(format!(
            "Fade duration: {}ms",
            self.config.keyboard.fade_out_duration_ms
        ))
        .size(14)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(
            0.7, 0.7, 0.7,
        )));
        let fade = slider(
            100.0..=2000.0,
            self.config.keyboard.fade_out_duration_ms as f32,
            |v| SettingsMessage::KeyboardFadeChanged(v as u32),
        )
        .step(100.0);

        let font_label = text(format!(
            "Font size: {:.0}px",
            self.config.keyboard.font_size
        ))
        .size(14)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(
            0.7, 0.7, 0.7,
        )));
        let font = slider(12.0..=48.0, self.config.keyboard.font_size, |v| {
            SettingsMessage::KeyboardFontSizeChanged(v)
        })
        .step(2.0);

        column![
            header,
            Space::with_height(10),
            enabled,
            Space::with_height(10),
            position_row,
            fade_label,
            fade,
            font_label,
            font,
        ]
        .spacing(5)
        .into()
    }

    fn background_section(&self) -> Element<'_, SettingsMessage> {
        let header =
            text("Background")
                .size(18)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.8, 0.8, 0.8,
                )));

        let style_label =
            text("Style:")
                .size(14)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(
                    0.7, 0.7, 0.7,
                )));
        let style_options = vec![
            BackgroundOption::Transparent,
            BackgroundOption::Solid,
            BackgroundOption::Gradient,
        ];
        let style = pick_list(
            style_options,
            Some(BackgroundOption::from(&self.config.background.style)),
            SettingsMessage::BackgroundStyleChanged,
        );
        let style_row =
            row![style_label, Space::with_width(10), style].align_items(Alignment::Center);

        let padding_label = text(format!(
            "Padding: {:.0}px",
            self.config.background.padding.top
        ))
        .size(14)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(
            0.7, 0.7, 0.7,
        )));
        let padding = slider(0.0..=100.0, self.config.background.padding.top, |v| {
            SettingsMessage::BackgroundPaddingChanged(v)
        })
        .step(5.0);

        let radius_label = text(format!(
            "Corner radius: {:.0}px",
            self.config.background.corner_radius
        ))
        .size(14)
        .style(iced::theme::Text::Color(iced::Color::from_rgb(
            0.7, 0.7, 0.7,
        )));
        let radius = slider(0.0..=50.0, self.config.background.corner_radius, |v| {
            SettingsMessage::BackgroundCornerRadiusChanged(v)
        })
        .step(2.0);

        column![
            header,
            Space::with_height(10),
            style_row,
            padding_label,
            padding,
            radius_label,
            radius,
        ]
        .spacing(5)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_update() {
        let mut panel = SettingsPanel::new();

        // Test zoom toggle
        panel.update(SettingsMessage::ZoomEnabledChanged(false));
        assert!(!panel.config().zoom.enabled);

        // Test zoom level
        panel.update(SettingsMessage::ZoomLevelChanged(2.0));
        assert!((panel.config().zoom.max_zoom - 2.0).abs() < 0.01);

        // Test keyboard position
        panel.update(SettingsMessage::KeyboardPositionChanged(
            PositionOption::TopLeft,
        ));
        assert_eq!(panel.config().keyboard.position, CoreBadgePosition::TopLeft);

        // Test background style
        panel.update(SettingsMessage::BackgroundStyleChanged(
            BackgroundOption::Gradient,
        ));
        assert!(matches!(
            panel.config().background.style,
            BackgroundStyle::Gradient { .. }
        ));
    }

    #[test]
    fn test_option_conversions() {
        // Easing round-trip
        let original = EasingFunction::EaseInOutCubic;
        let option = EasingOption::from(original);
        let back: EasingFunction = option.into();
        assert_eq!(original, back);

        // Position round-trip
        let original = CoreBadgePosition::BottomRight;
        let option = PositionOption::from(original);
        let back: CoreBadgePosition = option.into();
        assert_eq!(original, back);
    }
}
