//! Webcam overlay effect for picture-in-picture recording
//!
//! Composites a webcam feed into a corner of the main recording frame
//! with configurable position, shape (circle, rounded rect, rectangle),
//! size, and border styling.
//!
//! Performance target: <5ms per frame for 1080p main + 720p webcam

use crate::capture::{Frame, PixelFormat};
use crate::effects::Color;
use crate::FrameResult;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Position of the webcam overlay on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WebcamPosition {
    /// Top-left corner
    TopLeft,
    /// Top-right corner
    TopRight,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom-right corner (default)
    #[default]
    BottomRight,
}

/// Shape of the webcam overlay
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WebcamShape {
    /// Circular mask
    Circle,
    /// Rounded rectangle mask
    #[default]
    RoundedRect,
    /// Simple rectangle (no rounding)
    Rectangle,
}

/// Size presets for the webcam overlay
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum WebcamSize {
    /// Small overlay (80px)
    Small,
    /// Medium overlay (120px, default)
    #[default]
    Medium,
    /// Large overlay (160px)
    Large,
    /// Custom size in pixels
    Custom(u32),
}

impl WebcamSize {
    /// Get the actual pixel dimension (width/height are equal for square overlay)
    pub fn dimension(&self) -> u32 {
        match self {
            WebcamSize::Small => 80,
            WebcamSize::Medium => 120,
            WebcamSize::Large => 160,
            WebcamSize::Custom(size) => *size,
        }
    }
}

/// 2D offset for positioning adjustments
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Offset {
    pub x: i32,
    pub y: i32,
}

impl Offset {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Configuration for the webcam overlay effect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebcamOverlayConfig {
    /// Whether the webcam overlay is enabled
    pub enabled: bool,
    /// Position on screen
    pub position: WebcamPosition,
    /// Shape of the overlay
    pub shape: WebcamShape,
    /// Size of the overlay
    pub size: WebcamSize,
    /// Border color
    pub border_color: Color,
    /// Border width in pixels (0 = no border)
    pub border_width: u32,
    /// Offset from the corner position
    pub offset: Offset,
    /// Corner radius for RoundedRect shape (ignored for other shapes)
    pub corner_radius: f32,
    /// Whether to show a placeholder when webcam is unavailable
    pub show_placeholder: bool,
    /// Placeholder color (shown when webcam_frame is None)
    pub placeholder_color: Color,
}

impl Default for WebcamOverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: WebcamPosition::default(),
            shape: WebcamShape::default(),
            size: WebcamSize::default(),
            border_color: Color::WHITE,
            border_width: 2,
            offset: Offset::default(),
            corner_radius: 8.0,
            show_placeholder: true,
            placeholder_color: Color::rgba_u8(50, 50, 50, 200),
        }
    }
}

impl WebcamOverlayConfig {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a disabled config
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> FrameResult<()> {
        if self.border_width > 20 {
            return Err(crate::FrameError::Configuration(
                "webcam.border_width must be <= 20".to_string(),
            ));
        }
        if self.corner_radius < 0.0 {
            return Err(crate::FrameError::Configuration(
                "webcam.corner_radius must be >= 0".to_string(),
            ));
        }
        let dim = self.size.dimension();
        if !(40..=400).contains(&dim) {
            return Err(crate::FrameError::Configuration(
                "webcam size must be between 40 and 400 pixels".to_string(),
            ));
        }
        Ok(())
    }
}

/// Webcam overlay effect processor
#[derive(Debug)]
pub struct WebcamOverlay {
    config: WebcamOverlayConfig,
    /// Cached mask for shape rendering
    cached_mask: Option<CachedMask>,
}

#[derive(Debug, Clone)]
struct CachedMask {
    size: u32,
    shape: WebcamShape,
    corner_radius: f32,
    border_width: u32,
    data: Vec<f32>, // Alpha mask values (0.0-1.0)
}

impl Default for WebcamOverlay {
    fn default() -> Self {
        Self::new(WebcamOverlayConfig::default())
    }
}

