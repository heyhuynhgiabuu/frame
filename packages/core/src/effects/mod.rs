//! Effects pipeline for Frame video processing
//!
//! This module provides video effects that are applied between capture and encoding:
//! - Cursor tracking and smooth zoom
//! - Keyboard shortcut display
//! - Background compositing with padding/rounded corners
//!
//! Effects are composable and can be enabled/disabled independently.

use crate::capture::Frame;
use crate::FrameResult;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// Sub-modules
pub mod background; // Task: effects-4
pub mod cursor; // Task: effects-1
pub mod keyboard; // Task: effects-3
pub mod pipeline; // Task: integration-1
pub mod zoom; // Task: effects-2

// Re-export the integrated pipeline
pub use pipeline::IntegratedPipeline;

/// Cursor position with timestamp for tracking
#[derive(Debug, Clone, Copy, Default)]
pub struct CursorPosition {
    pub x: f32,
    pub y: f32,
    pub timestamp: Duration,
}

impl CursorPosition {
    pub fn new(x: f32, y: f32, timestamp: Duration) -> Self {
        Self { x, y, timestamp }
    }
}

/// Mouse event types for zoom triggering
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseEvent {
    Click { x: f32, y: f32, button: MouseButton },
    Scroll { x: f32, y: f32, delta_y: f32 },
    Move { x: f32, y: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Keyboard modifier keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub command: bool, // ⌘ on macOS
    pub shift: bool,   // ⇧
    pub option: bool,  // ⌥ on macOS (Alt on other platforms)
    pub control: bool, // ⌃
}

impl Modifiers {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn any_active(&self) -> bool {
        self.command || self.shift || self.option || self.control
    }
}

/// A keyboard event with timing
#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    pub key: Key,
    pub modifiers: Modifiers,
    pub timestamp: Duration,
    pub pressed: bool, // true = key down, false = key up
}

/// Key representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// Character key (letter, number, symbol)
    Character(char),
    /// Named special key
    Named(NamedKey),
}

/// Named special keys
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedKey {
    Space,
    Return,
    Tab,
    Escape,
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

impl Key {
    /// Get display string for the key (macOS style)
    pub fn display(&self) -> String {
        match self {
            Key::Character(c) => c.to_uppercase().to_string(),
            Key::Named(n) => match n {
                NamedKey::Space => "Space".to_string(),
                NamedKey::Return => "↩".to_string(),
                NamedKey::Tab => "⇥".to_string(),
                NamedKey::Escape => "⎋".to_string(),
                NamedKey::Backspace => "⌫".to_string(),
                NamedKey::Delete => "⌦".to_string(),
                NamedKey::ArrowUp => "↑".to_string(),
                NamedKey::ArrowDown => "↓".to_string(),
                NamedKey::ArrowLeft => "←".to_string(),
                NamedKey::ArrowRight => "→".to_string(),
                NamedKey::Home => "↖".to_string(),
                NamedKey::End => "↘".to_string(),
                NamedKey::PageUp => "⇞".to_string(),
                NamedKey::PageDown => "⇟".to_string(),
                NamedKey::F1 => "F1".to_string(),
                NamedKey::F2 => "F2".to_string(),
                NamedKey::F3 => "F3".to_string(),
                NamedKey::F4 => "F4".to_string(),
                NamedKey::F5 => "F5".to_string(),
                NamedKey::F6 => "F6".to_string(),
                NamedKey::F7 => "F7".to_string(),
                NamedKey::F8 => "F8".to_string(),
                NamedKey::F9 => "F9".to_string(),
                NamedKey::F10 => "F10".to_string(),
                NamedKey::F11 => "F11".to_string(),
                NamedKey::F12 => "F12".to_string(),
            },
        }
    }
}

impl Modifiers {
    /// Get display string for modifiers (macOS style, in standard order: ⌃⌥⇧⌘)
    pub fn display(&self) -> String {
        let mut s = String::new();
        if self.control {
            s.push('⌃');
        }
        if self.option {
            s.push('⌥');
        }
        if self.shift {
            s.push('⇧');
        }
        if self.command {
            s.push('⌘');
        }
        s
    }
}

/// Background configuration for video compositing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Background {
    pub style: BackgroundStyle,
    pub padding: Padding,
    pub corner_radius: f32,
}

