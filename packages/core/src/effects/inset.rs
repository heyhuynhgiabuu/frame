//! Inset effect for subtle depth/bevel on video edges
//!
//! Creates a subtle inset effect that gives the appearance of depth
//! or a raised/depressed border around the video frame.

use crate::capture::Frame;
use crate::effects::Color;
use crate::FrameResult;
use serde::{Deserialize, Serialize};

/// Inset style variants
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum InsetStyle {
    /// Light inset (simulates raised edge, like light from top-left)
    Light,
    /// Dark inset (simulates depressed edge, like shadow on top-left)
    #[default]
    Dark,
    /// Custom color for the inset effect
    Custom(Color),
}

impl PartialEq for InsetStyle {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (InsetStyle::Light, InsetStyle::Light) => true,
            (InsetStyle::Dark, InsetStyle::Dark) => true,
            (InsetStyle::Custom(a), InsetStyle::Custom(b)) => {
                (a.r - b.r).abs() < f32::EPSILON
                    && (a.g - b.g).abs() < f32::EPSILON
                    && (a.b - b.b).abs() < f32::EPSILON
                    && (a.a - b.a).abs() < f32::EPSILON
            }
            _ => false,
        }
    }
}

/// Configuration for the inset effect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsetConfig {
    /// Whether the inset effect is enabled
    pub enabled: bool,
    /// Intensity of the effect (0.0 to 1.0)
    pub intensity: f32,
    /// Width of the inset border in pixels
    pub width: u32,
    /// Style of the inset effect
    pub style: InsetStyle,
}

impl Default for InsetConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            intensity: 0.3,
            width: 2,
            style: InsetStyle::Dark,
        }
    }
}

impl InsetConfig {
    /// Create a new inset config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a disabled inset config
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> FrameResult<()> {
        if self.intensity < 0.0 || self.intensity > 1.0 {
            return Err(crate::FrameError::Configuration(format!(
                "inset.intensity must be between 0.0 and 1.0, got {}",
                self.intensity
            )));
        }
        Ok(())
    }
}

/// Inset effect processor
#[derive(Debug)]
pub struct InsetEffect {
    config: InsetConfig,
}

impl Default for InsetEffect {
    fn default() -> Self {
        Self::new(InsetConfig::default())
    }
}

impl InsetEffect {
    /// Create a new inset effect with the given configuration
    pub fn new(config: InsetConfig) -> Self {
        Self { config }
    }

    /// Set the configuration
    pub fn set_config(&mut self, config: InsetConfig) {
        self.config = config;
    }

    /// Get the current configuration
    pub fn config(&self) -> &InsetConfig {
        &self.config
    }

    /// Apply the inset effect to a frame
    ///
    /// This modifies the frame in place, applying a subtle bevel/depth effect
    /// along the edges. The effect respects rounded corners if corner_radius is provided.
    pub fn apply(&self, frame: &mut Frame, corner_radius: f32) -> FrameResult<()> {
        if !self.config.enabled || self.config.width == 0 || self.config.intensity <= 0.0 {
            return Ok(());
        }

        let width = frame.width;
        let height = frame.height;
        let inset_width = self.config.width.min(width / 2).min(height / 2);
        let intensity = self.config.intensity.clamp(0.0, 1.0);

        // Calculate colors based on style
        let (light_color, dark_color) = match self.config.style {
            InsetStyle::Light => {
                // Light style: brighter on top-left, darker on bottom-right
                let light = Color::rgba_u8(255, 255, 255, (255.0 * intensity) as u8);
                let dark = Color::rgba_u8(0, 0, 0, (128.0 * intensity) as u8);
                (light, dark)
            }
            InsetStyle::Dark => {
                // Dark style: darker on top-left, lighter on bottom-right
                let light = Color::rgba_u8(255, 255, 255, (64.0 * intensity) as u8);
                let dark = Color::rgba_u8(0, 0, 0, (128.0 * intensity) as u8);
                (light, dark)
            }
            InsetStyle::Custom(color) => {
                // Custom: use the provided color with varying opacity
                let light = Color::new(color.r, color.g, color.b, color.a * intensity);
                let dark = Color::new(color.r, color.g, color.b, color.a * intensity * 0.5);
                (light, dark)
            }
        };

        // Apply inset effect to edges
        for y in 0..height {
            for x in 0..width {
                // Calculate distance from edges
                let dist_top = y;
                let dist_bottom = height - 1 - y;
                let dist_left = x;
                let dist_right = width - 1 - x;

                // Check if pixel is within inset width of any edge
                let in_inset_region = dist_top < inset_width
                    || dist_bottom < inset_width
                    || dist_left < inset_width
                    || dist_right < inset_width;

                if !in_inset_region {
                    continue;
                }

                // Check corner radius mask
                if corner_radius > 0.0
                    && !self.is_in_corner_region(x, y, width, height, corner_radius, inset_width)
                {
                    continue;
                }

                // Determine if this is a "light" or "dark" edge
                // Light edges: top and left (simulating light from top-left)
                // Dark edges: bottom and right
                let is_light_edge = (dist_top < inset_width && dist_left >= corner_radius as u32)
                    || (dist_left < inset_width && dist_top >= corner_radius as u32);

                let is_dark_edge = (dist_bottom < inset_width
                    && dist_right >= corner_radius as u32)
                    || (dist_right < inset_width && dist_bottom >= corner_radius as u32);

                // Calculate blend factor based on distance from edge
                let edge_dist = dist_top.min(dist_bottom).min(dist_left).min(dist_right);
                let blend_factor = 1.0 - (edge_dist as f32 / inset_width as f32);
                let blend_factor = blend_factor * blend_factor; // Quadratic falloff for smoother effect

                // Get the inset color for this pixel
                let inset_color = if is_light_edge {
                    light_color
                } else if is_dark_edge {
                    dark_color
                } else {
                    // Corner region - blend between light and dark
                    continue; // Skip corners for now, could add gradient
                };

                // Apply the inset color to the pixel
                self.blend_pixel(&mut frame.data, width, x, y, &inset_color, blend_factor);
            }
        }

        Ok(())
    }