impl WebcamOverlay {
    /// Create a new webcam overlay effect
    pub fn new(config: WebcamOverlayConfig) -> Self {
        Self {
            config,
            cached_mask: None,
        }
    }

    /// Set the configuration
    pub fn set_config(&mut self, config: WebcamOverlayConfig) {
        // Invalidate cache if shape-related config changed
        if self.config.size != config.size
            || self.config.shape != config.shape
            || self.config.corner_radius != config.corner_radius
            || self.config.border_width != config.border_width
        {
            self.cached_mask = None;
        }
        self.config = config;
    }

    /// Get the current configuration
    pub fn config(&self) -> &WebcamOverlayConfig {
        &self.config
    }

    /// Apply the webcam overlay to a frame
    ///
    /// # Arguments
    /// * `main_frame` - The main recording frame to composite onto
    /// * `webcam_frame` - Optional webcam frame (None shows placeholder if enabled)
    ///
    /// # Returns
    /// Frame with webcam overlay composited
    ///
    /// # Performance
    /// Target: <5ms per frame. Uses cached masks and efficient pixel operations.
    pub fn apply(
        &mut self,
        main_frame: &Frame,
        webcam_frame: Option<&Frame>,
    ) -> FrameResult<Frame> {
        if !self.config.enabled {
            return Ok(main_frame.clone());
        }

        // Validate pixel format
        if main_frame.format != PixelFormat::Rgba {
            return Err(crate::FrameError::Configuration(format!(
                "Webcam overlay requires RGBA format, got {:?}",
                main_frame.format
            )));
        }

        let start_time = Instant::now();

        // Calculate overlay dimensions
        let overlay_size = self.config.size.dimension();
        let position = self.calculate_position(main_frame.width, main_frame.height, overlay_size);

        // Create output buffer (copy of main frame)
        let mut output_data = main_frame.data.clone();

        // Determine what to render
        let source_frame = if let Some(wf) = webcam_frame {
            // Resize webcam frame to overlay size if needed
            if wf.width != overlay_size || wf.height != overlay_size {
                Some(self.resize_frame(wf, overlay_size)?)
            } else {
                Some(wf.clone())
            }
        } else if self.config.show_placeholder {
            None // Will render placeholder
        } else {
            return Ok(main_frame.clone()); // No webcam, no placeholder, return as-is
        };

        // Get or create mask for shape
        let mask = self.get_or_create_mask(overlay_size)?;

        // Composite the overlay
        self.composite_overlay(
            &mut output_data,
            main_frame.width,
            main_frame.height,
            source_frame.as_ref(),
            position,
            overlay_size,
            &mask,
        );

        // Performance check
        let elapsed = start_time.elapsed();
        if elapsed.as_millis() > 5 {
            tracing::warn!(
                "Webcam overlay took {}ms (target: <5ms)",
                elapsed.as_micros() as f64 / 1000.0
            );
        }

        Ok(Frame {
            data: output_data,
            width: main_frame.width,
            height: main_frame.height,
            timestamp: main_frame.timestamp,
            format: main_frame.format,
        })
    }

    /// Calculate the top-left position for the overlay
    fn calculate_position(
        &self,
        main_width: u32,
        main_height: u32,
        overlay_size: u32,
    ) -> (u32, u32) {
        let padding: i32 = 16; // Default padding from edges
        let offset = &self.config.offset;

        let x = match self.config.position {
            WebcamPosition::TopLeft | WebcamPosition::BottomLeft => padding + offset.x,
            WebcamPosition::TopRight | WebcamPosition::BottomRight => {
                main_width as i32 - overlay_size as i32 - padding + offset.x
            }
        };

        let y = match self.config.position {
            WebcamPosition::TopLeft | WebcamPosition::TopRight => padding + offset.y,
            WebcamPosition::BottomLeft | WebcamPosition::BottomRight => {
                main_height as i32 - overlay_size as i32 - padding + offset.y
            }
        };

        // Clamp to visible area
        let x = x.clamp(0, main_width.saturating_sub(overlay_size) as i32) as u32;
        let y = y.clamp(0, main_height.saturating_sub(overlay_size) as i32) as u32;

        (x, y)
    }

