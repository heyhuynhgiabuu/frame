//! Keyboard badge widget for displaying shortcut overlays
//!
//! Renders keyboard shortcuts as visually appealing badges during recording.
//! Supports customizable appearance, positioning, and fade animations.

use iced::widget::canvas::{self, Cache, Canvas, Frame, Geometry, Path, Program, Text};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};
use std::time::Duration;

/// Badge position on screen
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum BadgePosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    #[default]
    BottomCenter,
    BottomRight,
}

impl BadgePosition {
    /// Calculate the anchor point for a badge of given size within bounds
    pub fn anchor(&self, bounds: Size, badge_size: Size, margin: f32) -> Point {
        let x = match self {
            BadgePosition::TopLeft | BadgePosition::BottomLeft => margin,
            BadgePosition::TopCenter | BadgePosition::BottomCenter => {
                (bounds.width - badge_size.width) / 2.0
            }
            BadgePosition::TopRight | BadgePosition::BottomRight => {
                bounds.width - badge_size.width - margin
            }
        };

        let y = match self {
            BadgePosition::TopLeft | BadgePosition::TopCenter | BadgePosition::TopRight => margin,
            BadgePosition::BottomLeft
            | BadgePosition::BottomCenter
            | BadgePosition::BottomRight => bounds.height - badge_size.height - margin,
        };

        Point::new(x, y)
    }
}

/// Configuration for keyboard badge appearance
#[derive(Debug, Clone)]
pub struct BadgeConfig {
    /// Background color of the badge
    pub background: Color,
    /// Text color
    pub text_color: Color,
    /// Font size in pixels
    pub font_size: f32,
    /// Corner radius
    pub corner_radius: f32,
    /// Horizontal padding around text
    pub padding_h: f32,
    /// Vertical padding around text
    pub padding_v: f32,
    /// Position on screen
    pub position: BadgePosition,
    /// Margin from screen edge
    pub margin: f32,
    /// Fade duration in milliseconds
    pub fade_duration_ms: u64,
}

impl Default for BadgeConfig {
    fn default() -> Self {
        Self {
            background: Color::from_rgba(0.0, 0.0, 0.0, 0.75),
            text_color: Color::WHITE,
            font_size: 24.0,
            corner_radius: 8.0,
            padding_h: 16.0,
            padding_v: 10.0,
            position: BadgePosition::BottomCenter,
            margin: 20.0,
            fade_duration_ms: 2000,
        }
    }
}

impl BadgeConfig {
    /// Create a config optimized for video recording overlays
    pub fn for_recording() -> Self {
        Self {
            background: Color::from_rgba(0.1, 0.1, 0.1, 0.85),
            text_color: Color::from_rgb(0.95, 0.95, 0.95),
            font_size: 28.0,
            corner_radius: 12.0,
            padding_h: 20.0,
            padding_v: 12.0,
            ..Default::default()
        }
    }

    /// Create a minimal config for smaller badges
    pub fn minimal() -> Self {
        Self {
            background: Color::from_rgba(0.2, 0.2, 0.2, 0.7),
            text_color: Color::WHITE,
            font_size: 16.0,
            corner_radius: 4.0,
            padding_h: 8.0,
            padding_v: 4.0,
            ..Default::default()
        }
    }
}

/// Keyboard badge widget state
#[derive(Debug)]
pub struct KeyboardBadge {
    /// Current text to display (None = hidden)
    content: Option<String>,
    /// When the badge became visible
    show_time: Option<Duration>,
    /// Current time for animation
    current_time: Duration,
    /// Configuration
    config: BadgeConfig,
    /// Canvas cache for efficient rendering
    cache: Cache,
}

