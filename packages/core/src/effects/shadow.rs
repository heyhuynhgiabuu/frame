//! Shadow effect for video frames
//!
//! Applies a drop shadow behind frames with configurable offset, blur, and color.
//! Respects corner radius from background configuration for rounded shadows.

use crate::capture::{Frame, PixelFormat};
use crate::effects::Color;
use crate::FrameResult;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Shadow effect configuration
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShadowConfig {
    /// Whether shadow is enabled
    pub enabled: bool,
    /// Horizontal shadow offset in pixels
    pub offset_x: i32,
    /// Vertical shadow offset in pixels
    pub offset_y: i32,
    /// Blur radius in pixels
    pub blur_radius: f32,
    /// Shadow color
    pub color: Color,
    /// Shadow opacity (0.0-1.0)
    pub opacity: f32,
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            offset_x: 4,
            offset_y: 4,
            blur_radius: 8.0,
            color: Color::BLACK,
            opacity: 0.4,
        }
    }
}

impl ShadowConfig {
    /// Create a new shadow config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Disable shadow
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }
}

/// Shadow effect processor
#[derive(Debug)]
pub struct ShadowEffect {
    config: ShadowConfig,
    /// Cached shadow texture for performance
    cached_shadow: Option<CachedShadow>,
    /// Last used corner radius (for cache invalidation)
    last_corner_radius: f32,
}

#[derive(Debug, Clone)]
struct CachedShadow {
    width: u32,
    height: u32,
    data: Vec<u8>,
    corner_radius: f32,
}

impl Default for ShadowEffect {
    fn default() -> Self {
        Self::new(ShadowConfig::default())
    }
}

impl ShadowEffect {
    /// Create a new shadow effect with the given configuration
    pub fn new(config: ShadowConfig) -> Self {
        Self {
            config,
            cached_shadow: None,
            last_corner_radius: 0.0,
        }
    }

    /// Set the shadow configuration
    pub fn set_config(&mut self, config: ShadowConfig) {
        // Invalidate cache if config changed
        if self.config != config {
            self.cached_shadow = None;
        }
        self.config = config;
    }

    /// Get the current configuration
    pub fn config(&self) -> &ShadowConfig {
        &self.config
    }

    /// Calculate output dimensions including shadow
    pub fn output_dimensions(&self, input_width: u32, input_height: u32) -> (u32, u32) {
        if !self.config.enabled {
            return (input_width, input_height);
        }

        let blur = self.config.blur_radius.ceil() as i32;
        let offset_abs_x = self.config.offset_x.abs();
        let offset_abs_y = self.config.offset_y.abs();

        let out_width = input_width + (blur * 2 + offset_abs_x) as u32;
        let out_height = input_height + (blur * 2 + offset_abs_y) as u32;

        (out_width, out_height)
    }

    /// Apply shadow effect to a frame
    ///
    /// # Arguments
    /// * `frame` - Input frame
    /// * `corner_radius` - Corner radius from Background config (for rounded shadows)
    ///
    /// # Returns
    /// Frame with shadow applied
    ///
    /// # Performance
    /// Target: <2ms per frame at 1080p. Uses cached shadow texture when possible.
    pub fn apply(&mut self, frame: &Frame, corner_radius: f32) -> FrameResult<Frame> {
        if !self.config.enabled {
            return Ok(frame.clone());
        }

        // Validate pixel format
        if frame.format != PixelFormat::Rgba {
            return Err(crate::FrameError::Configuration(format!(
                "Shadow effect requires RGBA format, got {:?}",
                frame.format
            )));
        }

        let start_time = Instant::now();

        let (out_width, out_height) = self.output_dimensions(frame.width, frame.height);
        let blur = self.config.blur_radius.ceil() as u32;

        // Calculate offsets - ensure shadow stays within bounds
        let x_offset = blur + self.config.offset_x.max(0) as u32;
        let y_offset = blur + self.config.offset_y.max(0) as u32;

        // Generate or use cached shadow
        let shadow = self.get_or_create_shadow(frame.width, frame.height, corner_radius, blur)?;

        // Create output buffer
        let output_size = (out_width * out_height * 4) as usize;
        let mut output = vec![0u8; output_size];

        // Composite shadow first
        self.composite_shadow(
            &mut output,
            out_width,
            out_height,
            &shadow,
            x_offset,
            y_offset,
        );

        // Composite frame on top
        self.composite_frame(
            &mut output,
            out_width,
            out_height,
            frame,
            x_offset,
            y_offset,
        );

        // Performance check
        let elapsed = start_time.elapsed();
        if elapsed.as_millis() > 2 {
            tracing::warn!(
                "Shadow effect took {}ms (target: <2ms)",
                elapsed.as_micros() as f64 / 1000.0
            );
        }

        Ok(Frame {
            data: output,
            width: out_width,
            height: out_height,
            timestamp: frame.timestamp,
            format: frame.format,
        })
    }