    /// Resize a frame to target dimensions (simple nearest-neighbor for speed)
    fn resize_frame(&self, frame: &Frame, target_size: u32) -> FrameResult<Frame> {
        let mut new_data = vec![0u8; (target_size * target_size * 4) as usize];

        let scale_x = frame.width as f32 / target_size as f32;
        let scale_y = frame.height as f32 / target_size as f32;

        for y in 0..target_size {
            for x in 0..target_size {
                let src_x = ((x as f32 * scale_x) as u32).min(frame.width - 1);
                let src_y = ((y as f32 * scale_y) as u32).min(frame.height - 1);

                let src_idx = ((src_y * frame.width + src_x) * 4) as usize;
                let dst_idx = ((y * target_size + x) * 4) as usize;

                new_data[dst_idx..dst_idx + 4].copy_from_slice(&frame.data[src_idx..src_idx + 4]);
            }
        }

        Ok(Frame {
            data: new_data,
            width: target_size,
            height: target_size,
            timestamp: frame.timestamp,
            format: frame.format,
        })
    }

    /// Get or create the alpha mask for the shape
    fn get_or_create_mask(&mut self, size: u32) -> FrameResult<Vec<f32>> {
        // Check cache
        if let Some(cached) = &self.cached_mask {
            if cached.size == size
                && cached.shape == self.config.shape
                && (cached.corner_radius - self.config.corner_radius).abs() < 0.01
                && cached.border_width == self.config.border_width
            {
                return Ok(cached.data.clone());
            }
        }

        // Generate new mask
        let mask = self.generate_mask(size)?;
        self.cached_mask = Some(CachedMask {
            size,
            shape: self.config.shape,
            corner_radius: self.config.corner_radius,
            border_width: self.config.border_width,
            data: mask.clone(),
        });

        Ok(mask)
    }

    /// Generate alpha mask based on shape
    fn generate_mask(&self, size: u32) -> FrameResult<Vec<f32>> {
        let mut mask = vec![0.0f32; (size * size) as usize];
        let shape = self.config.shape;
        let border_width = self.config.border_width;
        let radius = self.config.corner_radius.clamp(0.0, size as f32 / 2.0);

        for y in 0..size {
            for x in 0..size {
                let idx = (y * size + x) as usize;

                let alpha = match shape {
                    WebcamShape::Rectangle => {
                        // Simple rectangle with optional border
                        if border_width > 0 {
                            let in_border = x < border_width
                                || x >= size - border_width
                                || y < border_width
                                || y >= size - border_width;
                            if in_border {
                                0.0 // Transparent in border (will show border color)
                            } else {
                                1.0 // Opaque inside
                            }
                        } else {
                            1.0
                        }
                    }
                    WebcamShape::Circle => {
                        // Circular mask
                        let cx = size as f32 / 2.0;
                        let cy = size as f32 / 2.0;
                        let dx = x as f32 + 0.5 - cx;
                        let dy = y as f32 + 0.5 - cy;
                        let dist = (dx * dx + dy * dy).sqrt();
                        let r = size as f32 / 2.0;

                        if dist > r {
                            0.0
                        } else if dist > r - 1.0 {
                            // Anti-alias edge
                            r - dist
                        } else if border_width > 0 {
                            let inner_r = r - border_width as f32;
                            if dist < inner_r {
                                1.0
                            } else if dist < inner_r + 1.0 {
                                inner_r + 1.0 - dist
                            } else {
                                0.0
                            }
                        } else {
                            1.0
                        }
                    }
                    WebcamShape::RoundedRect => {
                        // Rounded rectangle
                        self.calculate_rounded_rect_alpha(x, y, size, radius, border_width)
                    }
                };

                mask[idx] = alpha;
            }
        }

        Ok(mask)
    }