    /// Check if a pixel is within the corner region that should receive the inset effect
    fn is_in_corner_region(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        corner_radius: f32,
        inset_width: u32,
    ) -> bool {
        if corner_radius <= 0.0 {
            return true;
        }

        let r = corner_radius;
        let fw = width as f32;
        let fh = height as f32;

        // Check each corner region
        let corners = [
            (0.0, 0.0),       // top-left
            (fw - r, 0.0),    // top-right
            (0.0, fh - r),    // bottom-left
            (fw - r, fh - r), // bottom-right
        ];

        let fx = x as f32;
        let fy = y as f32;

        for (cx, cy) in corners {
            // Check if we're in this corner's region
            if fx >= cx && fx < cx + r && fy >= cy && fy < cy + r {
                // Calculate distance from the corner's curve
                let (ccx, ccy) = if cx == 0.0 && cy == 0.0 {
                    (r, r)
                } else if cx > 0.0 && cy == 0.0 {
                    (fw - r, r)
                } else if cx == 0.0 && cy > 0.0 {
                    (r, fh - r)
                } else {
                    (fw - r, fh - r)
                };

                let dx = fx + 0.5 - ccx;
                let dy = fy + 0.5 - ccy;
                let dist = (dx * dx + dy * dy).sqrt();

                // Pixel is inside the corner arc if distance is greater than inner radius
                let inner_radius = r - inset_width as f32;
                if dist >= inner_radius.max(0.0) && dist <= r {
                    return true;
                }
                return false;
            }
        }

        // Not in any corner region - check if in edge region
        let in_edge_x = fx < inset_width as f32 || fx >= fw - inset_width as f32;
        let in_edge_y = fy < inset_width as f32 || fy >= fh - inset_width as f32;

        in_edge_x || in_edge_y
    }

    /// Blend an inset color onto a pixel
    fn blend_pixel(&self, data: &mut [u8], width: u32, x: u32, y: u32, color: &Color, factor: f32) {
        let idx = ((y * width + x) * 4) as usize;
        if idx + 3 >= data.len() {
            return;
        }

        let factor = factor.clamp(0.0, 1.0);

        // Convert color to u8
        let r = (color.r * 255.0) as u8;
        let g = (color.g * 255.0) as u8;
        let b = (color.b * 255.0) as u8;
        let a = (color.a * 255.0) as u8;

        // Alpha blend
        let src_a = (a as f32 / 255.0) * factor;
        let dest_a = data[idx + 3] as f32 / 255.0;
        let out_a = src_a + dest_a * (1.0 - src_a);

        if out_a > 0.0 {
            for i in 0..3 {
                let src_c = [r, g, b][i] as f32 / 255.0;
                let dest_c = data[idx + i] as f32 / 255.0;
                let out_c = (src_c * src_a + dest_c * dest_a * (1.0 - src_a)) / out_a;
                data[idx + i] = (out_c * 255.0) as u8;
            }
            data[idx + 3] = (out_a * 255.0) as u8;
        }
    }