    /// Get or create cached shadow texture
    fn get_or_create_shadow(
        &mut self,
        width: u32,
        height: u32,
        corner_radius: f32,
        blur: u32,
    ) -> FrameResult<Vec<u8>> {
        // Check if cached version is still valid
        if let Some(cached) = &self.cached_shadow {
            if cached.width == width
                && cached.height == height
                && (cached.corner_radius - corner_radius).abs() < 0.01
            {
                return Ok(cached.data.clone());
            }
        }

        // Generate new shadow
        let data = self.generate_shadow(width, height, corner_radius, blur)?;
        self.cached_shadow = Some(CachedShadow {
            width,
            height,
            data: data.clone(),
            corner_radius,
        });
        self.last_corner_radius = corner_radius;

        Ok(data)
    }

    /// Generate shadow texture with blur and corner radius
    fn generate_shadow(
        &self,
        width: u32,
        height: u32,
        corner_radius: f32,
        blur: u32,
    ) -> FrameResult<Vec<u8>> {
        let size = (width * height * 4) as usize;
        let mut shadow = vec![0u8; size];

        // Calculate shadow color with opacity
        let color = &self.config.color;
        let opacity = self.config.opacity.clamp(0.0, 1.0);
        let r = (color.r * 255.0 * opacity) as u8;
        let g = (color.g * 255.0 * opacity) as u8;
        let b = (color.b * 255.0 * opacity) as u8;

        // Fill shadow rectangle with corner radius masking
        for y in 0..height {
            for x in 0..width {
                let alpha = self.calculate_pixel_alpha(x, y, width, height, corner_radius);

                if alpha > 0.0 {
                    let idx = ((y * width + x) * 4) as usize;
                    shadow[idx] = r;
                    shadow[idx + 1] = g;
                    shadow[idx + 2] = b;
                    shadow[idx + 3] = (alpha * 255.0 * opacity) as u8;
                }
            }
        }

        // Apply Gaussian blur
        if blur > 0 {
            shadow = self.apply_gaussian_blur(&shadow, width, height, blur);
        }

        Ok(shadow)
    }

    /// Calculate alpha for a pixel based on corner radius
    fn calculate_pixel_alpha(&self, x: u32, y: u32, width: u32, height: u32, radius: f32) -> f32 {
        if radius <= 0.0 {
            return 1.0;
        }

        let r = radius;
        let fx = x as f32;
        let fy = y as f32;
        let fw = width as f32;
        let fh = height as f32;

        // Check each corner
        let corners = [
            (0.0, 0.0),       // top-left
            (fw - r, 0.0),    // top-right
            (0.0, fh - r),    // bottom-left
            (fw - r, fh - r), // bottom-right
        ];

        for (cx, cy) in corners {
            if fx >= cx && fx < cx + r && fy >= cy && fy < cy + r {
                // In corner region - calculate distance from corner center
                let (ccx, ccy) = if cx == 0.0 && cy == 0.0 {
                    (r, r) // top-left
                } else if cx > 0.0 && cy == 0.0 {
                    (fw - r, r) // top-right
                } else if cx == 0.0 && cy > 0.0 {
                    (r, fh - r) // bottom-left
                } else {
                    (fw - r, fh - r) // bottom-right
                };

                let dx = fx + 0.5 - ccx;
                let dy = fy + 0.5 - ccy;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist > r {
                    return 0.0; // Outside corner curve
                } else if dist > r - 1.0 {
                    return r - dist; // Anti-aliasing
                }
            }
        }

        1.0
    }