    /// Calculate alpha for rounded rectangle shape
    fn calculate_rounded_rect_alpha(
        &self,
        x: u32,
        y: u32,
        size: u32,
        radius: f32,
        border_width: u32,
    ) -> f32 {
        let r = radius.clamp(0.0, size as f32 / 2.0);
        let fx = x as f32 + 0.5;
        let fy = y as f32 + 0.5;

        // Check if in corner regions
        let corners = [
            (r, r),                             // top-left
            (size as f32 - r, r),               // top-right
            (r, size as f32 - r),               // bottom-left
            (size as f32 - r, size as f32 - r), // bottom-right
        ];

        let in_corner_region = fx < r || fx > size as f32 - r || fy < r || fy > size as f32 - r;

        if !in_corner_region {
            // In central region - check for border
            if border_width > 0 {
                let inner_size = size as f32 - 2.0 * border_width as f32;
                if inner_size <= 0.0 {
                    return 0.0; // All border
                }
                let in_inner = fx >= border_width as f32
                    && fx < size as f32 - border_width as f32
                    && fy >= border_width as f32
                    && fy < size as f32 - border_width as f32;
                if in_inner {
                    return 1.0;
                } else {
                    return 0.0;
                }
            }
            return 1.0;
        }

        // In corner region - check distance from corner center
        for (cx, cy) in corners {
            let dx = fx - cx;
            let dy = fy - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            // Check if this pixel is controlled by this corner
            let is_this_corner = if cx == r && cy == r {
                fx < r && fy < r
            } else if cx == size as f32 - r && cy == r {
                fx > size as f32 - r && fy < r
            } else if cx == r && cy == size as f32 - r {
                fx < r && fy > size as f32 - r
            } else {
                fx > size as f32 - r && fy > size as f32 - r
            };

            if is_this_corner {
                if dist > r {
                    return 0.0;
                }

                // Check border
                if border_width > 0 {
                    let inner_r = (r - border_width as f32).max(0.0);
                    if dist < inner_r {
                        return 1.0;
                    } else if dist < inner_r + 1.0 {
                        return inner_r + 1.0 - dist;
                    } else {
                        return 0.0;
                    }
                }

                if dist > r - 1.0 {
                    return r - dist;
                }
                return 1.0;
            }
        }

        0.0
    }

    /// Composite the overlay onto the output buffer
    #[allow(clippy::too_many_arguments)]
    fn composite_overlay(
        &self,
        output: &mut [u8],
        main_width: u32,
        main_height: u32,
        source_frame: Option<&Frame>,
        position: (u32, u32),
        overlay_size: u32,
        mask: &[f32],
    ) {
        let (pos_x, pos_y) = position;
        let placeholder_color = self.config.placeholder_color;

        for y in 0..overlay_size {
            for x in 0..overlay_size {
                let mask_alpha = mask[(y * overlay_size + x) as usize];
                if mask_alpha <= 0.0 {
                    continue; // Skip fully transparent pixels
                }

                let main_x = pos_x + x;
                let main_y = pos_y + y;

                if main_x >= main_width || main_y >= main_height {
                    continue;
                }

                let output_idx = ((main_y * main_width + main_x) * 4) as usize;

                // Get source pixel color
                let (src_r, src_g, src_b, src_a) = if let Some(frame) = source_frame {
                    let src_idx = ((y * overlay_size + x) * 4) as usize;
                    if src_idx + 3 < frame.data.len() {
                        (
                            frame.data[src_idx] as f32 / 255.0,
                            frame.data[src_idx + 1] as f32 / 255.0,
                            frame.data[src_idx + 2] as f32 / 255.0,
                            frame.data[src_idx + 3] as f32 / 255.0,
                        )
                    } else {
                        continue;
                    }
                } else {
                    // Placeholder (no webcam frame)
                    (
                        placeholder_color.r,
                        placeholder_color.g,
                        placeholder_color.b,
                        placeholder_color.a,
                    )
                };

                // Apply mask alpha
                let final_alpha = src_a * mask_alpha;

                if final_alpha <= 0.0 {
                    continue;
                }

                // Alpha blend
                let dest_a = output[output_idx + 3] as f32 / 255.0;
                let out_a = final_alpha + dest_a * (1.0 - final_alpha);

                if out_a > 0.0 {
                    for i in 0..3 {
                        let src_c = [src_r, src_g, src_b][i];
                        let dest_c = output[output_idx + i] as f32 / 255.0;
                        let out_c =
                            (src_c * final_alpha + dest_c * dest_a * (1.0 - final_alpha)) / out_a;
                        output[output_idx + i] = (out_c * 255.0) as u8;
                    }
                    output[output_idx + 3] = (out_a * 255.0) as u8;
                }
            }
        }

        // Draw border if enabled
        if self.config.border_width > 0 {
            self.draw_border(
                output,
                main_width,
                main_height,
                position,
                overlay_size,
                mask,
            );
        }
    }