impl Default for Background {
    fn default() -> Self {
        Self {
            style: BackgroundStyle::Transparent,
            padding: Padding::default(),
            corner_radius: 0.0,
        }
    }
}

/// Background fill style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackgroundStyle {
    /// No background (transparent)
    Transparent,
    /// Solid color (RGBA)
    Solid(Color),
    /// Linear gradient
    Gradient {
        start: Color,
        end: Color,
        angle: f32, // degrees, 0 = left-to-right
    },
    /// Image background
    Image {
        path: std::path::PathBuf,
        scale_mode: ImageScaleMode,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ImageScaleMode {
    Fill,
    Fit,
    Tile,
}

/// RGBA color (0.0-1.0 range)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    pub const WHITE: Color = Color::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);
    pub const TRANSPARENT: Color = Color::new(0.0, 0.0, 0.0, 0.0);
}

/// Padding values (in pixels)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Padding {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Padding {
    pub const fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub const fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    pub const fn zero() -> Self {
        Self::all(0.0)
    }
}

/// Zoom effect configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoomConfig {
    /// Whether zoom is enabled
    pub enabled: bool,
    /// Maximum zoom level (1.0 = no zoom, 2.0 = 2x)
    pub max_zoom: f32,
    /// Duration of zoom transition (ms)
    pub transition_duration_ms: u32,
    /// Time before zooming out when idle (ms)
    pub idle_timeout_ms: u32,
    /// Easing function for zoom transitions
    pub easing: EasingFunction,
}

impl Default for ZoomConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_zoom: 1.5,
            transition_duration_ms: 300,
            idle_timeout_ms: 2000,
            easing: EasingFunction::EaseInOutCubic,
        }
    }
}

/// Keyboard display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardConfig {
    /// Whether keyboard display is enabled
    pub enabled: bool,
    /// Position of keyboard badges
    pub position: BadgePosition,
    /// Duration to show badge after key release (ms)
    pub fade_out_duration_ms: u32,
    /// Badge background color
    pub background_color: Color,
    /// Badge text color
    pub text_color: Color,
    /// Badge font size
    pub font_size: f32,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: BadgePosition::BottomRight,
            fade_out_duration_ms: 500,
            background_color: Color::rgba_u8(0, 0, 0, 200),
            text_color: Color::WHITE,
            font_size: 24.0,
        }
    }
}

/// Position for UI overlays
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BadgePosition {
    TopLeft,
    TopRight,
    BottomLeft,
    #[default]
    BottomRight,
    Center,
}

/// Easing functions for smooth animations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EasingFunction {
    Linear,
    EaseIn,
    EaseOut,
    #[default]
    EaseInOutCubic,
    EaseInOutQuad,
}

impl EasingFunction {
    /// Apply easing to a progress value (0.0 to 1.0)
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            EasingFunction::Linear => t,
            EasingFunction::EaseIn => t * t,
            EasingFunction::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            EasingFunction::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            EasingFunction::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
        }
    }
}

/// Complete effects configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EffectsConfig {
    pub zoom: ZoomConfig,
    pub keyboard: KeyboardConfig,
    pub background: Background,
}

/// Input events for effects processing
#[derive(Debug, Clone)]
pub enum EffectInput {
    Cursor(CursorPosition),
    Mouse(MouseEvent),
    Keyboard(KeyboardEvent),
}

/// Output from effects processing
#[derive(Debug, Clone)]
pub struct ProcessedFrame {
    /// The processed frame data
    pub frame: Frame,
    /// Keyboard badges to overlay (if any)
    pub keyboard_badges: Vec<KeyboardBadge>,
}

/// A keyboard badge to render
#[derive(Debug, Clone)]
pub struct KeyboardBadge {
    pub text: String,
    pub opacity: f32, // 0.0-1.0 for fade out
    pub position: BadgePosition,
}