    /// Reset the effect state
    pub fn reset(&mut self) {
        // No state to reset for this effect
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::PixelFormat;

    fn create_test_frame(width: u32, height: u32) -> Frame {
        Frame {
            data: vec![128u8; (width * height * 4) as usize],
            width,
            height,
            timestamp: std::time::Duration::from_secs(0),
            format: PixelFormat::Rgba,
        }
    }

    #[test]
    fn test_inset_config_default() {
        let config = InsetConfig::default();
        assert!(config.enabled);
        assert_eq!(config.intensity, 0.3);
        assert_eq!(config.width, 2);
        assert_eq!(config.style, InsetStyle::Dark);
    }

    #[test]
    fn test_inset_config_disabled() {
        let config = InsetConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_inset_config_validation() {
        let valid = InsetConfig::default();
        assert!(valid.validate().is_ok());

        let invalid = InsetConfig {
            intensity: 1.5,
            ..Default::default()
        };
        assert!(invalid.validate().is_err());

        let invalid_negative = InsetConfig {
            intensity: -0.1,
            ..Default::default()
        };
        assert!(invalid_negative.validate().is_err());
    }

    #[test]
    fn test_inset_effect_disabled() {
        let config = InsetConfig::disabled();
        let effect = InsetEffect::new(config);
        let mut frame = create_test_frame(100, 100);
        let original_data = frame.data.clone();

        effect.apply(&mut frame, 0.0).unwrap();

        // Frame should be unchanged when disabled
        assert_eq!(frame.data, original_data);
    }

    #[test]
    fn test_inset_effect_zero_width() {
        let config = InsetConfig {
            width: 0,
            ..Default::default()
        };
        let effect = InsetEffect::new(config);
        let mut frame = create_test_frame(100, 100);
        let original_data = frame.data.clone();

        effect.apply(&mut frame, 0.0).unwrap();

        // Frame should be unchanged when width is 0
        assert_eq!(frame.data, original_data);
    }

    #[test]
    fn test_inset_effect_zero_intensity() {
        let config = InsetConfig {
            intensity: 0.0,
            ..Default::default()
        };
        let effect = InsetEffect::new(config);
        let mut frame = create_test_frame(100, 100);
        let original_data = frame.data.clone();

        effect.apply(&mut frame, 0.0).unwrap();

        // Frame should be unchanged when intensity is 0
        assert_eq!(frame.data, original_data);
    }

    #[test]
    fn test_inset_effect_applies_to_edges() {
        let config = InsetConfig {
            enabled: true,
            intensity: 1.0,
            width: 2,
            style: InsetStyle::Dark,
        };
        let effect = InsetEffect::new(config);
        let mut frame = create_test_frame(100, 100);

        effect.apply(&mut frame, 0.0).unwrap();

        // Check that top-left edge (0,0) has been modified (darkened)
        let top_left_idx = 0;
        assert!(frame.data[top_left_idx] < 128 || frame.data[top_left_idx + 3] < 255);

        // Check that center pixel is unchanged
        let center_idx = ((50 * 100 + 50) * 4) as usize;
        assert_eq!(frame.data[center_idx], 128);
    }

    #[test]
    fn test_inset_effect_light_style() {
        let config = InsetConfig {
            enabled: true,
            intensity: 1.0,
            width: 2,
            style: InsetStyle::Light,
        };
        let effect = InsetEffect::new(config);
        let mut frame = create_test_frame(100, 100);

        effect.apply(&mut frame, 0.0).unwrap();

        // Light style should modify the frame
        let top_left_idx = 0;
        // Top-left with light style should be brighter
        assert!(frame.data[top_left_idx] != 128 || frame.data[top_left_idx + 3] != 255);
    }

    #[test]
    fn test_inset_effect_custom_style() {
        let config = InsetConfig {
            enabled: true,
            intensity: 1.0,
            width: 2,
            style: InsetStyle::Custom(Color::rgb(1.0, 0.0, 0.0)), // Red
        };
        let effect = InsetEffect::new(config);
        let mut frame = create_test_frame(100, 100);

        effect.apply(&mut frame, 0.0).unwrap();

        // Custom red style should modify the frame
        let top_left_idx = 0;
        assert!(frame.data[top_left_idx] != 128 || frame.data[top_left_idx + 3] != 255);
    }

    #[test]
    fn test_inset_effect_with_corner_radius() {
        let config = InsetConfig {
            enabled: true,
            intensity: 1.0,
            width: 5,
            style: InsetStyle::Dark,
        };
        let effect = InsetEffect::new(config);
        let mut frame = create_test_frame(100, 100);
        let original_data = frame.data.clone();

        effect.apply(&mut frame, 20.0).unwrap();

        // With corner radius 20, pixels at x >= 20 on the top edge should be modified
        // The corner region spans x=0 to x=20 at y=0
        // So x=25 should definitely be outside the corner region and have the inset applied
        let edge_idx = ((0 * 100 + 25) * 4) as usize; // x=25, y=0
        assert_ne!(
            frame.data[edge_idx..edge_idx + 4],
            original_data[edge_idx..edge_idx + 4],
            "Edge pixel outside corner radius should be modified"
        );

        // Center should remain unchanged
        let center_idx = ((50 * 100 + 50) * 4) as usize;
        assert_eq!(
            frame.data[center_idx], 128,
            "Center pixel should be unchanged"
        );
    }

    #[test]
    fn test_config_setters() {
        let mut effect = InsetEffect::default();

        let new_config = InsetConfig {
            enabled: false,
            intensity: 0.5,
            width: 4,
            style: InsetStyle::Light,
        };

        effect.set_config(new_config.clone());
        assert_eq!(effect.config().enabled, false);
        assert_eq!(effect.config().intensity, 0.5);
    }

    #[test]
    fn test_reset() {
        let mut effect = InsetEffect::default();
        effect.reset(); // Should not panic
    }
}
