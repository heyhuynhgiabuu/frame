//! Aspect ratio calculations for video frames
//!
//! Provides utilities for calculating frame dimensions with different aspect ratios,
//! letterboxing/pillarboxing for fitting content into target dimensions,
//! and content alignment options.

use serde::{Deserialize, Serialize};

/// Common aspect ratios for video content
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum AspectRatio {
    /// 16:9 landscape (YouTube, modern displays)
    #[default]
    Horizontal16x9,
    /// 9:16 portrait (mobile/TikTok)
    Vertical9x16,
    /// 1:1 square (Instagram)
    Square,
    /// 4:3 standard (classic displays)
    Standard4x3,
    /// 21:9 ultrawide (cinemascope)
    Ultrawide21x9,
    /// 1.85:1 cinema widescreen
    Cinema185,
    /// 2.39:1 cinema anamorphic
    Cinema239,
    /// Custom ratio with explicit width and height
    Custom(u32, u32),
}

impl AspectRatio {
    /// Get the ratio as a floating point value (width / height)
    pub fn ratio_f32(&self) -> f32 {
        match self {
            AspectRatio::Horizontal16x9 => 16.0 / 9.0,
            AspectRatio::Vertical9x16 => 9.0 / 16.0,
            AspectRatio::Square => 1.0,
            AspectRatio::Standard4x3 => 4.0 / 3.0,
            AspectRatio::Ultrawide21x9 => 21.0 / 9.0,
            AspectRatio::Cinema185 => 1.85,
            AspectRatio::Cinema239 => 2.39,
            AspectRatio::Custom(w, h) => *w as f32 / *h as f32,
        }
    }

    /// Calculate dimensions given a base dimension
    ///
    /// For landscape ratios, base is used as the width.
    /// For portrait ratios, base is used as the height.
    /// Returns (width, height) tuple.
    ///
    /// # Examples
    /// ```
    /// use frame_core::effects::aspect_ratio::AspectRatio;
    ///
    /// let (w, h) = AspectRatio::Horizontal16x9.dimensions(1920);
    /// assert_eq!(w, 1920);
    /// assert_eq!(h, 1080);
    /// ```
    pub fn dimensions(&self, base: u32) -> (u32, u32) {
        match self {
            AspectRatio::Vertical9x16 => {
                // For portrait, base is height
                let width = (base as f32 * self.ratio_f32()).round() as u32;
                (width, base)
            }
            _ => {
                // For landscape and square, base is width
                let height = (base as f32 / self.ratio_f32()).round() as u32;
                (base, height)
            }
        }
    }

    /// Get dimensions with a specific width
    pub fn dimensions_for_width(&self, width: u32) -> (u32, u32) {
        let height = (width as f32 / self.ratio_f32()).round() as u32;
        (width, height)
    }

    /// Get dimensions with a specific height
    pub fn dimensions_for_height(&self, height: u32) -> (u32, u32) {
        let width = (height as f32 * self.ratio_f32()).round() as u32;
        (width, height)
    }
}

/// Content alignment options for positioning within letterbox/pillarbox
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ContentAlignment {
    /// Center the content (default)
    #[default]
    Center,
    /// Align to the top
    Top,
    /// Align to the bottom
    Bottom,
    /// Align to the left
    Left,
    /// Align to the right
    Right,
    /// Align to top-left
    TopLeft,
    /// Align to top-right
    TopRight,
    /// Align to bottom-left
    BottomLeft,
    /// Align to bottom-right
    BottomRight,
}

impl ContentAlignment {
    /// Get horizontal offset factor (0.0 = left, 0.5 = center, 1.0 = right)
    pub fn horizontal_factor(&self) -> f32 {
        match self {
            ContentAlignment::Left | ContentAlignment::TopLeft | ContentAlignment::BottomLeft => {
                0.0
            }
            ContentAlignment::Center | ContentAlignment::Top | ContentAlignment::Bottom => 0.5,
            ContentAlignment::Right
            | ContentAlignment::TopRight
            | ContentAlignment::BottomRight => 1.0,
        }
    }

    /// Get vertical offset factor (0.0 = top, 0.5 = center, 1.0 = bottom)
    pub fn vertical_factor(&self) -> f32 {
        match self {
            ContentAlignment::Top | ContentAlignment::TopLeft | ContentAlignment::TopRight => 0.0,
            ContentAlignment::Center | ContentAlignment::Left | ContentAlignment::Right => 0.5,
            ContentAlignment::Bottom
            | ContentAlignment::BottomLeft
            | ContentAlignment::BottomRight => 1.0,
        }
    }
}