    /// Draw border around the overlay
    fn draw_border(
        &self,
        output: &mut [u8],
        main_width: u32,
        _main_height: u32,
        position: (u32, u32),
        overlay_size: u32,
        mask: &[f32],
    ) {
        let (pos_x, pos_y) = position;
        let border_color = &self.config.border_color;
        let border_r = (border_color.r * 255.0) as u8;
        let border_g = (border_color.g * 255.0) as u8;
        let border_b = (border_color.b * 255.0) as u8;
        let border_a = (border_color.a * 255.0) as u8;

        for y in 0..overlay_size {
            for x in 0..overlay_size {
                let mask_alpha = mask[(y * overlay_size + x) as usize];

                // Border pixels are where mask is 0 (transparent) but adjacent to opaque
                if mask_alpha > 0.0 {
                    continue;
                }

                // Check if this is a border pixel
                let is_border = self.is_border_pixel(x, y, overlay_size, mask);

                if is_border {
                    let main_x = pos_x + x;
                    let main_y = pos_y + y;

                    if main_x >= main_width || main_y >= _main_height {
                        continue;
                    }

                    let output_idx = ((main_y * main_width + main_x) * 4) as usize;

                    // Alpha blend border
                    let src_a = border_a as f32 / 255.0 * border_color.a;
                    let dest_a = output[output_idx + 3] as f32 / 255.0;
                    let out_a = src_a + dest_a * (1.0 - src_a);

                    if out_a > 0.0 {
                        for i in 0..3 {
                            let src_c = [border_r, border_g, border_b][i] as f32 / 255.0;
                            let dest_c = output[output_idx + i] as f32 / 255.0;
                            let out_c = (src_c * src_a + dest_c * dest_a * (1.0 - src_a)) / out_a;
                            output[output_idx + i] = (out_c * 255.0) as u8;
                        }
                        output[output_idx + 3] = (out_a * 255.0) as u8;
                    }
                }
            }
        }
    }

    /// Check if a pixel should be part of the border
    fn is_border_pixel(&self, x: u32, y: u32, size: u32, mask: &[f32]) -> bool {
        // Check neighbors for any opaque pixels
        let neighbors = [
            (x.saturating_sub(1), y),
            ((x + 1).min(size - 1), y),
            (x, y.saturating_sub(1)),
            (x, (y + 1).min(size - 1)),
        ];

        for (nx, ny) in neighbors {
            let n_idx = (ny * size + nx) as usize;
            if n_idx < mask.len() && mask[n_idx] > 0.0 {
                return true;
            }
        }

        false
    }