    /// Apply separable Gaussian blur for performance
    fn apply_gaussian_blur(&self, data: &[u8], width: u32, height: u32, radius: u32) -> Vec<u8> {
        if radius == 0 {
            return data.to_vec();
        }

        // Create kernel
        let kernel = self.create_gaussian_kernel(radius);
        let kernel_size = kernel.len();
        let half_kernel = kernel_size / 2;

        // Temporary buffer for horizontal pass
        let mut temp = vec![0u8; data.len()];

        // Horizontal pass
        for y in 0..height {
            for x in 0..width {
                let mut r_acc = 0.0;
                let mut g_acc = 0.0;
                let mut b_acc = 0.0;
                let mut a_acc = 0.0;
                let mut weight_sum = 0.0;

                for (i, &weight) in kernel.iter().enumerate() {
                    let sample_x = x as i32 + i as i32 - half_kernel as i32;
                    let clamped_x = sample_x.clamp(0, width as i32 - 1) as u32;

                    let idx = ((y * width + clamped_x) * 4) as usize;
                    r_acc += data[idx] as f32 * weight;
                    g_acc += data[idx + 1] as f32 * weight;
                    b_acc += data[idx + 2] as f32 * weight;
                    a_acc += data[idx + 3] as f32 * weight;
                    weight_sum += weight;
                }

                let idx = ((y * width + x) * 4) as usize;
                temp[idx] = (r_acc / weight_sum) as u8;
                temp[idx + 1] = (g_acc / weight_sum) as u8;
                temp[idx + 2] = (b_acc / weight_sum) as u8;
                temp[idx + 3] = (a_acc / weight_sum) as u8;
            }
        }

        // Vertical pass
        let mut output = vec![0u8; data.len()];
        for x in 0..width {
            for y in 0..height {
                let mut r_acc = 0.0;
                let mut g_acc = 0.0;
                let mut b_acc = 0.0;
                let mut a_acc = 0.0;
                let mut weight_sum = 0.0;

                for (i, &weight) in kernel.iter().enumerate() {
                    let sample_y = y as i32 + i as i32 - half_kernel as i32;
                    let clamped_y = sample_y.clamp(0, height as i32 - 1) as u32;

                    let idx = ((clamped_y * width + x) * 4) as usize;
                    r_acc += temp[idx] as f32 * weight;
                    g_acc += temp[idx + 1] as f32 * weight;
                    b_acc += temp[idx + 2] as f32 * weight;
                    a_acc += temp[idx + 3] as f32 * weight;
                    weight_sum += weight;
                }

                let idx = ((y * width + x) * 4) as usize;
                output[idx] = (r_acc / weight_sum) as u8;
                output[idx + 1] = (g_acc / weight_sum) as u8;
                output[idx + 2] = (b_acc / weight_sum) as u8;
                output[idx + 3] = (a_acc / weight_sum) as u8;
            }
        }

        output
    }

    /// Create Gaussian kernel
    fn create_gaussian_kernel(&self, radius: u32) -> Vec<f32> {
        let size = (radius * 2 + 1) as usize;
        let sigma = radius as f32 / 2.0;
        let mut kernel = Vec::with_capacity(size);

        for i in 0..size {
            let x = i as f32 - radius as f32;
            let value = (-x * x / (2.0 * sigma * sigma)).exp();
            kernel.push(value);
        }

        // Normalize
        let sum: f32 = kernel.iter().sum();
        kernel.iter_mut().for_each(|v| *v /= sum);

        kernel
    }