impl Default for KeyboardBadge {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyboardBadge {
    /// Create a new keyboard badge widget
    pub fn new() -> Self {
        Self {
            content: None,
            show_time: None,
            current_time: Duration::ZERO,
            config: BadgeConfig::default(),
            cache: Cache::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: BadgeConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Set the content to display
    pub fn set_content(&mut self, content: Option<String>) {
        let changed = self.content != content;
        if changed {
            self.content = content.clone();
            if content.is_some() && self.show_time.is_none() {
                self.show_time = Some(self.current_time);
            } else if content.is_none() {
                self.show_time = None;
            }
            self.cache.clear();
        }
    }

    /// Update current time for animations
    pub fn update_time(&mut self, current_time: Duration) {
        let need_redraw = self.current_time != current_time && self.content.is_some();
        self.current_time = current_time;
        if need_redraw {
            self.cache.clear();
        }
    }

    /// Get the current opacity based on fade animation
    pub fn opacity(&self) -> f32 {
        let Some(show_time) = self.show_time else {
            return 0.0;
        };

        let elapsed = self.current_time.saturating_sub(show_time);
        let fade_duration = Duration::from_millis(self.config.fade_duration_ms);

        if elapsed < fade_duration {
            1.0
        } else {
            // Fade out over 500ms after fade_duration
            let fade_elapsed = elapsed.saturating_sub(fade_duration);
            let fade_progress = fade_elapsed.as_secs_f32() / 0.5;
            (1.0 - fade_progress).clamp(0.0, 1.0)
        }
    }

    /// Check if badge is currently visible
    pub fn is_visible(&self) -> bool {
        self.content.is_some() && self.opacity() > 0.01
    }

    /// Get configuration
    pub fn config(&self) -> &BadgeConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: BadgeConfig) {
        self.config = config;
        self.cache.clear();
    }

    /// Set badge position
    pub fn set_position(&mut self, position: BadgePosition) {
        self.config.position = position;
        self.cache.clear();
    }

    /// Build the view element
    pub fn view(&self) -> Element<'_, ()> {
        Canvas::new(KeyboardBadgeProgram { badge: self })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Build a fixed-size view element
    pub fn view_fixed(&self, width: f32, height: f32) -> Element<'_, ()> {
        Canvas::new(KeyboardBadgeProgram { badge: self })
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .into()
    }
}

/// Canvas program for rendering the badge
struct KeyboardBadgeProgram<'a> {
    badge: &'a KeyboardBadge,
}

impl<'a, Message> Program<Message> for KeyboardBadgeProgram<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let Some(content) = &self.badge.content else {
            return vec![];
        };

        let opacity = self.badge.opacity();
        if opacity < 0.01 {
            return vec![];
        }

        let mut frame = Frame::new(renderer, bounds.size());
        let config = &self.badge.config;

        // Calculate badge size based on text
        // Approximate: 0.6 * font_size per character for width
        let text_width = content.len() as f32 * config.font_size * 0.6;
        let badge_width = text_width + config.padding_h * 2.0;
        let badge_height = config.font_size + config.padding_v * 2.0;
        let badge_size = Size::new(badge_width, badge_height);

        // Get position
        let pos = config
            .position
            .anchor(bounds.size(), badge_size, config.margin);

        // Draw rounded rectangle background
        let bg_color = Color {
            a: config.background.a * opacity,
            ..config.background
        };

        let rect = rounded_rect(pos, badge_size, config.corner_radius);
        frame.fill(&rect, bg_color);

        // Draw text centered in badge
        let text_color = Color {
            a: config.text_color.a * opacity,
            ..config.text_color
        };

        let text = Text {
            content: content.clone(),
            position: Point::new(
                pos.x + badge_width / 2.0,
                pos.y + badge_height / 2.0 - config.font_size * 0.35, // Visual centering
            ),
            color: text_color,
            size: iced::Pixels(config.font_size),
            horizontal_alignment: iced::alignment::Horizontal::Center,
            vertical_alignment: iced::alignment::Vertical::Top,
            ..Text::default()
        };
        frame.fill_text(text);

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        _event: canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        // Badge is display-only, doesn't handle input
        (canvas::event::Status::Ignored, None)
    }
}

/// Create a rounded rectangle path
fn rounded_rect(pos: Point, size: Size, radius: f32) -> Path {
    Path::new(|builder| {
        let x = pos.x;
        let y = pos.y;
        let w = size.width;
        let h = size.height;
        let r = radius.min(w / 2.0).min(h / 2.0);

        // Start at top-left corner after the radius
        builder.move_to(Point::new(x + r, y));

        // Top edge
        builder.line_to(Point::new(x + w - r, y));
        // Top-right corner
        builder.arc_to(Point::new(x + w, y), Point::new(x + w, y + r), r);

        // Right edge
        builder.line_to(Point::new(x + w, y + h - r));
        // Bottom-right corner
        builder.arc_to(Point::new(x + w, y + h), Point::new(x + w - r, y + h), r);

        // Bottom edge
        builder.line_to(Point::new(x + r, y + h));
        // Bottom-left corner
        builder.arc_to(Point::new(x, y + h), Point::new(x, y + h - r), r);

        // Left edge
        builder.line_to(Point::new(x, y + r));
        // Top-left corner
        builder.arc_to(Point::new(x, y), Point::new(x + r, y), r);

        builder.close();
    })
}