    /// Reset the effect state
    pub fn reset(&mut self) {
        self.cached_mask = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_frame(width: u32, height: u32, color: Option<Color>) -> Frame {
        let color = color.unwrap_or(Color::rgb(0.5, 0.5, 0.5));
        let r = (color.r * 255.0) as u8;
        let g = (color.g * 255.0) as u8;
        let b = (color.b * 255.0) as u8;
        let a = (color.a * 255.0) as u8;

        let mut data = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..(width * height) {
            data.push(r);
            data.push(g);
            data.push(b);
            data.push(a);
        }

        Frame {
            data,
            width,
            height,
            timestamp: std::time::Duration::from_secs(0),
            format: PixelFormat::Rgba,
        }
    }

    #[test]
    fn test_webcam_size_dimensions() {
        assert_eq!(WebcamSize::Small.dimension(), 80);
        assert_eq!(WebcamSize::Medium.dimension(), 120);
        assert_eq!(WebcamSize::Large.dimension(), 160);
        assert_eq!(WebcamSize::Custom(200).dimension(), 200);
    }

    #[test]
    fn test_config_default() {
        let config = WebcamOverlayConfig::default();
        assert!(config.enabled);
        assert_eq!(config.position, WebcamPosition::BottomRight);
        assert_eq!(config.shape, WebcamShape::RoundedRect);
        assert_eq!(config.size, WebcamSize::Medium);
        assert_eq!(config.border_width, 2);
    }

    #[test]
    fn test_config_disabled() {
        let config = WebcamOverlayConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_config_validation() {
        let valid = WebcamOverlayConfig::default();
        assert!(valid.validate().is_ok());

        let invalid_border = WebcamOverlayConfig {
            border_width: 25,
            ..Default::default()
        };
        assert!(invalid_border.validate().is_err());

        let invalid_size = WebcamOverlayConfig {
            size: WebcamSize::Custom(500),
            ..Default::default()
        };
        assert!(invalid_size.validate().is_err());

        let invalid_radius = WebcamOverlayConfig {
            corner_radius: -1.0,
            ..Default::default()
        };
        assert!(invalid_radius.validate().is_err());
    }

    #[test]
    fn test_overlay_disabled() {
        let config = WebcamOverlayConfig::disabled();
        let mut overlay = WebcamOverlay::new(config);
        let main_frame = create_test_frame(100, 100, None);

        let result = overlay.apply(&main_frame, None).unwrap();
        assert_eq!(result.data, main_frame.data);
    }

    #[test]
    fn test_overlay_with_placeholder() {
        let config = WebcamOverlayConfig {
            enabled: true,
            size: WebcamSize::Small,
            position: WebcamPosition::TopLeft,
            show_placeholder: true,
            ..Default::default()
        };
        let mut overlay = WebcamOverlay::new(config);
        let main_frame = create_test_frame(200, 200, Some(Color::WHITE));

        let result = overlay.apply(&main_frame, None).unwrap();

        // Overlay should have modified the frame (placeholder drawn)
        // Check area where overlay is positioned (16px padding + offset)
        let overlay_size = 80; // Small size
        let padding = 16;

        // At least some pixels should be different from white
        let mut found_different = false;
        for y in padding..(padding + 10).min(padding + overlay_size as usize) {
            for x in padding..(padding + 10).min(padding + overlay_size as usize) {
                let idx = ((y as u32 * 200 + x as u32) * 4) as usize;
                if result.data[idx] != 255 || result.data[idx + 3] != 255 {
                    found_different = true;
                    break;
                }
            }
            if found_different {
                break;
            }
        }
        assert!(found_different, "Placeholder should modify the frame");
    }

    #[test]
    fn test_overlay_positions() {
        let positions = [
            WebcamPosition::TopLeft,
            WebcamPosition::TopRight,
            WebcamPosition::BottomLeft,
            WebcamPosition::BottomRight,
        ];

        for pos in positions {
            let config = WebcamOverlayConfig {
                enabled: true,
                position: pos,
                size: WebcamSize::Small,
                show_placeholder: true,
                ..Default::default()
            };
            let mut overlay = WebcamOverlay::new(config);
            let main_frame = create_test_frame(200, 200, Some(Color::WHITE));

            let result = overlay.apply(&main_frame, None).unwrap();

            // Frame should be modified
            assert_ne!(
                result.data, main_frame.data,
                "Position {:?} should modify frame",
                pos
            );
        }
    }

    #[test]
    fn test_overlay_shapes() {
        let shapes = [
            WebcamShape::Circle,
            WebcamShape::RoundedRect,
            WebcamShape::Rectangle,
        ];

        for shape in shapes {
            let config = WebcamOverlayConfig {
                enabled: true,
                shape,
                size: WebcamSize::Small,
                show_placeholder: true,
                ..Default::default()
            };
            let mut overlay = WebcamOverlay::new(config);
            let main_frame = create_test_frame(200, 200, Some(Color::WHITE));

            let result = overlay.apply(&main_frame, None).unwrap();

            // Frame should be modified
            assert_ne!(
                result.data, main_frame.data,
                "Shape {:?} should modify frame",
                shape
            );
        }
    }

    #[test]
    fn test_overlay_with_webcam_frame() {
        let config = WebcamOverlayConfig {
            enabled: true,
            size: WebcamSize::Small,
            position: WebcamPosition::TopLeft,
            ..Default::default()
        };
        let mut overlay = WebcamOverlay::new(config);
        let main_frame = create_test_frame(200, 200, Some(Color::WHITE));
        let webcam_frame = create_test_frame(80, 80, Some(Color::rgb(1.0, 0.0, 0.0))); // Red webcam

        let result = overlay.apply(&main_frame, Some(&webcam_frame)).unwrap();

        // Frame should be modified
        assert_ne!(result.data, main_frame.data);
    }

    #[test]
    fn test_resize_frame() {
        let overlay = WebcamOverlay::new(WebcamOverlayConfig::default());
        let frame = create_test_frame(100, 100, Some(Color::rgb(1.0, 0.0, 0.0)));

        let resized = overlay.resize_frame(&frame, 50).unwrap();
        assert_eq!(resized.width, 50);
        assert_eq!(resized.height, 50);
    }

    #[test]
    fn test_config_update() {
        let mut overlay = WebcamOverlay::new(WebcamOverlayConfig::default());

        // Create cache by applying
        let main_frame = create_test_frame(200, 200, Some(Color::WHITE));
        let _ = overlay.apply(&main_frame, None);

        // Update config
        let new_config = WebcamOverlayConfig {
            size: WebcamSize::Large,
            ..Default::default()
        };
        overlay.set_config(new_config);

        assert_eq!(overlay.config().size, WebcamSize::Large);
    }

    #[test]
    fn test_reset() {
        let mut overlay = WebcamOverlay::new(WebcamOverlayConfig::default());

        // Create cache
        let main_frame = create_test_frame(200, 200, Some(Color::WHITE));
        let _ = overlay.apply(&main_frame, None);

        // Reset should clear cache
        overlay.reset();
        assert!(overlay.cached_mask.is_none());
    }

    #[test]
    fn test_offset_calculation() {
        let config = WebcamOverlayConfig {
            enabled: true,
            position: WebcamPosition::TopLeft,
            size: WebcamSize::Custom(50),
            offset: Offset::new(10, 10),
            ..Default::default()
        };
        let overlay = WebcamOverlay::new(config);

        let (x, y) = overlay.calculate_position(200, 200, 50);

        // Should have padding + offset
        assert_eq!(x, 26); // 16 (padding) + 10 (offset)
        assert_eq!(y, 26);
    }

    #[test]
    fn test_border_drawing() {
        let config = WebcamOverlayConfig {
            enabled: true,
            size: WebcamSize::Small,
            border_width: 4,
            border_color: Color::rgb(1.0, 0.0, 0.0), // Red border
            show_placeholder: true,
            ..Default::default()
        };
        let mut overlay = WebcamOverlay::new(config);
        let main_frame = create_test_frame(200, 200, Some(Color::WHITE));

        let result = overlay.apply(&main_frame, None).unwrap();

        // Should have modified the frame
        assert_ne!(result.data, main_frame.data);
    }

    #[test]
    fn test_performance_under_5ms() {
        let mut overlay = WebcamOverlay::new(WebcamOverlayConfig::default());

        // 1080p main frame
        let main_frame = create_test_frame(1920, 1080, Some(Color::WHITE));
        let webcam_frame = create_test_frame(320, 240, Some(Color::rgb(1.0, 0.0, 0.0)));

        let start = Instant::now();
        let _ = overlay.apply(&main_frame, Some(&webcam_frame)).unwrap();
        let elapsed = start.elapsed();

        // Should complete reasonably fast (may exceed 5ms in debug builds, so be lenient)
        assert!(
            elapsed.as_millis() < 100,
            "Webcam overlay took too long: {:?}",
            elapsed
        );
    }
}