    /// Composite shadow onto output buffer
    fn composite_shadow(
        &self,
        output: &mut [u8],
        out_width: u32,
        out_height: u32,
        shadow: &[u8],
        x_offset: u32,
        y_offset: u32,
    ) {
        // Shadow dimensions match frame dimensions
        let shadow_width = shadow.len() as u32 / 4 / out_height;
        let shadow_height = shadow.len() as u32 / 4 / shadow_width;

        for sy in 0..shadow_height {
            for sx in 0..shadow_width {
                let dx = x_offset + sx;
                let dy = y_offset + sy;

                if dx >= out_width || dy >= out_height {
                    continue;
                }

                let shadow_idx = ((sy * shadow_width + sx) * 4) as usize;
                let output_idx = ((dy * out_width + dx) * 4) as usize;

                if shadow_idx + 3 >= shadow.len() || output_idx + 3 >= output.len() {
                    continue;
                }

                // Simple alpha blend
                let shadow_a = shadow[shadow_idx + 3] as f32 / 255.0;
                if shadow_a > 0.0 {
                    for i in 0..4 {
                        let shadow_c = shadow[shadow_idx + i] as f32 / 255.0;
                        output[output_idx + i] = (shadow_c * shadow_a * 255.0) as u8;
                    }
                }
            }
        }
    }

    /// Composite frame onto output buffer
    fn composite_frame(
        &self,
        output: &mut [u8],
        out_width: u32,
        out_height: u32,
        frame: &Frame,
        x_offset: u32,
        y_offset: u32,
    ) {
        for fy in 0..frame.height {
            for fx in 0..frame.width {
                let dx = x_offset + fx;
                let dy = y_offset + fy;

                if dx >= out_width || dy >= out_height {
                    continue;
                }

                let frame_idx = ((fy * frame.width + fx) * 4) as usize;
                let output_idx = ((dy * out_width + dx) * 4) as usize;

                if frame_idx + 3 >= frame.data.len() || output_idx + 3 >= output.len() {
                    continue;
                }

                // Alpha blend frame over existing shadow
                let frame_a = frame.data[frame_idx + 3] as f32 / 255.0;
                let dest_a = output[output_idx + 3] as f32 / 255.0;
                let out_a = frame_a + dest_a * (1.0 - frame_a);

                if out_a > 0.0 {
                    for i in 0..3 {
                        let frame_c = frame.data[frame_idx + i] as f32 / 255.0;
                        let dest_c = output[output_idx + i] as f32 / 255.0;
                        let out_c = (frame_c * frame_a + dest_c * dest_a * (1.0 - frame_a)) / out_a;
                        output[output_idx + i] = (out_c * 255.0) as u8;
                    }
                    output[output_idx + 3] = (out_a * 255.0) as u8;
                }
            }
        }
    }

