//! Background compositing for video frames
//!
//! Handles solid colors, gradients, padding, and corner radius.

use crate::capture::Frame;
use crate::effects::{Background, BackgroundStyle, Color};
use crate::FrameResult;

/// Background compositor that applies backgrounds and padding
#[derive(Debug)]
pub struct BackgroundCompositor {
    config: Background,
    /// Cached background data for performance
    cached_background: Option<CachedBackground>,
}

#[derive(Debug)]
struct CachedBackground {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Default for BackgroundCompositor {
    fn default() -> Self {
        Self::new(Background::default())
    }
}

impl BackgroundCompositor {
    pub fn new(config: Background) -> Self {
        Self {
            config,
            cached_background: None,
        }
    }

    /// Set background configuration
    pub fn set_config(&mut self, config: Background) {
        // Invalidate cache if config changed
        self.cached_background = None;
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &Background {
        &self.config
    }

    /// Calculate output dimensions with padding
    pub fn output_dimensions(&self, input_width: u32, input_height: u32) -> (u32, u32) {
        let pad = &self.config.padding;
        let out_width = input_width + (pad.left + pad.right) as u32;
        let out_height = input_height + (pad.top + pad.bottom) as u32;
        (out_width, out_height)
    }

    /// Composite a frame onto the background
    pub fn composite(&mut self, frame: &Frame) -> FrameResult<Frame> {
        let (out_width, out_height) = self.output_dimensions(frame.width, frame.height);

        // Generate or use cached background
        let background = self.get_or_create_background(out_width, out_height)?;

        // Create output buffer starting with background
        let mut output = background.clone();

        // Copy frame data onto background with padding offset
        let x_offset = self.config.padding.left as u32;
        let y_offset = self.config.padding.top as u32;

        self.blit_with_corner_radius(
            &mut output,
            out_width,
            out_height,
            &frame.data,
            frame.width,
            frame.height,
            x_offset,
            y_offset,
        );

        Ok(Frame {
            data: output,
            width: out_width,
            height: out_height,
            timestamp: frame.timestamp,
            format: frame.format,
        })
    }

    /// Get or create cached background
    fn get_or_create_background(&mut self, width: u32, height: u32) -> FrameResult<Vec<u8>> {
        // Check if cached version is still valid
        if let Some(cached) = &self.cached_background {
            if cached.width == width && cached.height == height {
                return Ok(cached.data.clone());
            }
        }

        // Generate new background
        let data = self.generate_background(width, height)?;
        self.cached_background = Some(CachedBackground {
            width,
            height,
            data: data.clone(),
        });

        Ok(data)
    }

    /// Generate background data
    fn generate_background(&self, width: u32, height: u32) -> FrameResult<Vec<u8>> {
        let size = (width * height * 4) as usize;
        let mut data = vec![0u8; size];

        match &self.config.style {
            BackgroundStyle::Transparent => {
                // Already zeroed (transparent)
            }
            BackgroundStyle::Solid(color) => {
                self.fill_solid(&mut data, color);
            }
            BackgroundStyle::Gradient { start, end, angle } => {
                self.fill_gradient(&mut data, width, height, start, end, *angle);
            }
            BackgroundStyle::Image {
                path: _,
                scale_mode: _,
            } => {
                // Image loading is a separate concern - for now, use solid fallback
                // The actual image loading would be done in the renderer package
            }
        }

        Ok(data)
    }

    /// Fill buffer with solid color
    fn fill_solid(&self, data: &mut [u8], color: &Color) {
        let r = (color.r * 255.0) as u8;
        let g = (color.g * 255.0) as u8;
        let b = (color.b * 255.0) as u8;
        let a = (color.a * 255.0) as u8;

        for chunk in data.chunks_exact_mut(4) {
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a;
        }
    }

    /// Fill buffer with gradient
    fn fill_gradient(
        &self,
        data: &mut [u8],
        width: u32,
        height: u32,
        start: &Color,
        end: &Color,
        angle: f32,
    ) {
        let angle_rad = angle.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        // Calculate gradient length based on angle
        let diag = ((width as f32).powi(2) + (height as f32).powi(2)).sqrt();

        for y in 0..height {
            for x in 0..width {
                // Project point onto gradient line
                let nx = x as f32 / width as f32 - 0.5;
                let ny = y as f32 / height as f32 - 0.5;
                let t = ((nx * cos_a + ny * sin_a) / (diag / width as f32) + 0.5).clamp(0.0, 1.0);

                // Interpolate colors
                let r = (start.r + (end.r - start.r) * t) * 255.0;
                let g = (start.g + (end.g - start.g) * t) * 255.0;
                let b = (start.b + (end.b - start.b) * t) * 255.0;
                let a = (start.a + (end.a - start.a) * t) * 255.0;

                let idx = ((y * width + x) * 4) as usize;
                data[idx] = r as u8;
                data[idx + 1] = g as u8;
                data[idx + 2] = b as u8;
                data[idx + 3] = a as u8;
            }
        }
    }

    /// Blit source onto destination with corner radius masking
    #[allow(clippy::too_many_arguments)]
    fn blit_with_corner_radius(
        &self,
        dest: &mut [u8],
        dest_width: u32,
        dest_height: u32,
        src: &[u8],
        src_width: u32,
        src_height: u32,
        x_offset: u32,
        y_offset: u32,
    ) {
        let radius = self.config.corner_radius;

        for sy in 0..src_height {
            for sx in 0..src_width {
                let dx = x_offset + sx;
                let dy = y_offset + sy;

                if dx >= dest_width || dy >= dest_height {
                    continue;
                }

                // Check corner radius mask
                let alpha = self.corner_alpha(sx, sy, src_width, src_height, radius);
                if alpha < 0.01 {
                    continue;
                }

                let src_idx = ((sy * src_width + sx) * 4) as usize;
                let dest_idx = ((dy * dest_width + dx) * 4) as usize;

                if src_idx + 3 >= src.len() || dest_idx + 3 >= dest.len() {
                    continue;
                }

                // Alpha blend with corner mask
                if alpha >= 0.99 {
                    // Opaque copy
                    dest[dest_idx..dest_idx + 4].copy_from_slice(&src[src_idx..src_idx + 4]);
                } else {
                    // Blend with corner alpha
                    let src_a = src[src_idx + 3] as f32 / 255.0 * alpha;
                    let dest_a = dest[dest_idx + 3] as f32 / 255.0;
                    let out_a = src_a + dest_a * (1.0 - src_a);

                    if out_a > 0.0 {
                        for i in 0..3 {
                            let src_c = src[src_idx + i] as f32 / 255.0;
                            let dest_c = dest[dest_idx + i] as f32 / 255.0;
                            let out_c = (src_c * src_a + dest_c * dest_a * (1.0 - src_a)) / out_a;
                            dest[dest_idx + i] = (out_c * 255.0) as u8;
                        }
                        dest[dest_idx + 3] = (out_a * 255.0) as u8;
                    }
                }
            }
        }
    }

    /// Calculate alpha value for a pixel based on corner radius
    fn corner_alpha(&self, x: u32, y: u32, width: u32, height: u32, radius: f32) -> f32 {
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
                // Adjust corner center for each quadrant
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

    /// Reset compositor state
    pub fn reset(&mut self) {
        self.cached_background = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::PixelFormat;
    use crate::effects::Padding;

    #[test]
    fn test_output_dimensions() {
        let config = Background {
            padding: Padding::all(20.0),
            ..Default::default()
        };
        let comp = BackgroundCompositor::new(config);

        let (w, h) = comp.output_dimensions(1920, 1080);
        assert_eq!(w, 1960);
        assert_eq!(h, 1120);
    }

    #[test]
    fn test_solid_background() {
        let config = Background {
            style: BackgroundStyle::Solid(Color::rgb(1.0, 0.0, 0.0)),
            padding: Padding::zero(),
            corner_radius: 0.0,
        };
        let mut comp = BackgroundCompositor::new(config);

        let frame = Frame {
            data: vec![0u8; 4 * 10 * 10], // 10x10 transparent
            width: 10,
            height: 10,
            timestamp: std::time::Duration::from_secs(0),
            format: PixelFormat::Rgba,
        };

        let result = comp.composite(&frame).unwrap();
        assert_eq!(result.width, 10);
        assert_eq!(result.height, 10);
    }

    #[test]
    fn test_padding_applied() {
        let config = Background {
            style: BackgroundStyle::Solid(Color::rgb(0.0, 1.0, 0.0)),
            padding: Padding::all(10.0),
            corner_radius: 0.0,
        };
        let mut comp = BackgroundCompositor::new(config);

        let frame = Frame {
            data: vec![255u8; 4 * 100 * 100],
            width: 100,
            height: 100,
            timestamp: std::time::Duration::from_secs(0),
            format: PixelFormat::Rgba,
        };

        let result = comp.composite(&frame).unwrap();
        assert_eq!(result.width, 120); // 100 + 10 + 10
        assert_eq!(result.height, 120);
    }

    #[test]
    fn test_corner_alpha() {
        let config = Background {
            corner_radius: 10.0,
            ..Default::default()
        };
        let comp = BackgroundCompositor::new(config);

        // Center should be fully opaque
        assert!((comp.corner_alpha(50, 50, 100, 100, 10.0) - 1.0).abs() < 0.01);

        // Very corner should be transparent
        assert!(comp.corner_alpha(0, 0, 100, 100, 10.0) < 0.5);
    }
}