/// Render a keyboard badge to a RGBA buffer for compositing
///
/// This is used when rendering badges directly into video frames,
/// bypassing the iced canvas system.
pub fn render_badge_to_buffer(
    content: &str,
    config: &BadgeConfig,
    opacity: f32,
    width: u32,
    height: u32,
) -> Vec<u8> {
    // Create RGBA buffer
    let mut buffer = vec![0u8; (width * height * 4) as usize];

    // Calculate badge dimensions
    let text_width = content.len() as f32 * config.font_size * 0.6;
    let badge_width = text_width + config.padding_h * 2.0;
    let badge_height = config.font_size + config.padding_v * 2.0;
    let badge_size = Size::new(badge_width, badge_height);

    // Get position
    let bounds_size = Size::new(width as f32, height as f32);
    let pos = config
        .position
        .anchor(bounds_size, badge_size, config.margin);

    // Draw rounded rectangle (simplified - uses rectangles)
    let bg_alpha = (config.background.a * opacity * 255.0) as u8;
    let bg_r = (config.background.r * 255.0) as u8;
    let bg_g = (config.background.g * 255.0) as u8;
    let bg_b = (config.background.b * 255.0) as u8;

    // Fill badge background (simplified rectangular version)
    let start_x = pos.x.max(0.0) as u32;
    let start_y = pos.y.max(0.0) as u32;
    let end_x = (pos.x + badge_width).min(width as f32) as u32;
    let end_y = (pos.y + badge_height).min(height as f32) as u32;

    for y in start_y..end_y {
        for x in start_x..end_x {
            // Simple rounded corner check
            let local_x = x as f32 - pos.x;
            let local_y = y as f32 - pos.y;

            let in_corner = is_in_rounded_rect(
                local_x,
                local_y,
                badge_width,
                badge_height,
                config.corner_radius,
            );

            if in_corner {
                let idx = ((y * width + x) * 4) as usize;
                buffer[idx] = bg_r;
                buffer[idx + 1] = bg_g;
                buffer[idx + 2] = bg_b;
                buffer[idx + 3] = bg_alpha;
            }
        }
    }

    // Note: Text rendering would require a font rasterizer (e.g., fontdue)
    // For now, the background is rendered and text would be added by
    // the video compositor or through a separate text rendering pass.

    buffer
}

/// Check if point is within rounded rectangle
fn is_in_rounded_rect(x: f32, y: f32, width: f32, height: f32, radius: f32) -> bool {
    let r = radius.min(width / 2.0).min(height / 2.0);

    // Check corners
    if x < r && y < r {
        // Top-left corner
        let dx = r - x;
        let dy = r - y;
        return dx * dx + dy * dy <= r * r;
    }
    if x > width - r && y < r {
        // Top-right corner
        let dx = x - (width - r);
        let dy = r - y;
        return dx * dx + dy * dy <= r * r;
    }
    if x < r && y > height - r {
        // Bottom-left corner
        let dx = r - x;
        let dy = y - (height - r);
        return dx * dx + dy * dy <= r * r;
    }
    if x > width - r && y > height - r {
        // Bottom-right corner
        let dx = x - (width - r);
        let dy = y - (height - r);
        return dx * dx + dy * dy <= r * r;
    }

    // Inside the rectangle body
    x >= 0.0 && x <= width && y >= 0.0 && y <= height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_position() {
        let bounds = Size::new(1920.0, 1080.0);
        let badge = Size::new(100.0, 40.0);
        let margin = 20.0;

        // Bottom center
        let pos = BadgePosition::BottomCenter.anchor(bounds, badge, margin);
        assert!((pos.x - 910.0).abs() < 0.01); // (1920 - 100) / 2
        assert!((pos.y - 1020.0).abs() < 0.01); // 1080 - 40 - 20

        // Top left
        let pos = BadgePosition::TopLeft.anchor(bounds, badge, margin);
        assert!((pos.x - 20.0).abs() < 0.01);
        assert!((pos.y - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_badge_opacity() {
        let mut badge = KeyboardBadge::new();
        badge.config.fade_duration_ms = 1000;

        // No content = no opacity
        assert!(badge.opacity() < 0.01);

        // Set content
        badge.set_content(Some("⌘S".to_string()));
        assert!((badge.opacity() - 1.0).abs() < 0.01);

        // After fade duration, should start fading
        badge.update_time(Duration::from_millis(1500));
        assert!(badge.opacity() < 1.0);
        assert!(badge.opacity() > 0.0);
    }

    #[test]
    fn test_rounded_rect_corners() {
        // Center of rect
        assert!(is_in_rounded_rect(50.0, 20.0, 100.0, 40.0, 8.0));

        // Just inside corner
        assert!(is_in_rounded_rect(6.0, 6.0, 100.0, 40.0, 8.0));

        // Outside corner (in the cut-off area)
        assert!(!is_in_rounded_rect(0.5, 0.5, 100.0, 40.0, 8.0));
    }

    #[test]
    fn test_render_buffer() {
        let config = BadgeConfig::default();
        let buffer = render_badge_to_buffer("Test", &config, 1.0, 200, 100);

        // Buffer should be correct size
        assert_eq!(buffer.len(), 200 * 100 * 4);

        // Some pixels should be non-zero (badge is drawn)
        let non_zero = buffer.iter().filter(|&&b| b != 0).count();
        assert!(non_zero > 0);
    }
}