/// Trait for effects pipeline implementation
///
/// The effects pipeline processes frames between capture and encoding,
/// applying zoom, overlays, and compositing.
pub trait EffectsPipeline: Send + Sync {
    /// Get the current configuration
    fn config(&self) -> &EffectsConfig;

    /// Update the configuration
    fn set_config(&mut self, config: EffectsConfig);

    /// Process an input event (cursor movement, key press, etc.)
    fn process_input(&mut self, input: EffectInput);

    /// Process a frame with current effects state
    fn process_frame(&mut self, frame: Frame) -> FrameResult<ProcessedFrame>;

    /// Reset effects state (e.g., when starting new recording)
    fn reset(&mut self);
}

/// Default no-op effects pipeline (passes frames through unchanged)
pub struct NoOpEffectsPipeline {
    config: EffectsConfig,
}

impl NoOpEffectsPipeline {
    pub fn new() -> Self {
        Self {
            config: EffectsConfig::default(),
        }
    }
}

impl Default for NoOpEffectsPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectsPipeline for NoOpEffectsPipeline {
    fn config(&self) -> &EffectsConfig {
        &self.config
    }

    fn set_config(&mut self, config: EffectsConfig) {
        self.config = config;
    }

    fn process_input(&mut self, _input: EffectInput) {
        // No-op
    }

    fn process_frame(&mut self, frame: Frame) -> FrameResult<ProcessedFrame> {
        Ok(ProcessedFrame {
            frame,
            keyboard_badges: vec![],
        })
    }

    fn reset(&mut self) {
        // No state to reset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_functions() {
        // Linear should be identity
        assert_eq!(EasingFunction::Linear.apply(0.5), 0.5);

        // All functions should map 0 -> 0 and 1 -> 1
        for easing in [
            EasingFunction::Linear,
            EasingFunction::EaseIn,
            EasingFunction::EaseOut,
            EasingFunction::EaseInOutCubic,
            EasingFunction::EaseInOutQuad,
        ] {
            assert!((easing.apply(0.0) - 0.0).abs() < 0.001);
            assert!((easing.apply(1.0) - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_modifiers_display() {
        let mods = Modifiers {
            command: true,
            shift: true,
            option: false,
            control: false,
        };
        assert_eq!(mods.display(), "⇧⌘");

        let all = Modifiers {
            command: true,
            shift: true,
            option: true,
            control: true,
        };
        // Order should be ⌃⌥⇧⌘
        assert_eq!(all.display(), "⌃⌥⇧⌘");
    }

    #[test]
    fn test_key_display() {
        assert_eq!(Key::Character('s').display(), "S");
        assert_eq!(Key::Named(NamedKey::Return).display(), "↩");
        assert_eq!(Key::Named(NamedKey::Tab).display(), "⇥");
    }

    #[test]
    fn test_color_constructors() {
        let white = Color::WHITE;
        assert_eq!(white.r, 1.0);
        assert_eq!(white.a, 1.0);

        let semi = Color::rgba_u8(255, 128, 0, 128);
        assert!((semi.r - 1.0).abs() < 0.01);
        assert!((semi.g - 0.5).abs() < 0.01);
        assert!((semi.a - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_no_op_pipeline() {
        use crate::capture::PixelFormat;

        let mut pipeline = NoOpEffectsPipeline::new();

        let frame = Frame {
            data: vec![0u8; 100],
            width: 10,
            height: 10,
            timestamp: Duration::from_millis(100),
            format: PixelFormat::Rgba,
        };

        let result = pipeline.process_frame(frame.clone()).unwrap();
        assert_eq!(result.frame.data.len(), frame.data.len());
        assert!(result.keyboard_badges.is_empty());
    }

    #[test]
    fn test_padding() {
        let uniform = Padding::all(10.0);
        assert_eq!(uniform.top, 10.0);
        assert_eq!(uniform.right, 10.0);

        let symmetric = Padding::symmetric(5.0, 10.0);
        assert_eq!(symmetric.top, 5.0);
        assert_eq!(symmetric.left, 10.0);
    }
}