/// Letterbox/pillarbox padding information
///
/// Represents the padding required to fit content into a target
/// aspect ratio while maintaining aspect ratio preservation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct LetterboxInfo {
    /// Top padding in pixels
    pub top: u32,
    /// Bottom padding in pixels
    pub bottom: u32,
    /// Left padding in pixels
    pub left: u32,
    /// Right padding in pixels
    pub right: u32,
}

impl LetterboxInfo {
    /// Create uniform padding on all sides
    pub const fn all(padding: u32) -> Self {
        Self {
            top: padding,
            right: padding,
            bottom: padding,
            left: padding,
        }
    }

    /// Create symmetric padding (vertical, horizontal)
    pub const fn symmetric(vertical: u32, horizontal: u32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Create zero padding
    pub const fn zero() -> Self {
        Self::all(0)
    }

    /// Total horizontal padding (left + right)
    pub fn horizontal(&self) -> u32 {
        self.left + self.right
    }

    /// Total vertical padding (top + bottom)
    pub fn vertical(&self) -> u32 {
        self.top + self.bottom
    }

    /// Calculate the content region rectangle (x, y, width, height)
    pub fn content_rect(
        &self,
        container_width: u32,
        container_height: u32,
    ) -> (u32, u32, u32, u32) {
        let width = container_width.saturating_sub(self.horizontal());
        let height = container_height.saturating_sub(self.vertical());
        (self.left, self.top, width, height)
    }

    /// Check if any padding is needed
    pub fn is_empty(&self) -> bool {
        self.top == 0 && self.bottom == 0 && self.left == 0 && self.right == 0
    }
}

/// Calculate letterbox/pillarbox padding to fit content into a target aspect ratio
///
/// This calculates the padding needed to fit content with dimensions
/// (input_width, input_height) into a container with the target aspect ratio
/// while preserving the content's aspect ratio.
///
/// # Arguments
/// * `input_width` - Width of the input content
/// * `input_height` - Height of the input content
/// * `target_ratio` - Target aspect ratio to fit into
/// * `alignment` - Content alignment within the container (default: center)
///
/// # Returns
/// `LetterboxInfo` struct with top, bottom, left, right padding values
///
/// # Examples
/// ```
/// use frame_core::effects::aspect_ratio::{AspectRatio, calculate_letterbox, ContentAlignment};
///
/// // Fitting 16:9 content into a 1:1 square
/// let info = calculate_letterbox(1920, 1080, AspectRatio::Square, ContentAlignment::Center);
/// assert_eq!(info.top, 420);
/// assert_eq!(info.bottom, 420);
/// ```
pub fn calculate_letterbox(
    input_width: u32,
    input_height: u32,
    target_ratio: AspectRatio,
    alignment: ContentAlignment,
) -> LetterboxInfo {
    if input_width == 0 || input_height == 0 {
        return LetterboxInfo::zero();
    }

    let input_ratio = input_width as f32 / input_height as f32;
    let target_ratio_f = target_ratio.ratio_f32();

    // Determine if we need letterbox (horizontal bars) or pillarbox (vertical bars)
    if input_ratio > target_ratio_f {
        // Content is wider than target - pillarbox (add left/right padding)
        let scaled_height = input_height;
        let scaled_width = (scaled_height as f32 * target_ratio_f).round() as u32;
        let padding_total = input_width.saturating_sub(scaled_width);

        let left = (padding_total as f32 * alignment.horizontal_factor()).round() as u32;
        let right = padding_total.saturating_sub(left);

        LetterboxInfo {
            top: 0,
            bottom: 0,
            left,
            right,
        }
    } else if input_ratio < target_ratio_f {
        // Content is taller than target - letterbox (add top/bottom padding)
        let scaled_width = input_width;
        let scaled_height = (scaled_width as f32 / target_ratio_f).round() as u32;
        let padding_total = input_height.saturating_sub(scaled_height);

        let top = (padding_total as f32 * alignment.vertical_factor()).round() as u32;
        let bottom = padding_total.saturating_sub(top);

        LetterboxInfo {
            top,
            bottom,
            left: 0,
            right: 0,
        }
    } else {
        // Ratios match exactly, no padding needed
        LetterboxInfo::zero()
    }
}

/// Calculate dimensions to fit content into a container while preserving aspect ratio
///
/// Similar to `calculate_letterbox` but returns the scaled content dimensions
/// instead of padding values.
pub fn fit_to_container(
    content_width: u32,
    content_height: u32,
    container_width: u32,
    container_height: u32,
) -> (u32, u32) {
    if content_width == 0 || content_height == 0 || container_width == 0 || container_height == 0 {
        return (0, 0);
    }

    let content_ratio = content_width as f32 / content_height as f32;
    let container_ratio = container_width as f32 / container_height as f32;

    if content_ratio > container_ratio {
        // Content is wider - scale to fit container width
        let scaled_width = container_width;
        let scaled_height = (container_width as f32 / content_ratio).round() as u32;
        (scaled_width, scaled_height)
    } else {
        // Content is taller or equal - scale to fit container height
        let scaled_height = container_height;
        let scaled_width = (container_height as f32 * content_ratio).round() as u32;
        (scaled_width, scaled_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aspect_ratio_ratios() {
        assert!((AspectRatio::Horizontal16x9.ratio_f32() - 16.0 / 9.0).abs() < 0.001);
        assert!((AspectRatio::Vertical9x16.ratio_f32() - 9.0 / 16.0).abs() < 0.001);
        assert!((AspectRatio::Square.ratio_f32() - 1.0).abs() < 0.001);
        assert!((AspectRatio::Standard4x3.ratio_f32() - 4.0 / 3.0).abs() < 0.001);
        assert!((AspectRatio::Ultrawide21x9.ratio_f32() - 21.0 / 9.0).abs() < 0.001);
        assert!((AspectRatio::Cinema185.ratio_f32() - 1.85).abs() < 0.001);
        assert!((AspectRatio::Cinema239.ratio_f32() - 2.39).abs() < 0.001);

        let custom = AspectRatio::Custom(3, 2);
        assert!((custom.ratio_f32() - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_dimensions() {
        // 16:9 landscape - base is width
        let (w, h) = AspectRatio::Horizontal16x9.dimensions(1920);
        assert_eq!(w, 1920);
        assert_eq!(h, 1080);

        // 4:3 landscape
        let (w, h) = AspectRatio::Standard4x3.dimensions(1024);
        assert_eq!(w, 1024);
        assert_eq!(h, 768);

        // 9:16 portrait - base is height
        let (w, h) = AspectRatio::Vertical9x16.dimensions(1920);
        assert_eq!(w, 1080);
        assert_eq!(h, 1920);

        // Square
        let (w, h) = AspectRatio::Square.dimensions(500);
        assert_eq!(w, 500);
        assert_eq!(h, 500);

        // Custom
        let (w, h) = AspectRatio::Custom(21, 9).dimensions(1920);
        assert_eq!(w, 1920);
        assert_eq!(h, 823); // 1920 / (21/9) = 823
    }

    #[test]
    fn test_dimensions_for_width() {
        let (w, h) = AspectRatio::Horizontal16x9.dimensions_for_width(1920);
        assert_eq!(w, 1920);
        assert_eq!(h, 1080);

        let (w, h) = AspectRatio::Vertical9x16.dimensions_for_width(1080);
        assert_eq!(w, 1080);
        assert_eq!(h, 1920);
    }

    #[test]
    fn test_dimensions_for_height() {
        let (w, h) = AspectRatio::Horizontal16x9.dimensions_for_height(1080);
        assert_eq!(w, 1920);
        assert_eq!(h, 1080);

        let (w, h) = AspectRatio::Vertical9x16.dimensions_for_height(1920);
        assert_eq!(w, 1080);
        assert_eq!(h, 1920);
    }

    #[test]
    fn test_content_alignment_factors() {
        let center = ContentAlignment::Center;
        assert_eq!(center.horizontal_factor(), 0.5);
        assert_eq!(center.vertical_factor(), 0.5);

        let top_left = ContentAlignment::TopLeft;
        assert_eq!(top_left.horizontal_factor(), 0.0);
        assert_eq!(top_left.vertical_factor(), 0.0);

        let bottom_right = ContentAlignment::BottomRight;
        assert_eq!(bottom_right.horizontal_factor(), 1.0);
        assert_eq!(bottom_right.vertical_factor(), 1.0);
    }

    #[test]
    fn test_letterbox_info() {
        let info = LetterboxInfo::all(10);
        assert_eq!(info.top, 10);
        assert_eq!(info.right, 10);
        assert_eq!(info.bottom, 10);
        assert_eq!(info.left, 10);
        assert_eq!(info.horizontal(), 20);
        assert_eq!(info.vertical(), 20);

        let sym = LetterboxInfo::symmetric(5, 10);
        assert_eq!(sym.top, 5);
        assert_eq!(sym.right, 10);
        assert_eq!(sym.bottom, 5);
        assert_eq!(sym.left, 10);

        let zero = LetterboxInfo::zero();
        assert!(zero.is_empty());
        assert_eq!(zero.horizontal(), 0);
        assert_eq!(zero.vertical(), 0);
    }

    #[test]
    fn test_content_rect() {
        let info = LetterboxInfo::symmetric(10, 20);
        let (x, y, w, h) = info.content_rect(100, 100);
        assert_eq!(x, 20);
        assert_eq!(y, 10);
        assert_eq!(w, 60); // 100 - 20 - 20
        assert_eq!(h, 80); // 100 - 10 - 10
    }

    #[test]
    fn test_calculate_letterbox_letterbox() {
        // 16:9 content into 1:1 square (needs letterbox - top/bottom bars)
        let info = calculate_letterbox(1920, 1080, AspectRatio::Square, ContentAlignment::Center);
        assert_eq!(info.left, 0);
        assert_eq!(info.right, 0);
        assert_eq!(info.top, 420);
        assert_eq!(info.bottom, 420);

        // Same but aligned to top
        let info = calculate_letterbox(1920, 1080, AspectRatio::Square, ContentAlignment::Top);
        assert_eq!(info.top, 0);
        assert_eq!(info.bottom, 840);
    }

    #[test]
    fn test_calculate_letterbox_pillarbox() {
        // 1:1 content into 16:9 container (needs pillarbox - left/right bars)
        let info = calculate_letterbox(
            1080,
            1080,
            AspectRatio::Horizontal16x9,
            ContentAlignment::Center,
        );
        assert_eq!(info.top, 0);
        assert_eq!(info.bottom, 0);
        // 1080 * 9/16 = 607.5, so pillarbox of (1080 - 607.5) / 2 = ~236
        assert_eq!(info.left, 236);
        assert_eq!(info.right, 236);

        // Same but aligned to left
        let info = calculate_letterbox(
            1080,
            1080,
            AspectRatio::Horizontal16x9,
            ContentAlignment::Left,
        );
        assert_eq!(info.left, 0);
        assert_eq!(info.right, 472);
    }

    #[test]
    fn test_calculate_letterbox_no_padding() {
        // 16:9 content into 16:9 container (exact match)
        let info = calculate_letterbox(
            1920,
            1080,
            AspectRatio::Horizontal16x9,
            ContentAlignment::Center,
        );
        assert!(info.is_empty());
        assert_eq!(info.top, 0);
        assert_eq!(info.bottom, 0);
        assert_eq!(info.left, 0);
        assert_eq!(info.right, 0);
    }

    #[test]
    fn test_calculate_letterbox_zero_input() {
        let info = calculate_letterbox(0, 1080, AspectRatio::Square, ContentAlignment::Center);
        assert!(info.is_empty());

        let info = calculate_letterbox(1920, 0, AspectRatio::Square, ContentAlignment::Center);
        assert!(info.is_empty());
    }

    #[test]
    fn test_fit_to_container() {
        // Wider content
        let (w, h) = fit_to_container(1920, 1080, 500, 500);
        assert_eq!(w, 500);
        assert_eq!(h, 281); // 500 / (1920/1080) = 281.25

        // Taller content
        let (w, h) = fit_to_container(1080, 1920, 500, 500);
        assert_eq!(w, 281); // 500 * (1080/1920) = 281.25
        assert_eq!(h, 500);

        // Exact match
        let (w, h) = fit_to_container(1920, 1080, 1920, 1080);
        assert_eq!(w, 1920);
        assert_eq!(h, 1080);

        // Zero input
        let (w, h) = fit_to_container(0, 1080, 500, 500);
        assert_eq!(w, 0);
        assert_eq!(h, 0);
    }

    #[test]
    fn test_serialization() {
        // Test that AspectRatio serializes/deserializes correctly
        let ratio = AspectRatio::Horizontal16x9;
        let json = serde_json::to_string(&ratio).unwrap();
        let deserialized: AspectRatio = serde_json::from_str(&json).unwrap();
        assert_eq!(ratio, deserialized);

        // Test Custom
        let custom = AspectRatio::Custom(3, 2);
        let json = serde_json::to_string(&custom).unwrap();
        let deserialized: AspectRatio = serde_json::from_str(&json).unwrap();
        assert_eq!(custom, deserialized);

        // Test ContentAlignment
        let align = ContentAlignment::BottomRight;
        let json = serde_json::to_string(&align).unwrap();
        let deserialized: ContentAlignment = serde_json::from_str(&json).unwrap();
        assert_eq!(align, deserialized);

        // Test LetterboxInfo
        let info = LetterboxInfo::symmetric(10, 20);
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: LetterboxInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, deserialized);
    }

    #[test]
    fn test_default_aspect_ratio() {
        let default: AspectRatio = Default::default();
        assert_eq!(default, AspectRatio::Horizontal16x9);
    }

    #[test]
    fn test_default_content_alignment() {
        let default: ContentAlignment = Default::default();
        assert_eq!(default, ContentAlignment::Center);
    }
}