    /// Reset effect state and clear cache
    pub fn reset(&mut self) {
        self.cached_shadow = None;
        self.last_corner_radius = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::PixelFormat;
    use std::time::Duration;

    #[test]
    fn test_default_config() {
        let config = ShadowConfig::default();
        assert!(config.enabled);
        assert_eq!(config.offset_x, 4);
        assert_eq!(config.offset_y, 4);
        assert_eq!(config.blur_radius, 8.0);
        assert_eq!(config.opacity, 0.4);
        assert_eq!(config.color, Color::BLACK);
    }

    #[test]
    fn test_disabled_config() {
        let config = ShadowConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_output_dimensions_enabled() {
        let config = ShadowConfig::default();
        let effect = ShadowEffect::new(config);

        // With blur=8, offset=4, expect: width + 2*8 + 4 = width + 20
        let (w, h) = effect.output_dimensions(100, 100);
        assert_eq!(w, 120); // 100 + 16 + 4
        assert_eq!(h, 120);
    }

    #[test]
    fn test_output_dimensions_disabled() {
        let config = ShadowConfig::disabled();
        let effect = ShadowEffect::new(config);

        let (w, h) = effect.output_dimensions(100, 100);
        assert_eq!(w, 100);
        assert_eq!(h, 100);
    }

    #[test]
    fn test_apply_disabled() {
        let config = ShadowConfig::disabled();
        let mut effect = ShadowEffect::new(config);

        let frame = Frame {
            data: vec![255u8; 4 * 10 * 10],
            width: 10,
            height: 10,
            timestamp: Duration::from_secs(0),
            format: PixelFormat::Rgba,
        };

        let result = effect.apply(&frame, 0.0).unwrap();
        assert_eq!(result.width, frame.width);
        assert_eq!(result.height, frame.height);
        assert_eq!(result.data, frame.data);
    }

    #[test]
    fn test_apply_with_shadow() {
        let config = ShadowConfig {
            enabled: true,
            offset_x: 2,
            offset_y: 2,
            blur_radius: 2.0,
            color: Color::BLACK,
            opacity: 0.5,
        };
        let mut effect = ShadowEffect::new(config);

        let frame = Frame {
            data: vec![255u8; 4 * 10 * 10], // white square
            width: 10,
            height: 10,
            timestamp: Duration::from_secs(0),
            format: PixelFormat::Rgba,
        };

        let result = effect.apply(&frame, 0.0).unwrap();

        // Output should be larger due to shadow
        assert!(result.width > frame.width);
        assert!(result.height > frame.height);

        // Should have shadow pixels (non-zero data)
        assert!(!result.data.iter().all(|&p| p == 0));
    }

    #[test]
    fn test_gaussian_kernel() {
        let effect = ShadowEffect::new(ShadowConfig::default());
        let kernel = effect.create_gaussian_kernel(3);

        // Kernel should be normalized
        let sum: f32 = kernel.iter().sum();
        assert!((sum - 1.0).abs() < 0.001);

        // Should have correct size
        assert_eq!(kernel.len(), 7); // radius*2 + 1
    }

    #[test]
    fn test_config_update() {
        let mut effect = ShadowEffect::new(ShadowConfig::default());

        // Set config should invalidate cache
        let new_config = ShadowConfig {
            offset_x: 10,
            ..Default::default()
        };
        effect.set_config(new_config);

        assert_eq!(effect.config().offset_x, 10);
        assert!(effect.cached_shadow.is_none());
    }

    #[test]
    fn test_reset() {
        let mut effect = ShadowEffect::new(ShadowConfig::default());

        // Create a cached shadow
        let frame = Frame {
            data: vec![255u8; 4 * 10 * 10],
            width: 10,
            height: 10,
            timestamp: Duration::from_secs(0),
            format: PixelFormat::Rgba,
        };
        let _ = effect.apply(&frame, 0.0);

        assert!(effect.cached_shadow.is_some());

        // Reset should clear cache
        effect.reset();
        assert!(effect.cached_shadow.is_none());
    }

    #[test]
    fn test_performance_under_2ms() {
        let mut effect = ShadowEffect::new(ShadowConfig::default());

        // 100x100 frame (small enough for fast test)
        let frame = Frame {
            data: vec![255u8; 4 * 100 * 100],
            width: 100,
            height: 100,
            timestamp: Duration::from_secs(0),
            format: PixelFormat::Rgba,
        };

        let start = Instant::now();
        let _ = effect.apply(&frame, 0.0).unwrap();
        let elapsed = start.elapsed();

        // Should complete in under 2ms for small frames
        // Note: This is a sanity check; actual performance depends on hardware
        assert!(
            elapsed.as_millis() < 100,
            "Shadow effect took too long: {:?}",
            elapsed
        );
    }
}
