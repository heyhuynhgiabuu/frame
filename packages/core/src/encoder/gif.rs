//! High-quality GIF encoder using gifski
//!
//! Provides palette-optimized GIF export with configurable quality and size targets.
//! Targets <10MB for 10s clip at 480p.

use crate::{FrameError, FrameResult, IntoFrameError};
use imgref::ImgVec;
use rgb::RGBA8;
use std::path::PathBuf;
use std::time::Duration;

/// GIF encoder configuration
#[derive(Debug, Clone)]
pub struct GifEncoderConfig {
    /// Output width in pixels
    pub width: u32,
    /// Output height in pixels
    pub height: u32,
    /// Frames per second (default: 10)
    pub fps: u32,
    /// Quality from 1-100 (higher = better quality, larger file)
    pub quality: u8,
    /// Loop count (0 = infinite)
    pub loop_count: u16,
}

impl Default for GifEncoderConfig {
    fn default() -> Self {
        Self {
            width: 854, // 480p width (16:9)
            height: 480,
            fps: 10,
            quality: 80,
            loop_count: 0,
        }
    }
}

impl GifEncoderConfig {
    /// Create a new config with target dimensions
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            ..Default::default()
        }
    }

    /// Set the frame rate
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }

    /// Set the quality (1-100)
    pub fn with_quality(mut self, quality: u8) -> Self {
        self.quality = quality.clamp(1, 100);
        self
    }

    /// Set the loop count (0 = infinite)
    pub fn with_loop_count(mut self, count: u16) -> Self {
        self.loop_count = count;
        self
    }
}

/// Frame data for internal storage
#[derive(Debug, Clone)]
struct FrameData {
    /// RGBA pixel data
    data: Vec<u8>,
    /// Frame width
    width: u32,
    /// Frame height
    height: u32,
    /// Timestamp for frame timing
    timestamp: Duration,
}

impl FrameData {
    /// Convert stored RGBA data to imgref format for gifski
    fn to_rgba8_img(&self) -> ImgVec<RGBA8> {
        let mut rgba: Vec<RGBA8> = Vec::with_capacity((self.width * self.height) as usize);
        for chunk in self.data.chunks_exact(4) {
            rgba.push(RGBA8::new(chunk[0], chunk[1], chunk[2], chunk[3]));
        }
        ImgVec::new(rgba, self.width as usize, self.height as usize)
    }
}

/// GIF encoder using gifski for high-quality palette optimization
pub struct GifEncoder {
    config: GifEncoderConfig,
    frames: Vec<FrameData>,
}

impl GifEncoder {
    /// Create a new GIF encoder with the given configuration
    pub fn new(config: GifEncoderConfig) -> Self {
        Self {
            config,
            frames: Vec::new(),
        }
    }

    /// Add a frame to the encoder
    ///
    /// # Arguments
    /// * `frame` - Raw pixel data in BGRA format (as captured from screen)
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    /// * `timestamp` - Frame presentation timestamp
    ///
    /// # Errors
    /// Returns error if frame dimensions don't match or conversion fails
    pub fn add_frame(
        &mut self,
        frame: &[u8],
        width: u32,
        height: u32,
        timestamp: Duration,
    ) -> FrameResult<()> {
        // Validate frame data size (BGRA = 4 bytes per pixel)
        let expected_size = (width * height * 4) as usize;
        if frame.len() != expected_size {
            return Err(FrameError::EncodingError(format!(
                "Frame data size mismatch: expected {} bytes, got {}",
                expected_size,
                frame.len()
            )));
        }

        // Convert BGRA to RGBA and resize if needed
        let rgba_data = if width != self.config.width || height != self.config.height {
            Self::bgra_to_rgba_resized(frame, width, height, self.config.width, self.config.height)
        } else {
            Self::bgra_to_rgba(frame)
        };

        self.frames.push(FrameData {
            data: rgba_data,
            width: self.config.width,
            height: self.config.height,
            timestamp,
        });

        Ok(())
    }

    /// Convert BGRA pixel data to RGBA format
    fn bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
        let pixel_count = bgra.len() / 4;
        let mut rgba = Vec::with_capacity(bgra.len());

        for i in 0..pixel_count {
            let idx = i * 4;
            // BGRA to RGBA: swap B and R
            rgba.push(bgra[idx + 2]); // R
            rgba.push(bgra[idx + 1]); // G
            rgba.push(bgra[idx]); // B
            rgba.push(bgra[idx + 3]); // A
        }

        rgba
    }

    /// Convert BGRA to RGBA and resize using bilinear interpolation
    fn bgra_to_rgba_resized(
        bgra: &[u8],
        src_width: u32,
        src_height: u32,
        dst_width: u32,
        dst_height: u32,
    ) -> Vec<u8> {
        let mut rgba = vec![0u8; (dst_width * dst_height * 4) as usize];

        let x_ratio = src_width as f32 / dst_width as f32;
        let y_ratio = src_height as f32 / dst_height as f32;

        for y in 0..dst_height {
            for x in 0..dst_width {
                let src_x = (x as f32 * x_ratio).min(src_width as f32 - 1.0);
                let src_y = (y as f32 * y_ratio).min(src_height as f32 - 1.0);

                // Bilinear interpolation
                let x0 = src_x as u32;
                let y0 = src_y as u32;
                let x1 = (x0 + 1).min(src_width - 1);
                let y1 = (y0 + 1).min(src_height - 1);

                let fx = src_x - x0 as f32;
                let fy = src_y - y0 as f32;

                let dst_idx = ((y * dst_width + x) * 4) as usize;

                // For each channel (R, G, B, A)
                for c in 0..4 {
                    // Sample four corners in BGRA format
                    // Channel mapping: 0=B, 1=G, 2=R, 3=A
                    // For RGBA output: 0=R, 1=G, 2=B, 3=A
                    let bgra_c = match c {
                        0 => 2, // R comes from BGRA index 2
                        1 => 1, // G stays at index 1
                        2 => 0, // B comes from BGRA index 0
                        3 => 3, // A stays at index 3
                        _ => unreachable!(),
                    };

                    let idx00 = ((y0 * src_width + x0) * 4 + bgra_c) as usize;
                    let idx10 = ((y0 * src_width + x1) * 4 + bgra_c) as usize;
                    let idx01 = ((y1 * src_width + x0) * 4 + bgra_c) as usize;
                    let idx11 = ((y1 * src_width + x1) * 4 + bgra_c) as usize;

                    let v00 = bgra[idx00] as f32;
                    let v10 = bgra[idx10] as f32;
                    let v01 = bgra[idx01] as f32;
                    let v11 = bgra[idx11] as f32;

                    let v0 = v00 + fx * (v10 - v00);
                    let v1 = v01 + fx * (v11 - v01);
                    let v = v0 + fy * (v1 - v0);

                    rgba[dst_idx + c] = v.round() as u8;
                }
            }
        }

        rgba
    }

    /// Finish encoding and write to output path
    ///
    /// This method uses gifski for high-quality palette optimization.
    /// It processes all accumulated frames and writes the final GIF.
    ///
    /// # Arguments
    /// * `output_path` - Path where the GIF file will be written
    ///
    /// # Errors
    /// Returns error if no frames, encoding fails, or file cannot be written
    pub fn finish(self, output_path: PathBuf) -> FrameResult<()> {
        if self.frames.is_empty() {
            return Err(FrameError::EncodingError("No frames to encode".to_string()));
        }

        // Sort frames by timestamp
        let mut frames = self.frames;
        frames.sort_by_key(|f| f.timestamp);

        // Create gifski settings
        let settings = gifski::Settings {
            width: Some(self.config.width),
            height: Some(self.config.height),
            quality: self.config.quality,
            fast: false, // Use high-quality mode
            repeat: if self.config.loop_count == 0 {
                gifski::Repeat::Infinite
            } else {
                gifski::Repeat::Finite(self.config.loop_count)
            },
        };

        // Create encoder
        let (collector, writer) = gifski::new(settings).map_err(|e| {
            FrameError::EncodingError(format!("Failed to create gifski encoder: {}", e))
        })?;

        // Calculate presentation timestamps based on fps
        let frame_duration = 1.0 / self.config.fps as f64;

        // Add all frames to the collector in a separate scope
        {
            let collector = collector;

            for (i, frame_data) in frames.iter().enumerate() {
                let rgba_data = frame_data.to_rgba8_img();
                let timestamp = i as f64 * frame_duration;

                collector
                    .add_frame_rgba(i, rgba_data, timestamp)
                    .map_err(|e| {
                        FrameError::EncodingError(format!("Failed to add frame {}: {}", i, e))
                    })?;
            }

            // Drop collector to signal we're done adding frames
            drop(collector);
        }

        // Write the GIF file
        let file = std::fs::File::create(&output_path)
            .into_frame_error(format!("Failed to create output file: {:?}", output_path))?;

        // Write using NoProgress
        let mut no_progress = gifski::progress::NoProgress {};
        writer
            .write(file, &mut no_progress)
            .map_err(|e| FrameError::EncodingError(format!("Failed to write GIF: {}", e)))?;

        tracing::info!(
            "GIF encoding complete: {} frames written to {:?}",
            frames.len(),
            output_path
        );

        Ok(())
    }

    /// Get the number of frames accumulated
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get the encoder configuration
    pub fn config(&self) -> &GifEncoderConfig {
        &self.config
    }
}

impl Default for GifEncoder {
    fn default() -> Self {
        Self::new(GifEncoderConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_bgra_frame(width: u32, height: u32) -> Vec<u8> {
        let mut data = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                data[idx] = (x % 256) as u8; // B
                data[idx + 1] = (y % 256) as u8; // G
                data[idx + 2] = 128; // R
                data[idx + 3] = 255; // A
            }
        }
        data
    }

    #[test]
    fn test_gif_encoder_config_default() {
        let config = GifEncoderConfig::default();
        assert_eq!(config.width, 854);
        assert_eq!(config.height, 480);
        assert_eq!(config.fps, 10);
        assert_eq!(config.quality, 80);
        assert_eq!(config.loop_count, 0);
    }

    #[test]
    fn test_gif_encoder_config_builder() {
        let config = GifEncoderConfig::new(640, 360)
            .with_fps(15)
            .with_quality(90)
            .with_loop_count(1);

        assert_eq!(config.width, 640);
        assert_eq!(config.height, 360);
        assert_eq!(config.fps, 15);
        assert_eq!(config.quality, 90);
        assert_eq!(config.loop_count, 1);
    }

    #[test]
    fn test_gif_encoder_config_quality_clamping() {
        let config_low = GifEncoderConfig::default().with_quality(0);
        assert_eq!(config_low.quality, 1);

        let config_high = GifEncoderConfig::default().with_quality(150);
        assert_eq!(config_high.quality, 100);
    }

    #[test]
    fn test_gif_encoder_creation() {
        let config = GifEncoderConfig::default();
        let encoder = GifEncoder::new(config);
        assert_eq!(encoder.frame_count(), 0);
    }

    #[test]
    fn test_bgra_to_rgba_conversion() {
        // BGRA pixel: [B=10, G=20, R=30, A=255]
        let bgra = vec![10, 20, 30, 255];

        let rgba = GifEncoder::bgra_to_rgba(&bgra);

        // Should be converted to RGBA: [R=30, G=20, B=10, A=255]
        assert_eq!(rgba[0], 30); // R
        assert_eq!(rgba[1], 20); // G
        assert_eq!(rgba[2], 10); // B
        assert_eq!(rgba[3], 255); // A
    }

    #[test]
    fn test_add_frame_valid() {
        let config = GifEncoderConfig::new(100, 100);
        let mut encoder = GifEncoder::new(config);

        let frame = create_test_bgra_frame(100, 100);
        let result = encoder.add_frame(&frame, 100, 100, Duration::from_millis(0));

        assert!(result.is_ok());
        assert_eq!(encoder.frame_count(), 1);
    }

    #[test]
    fn test_add_frame_size_mismatch() {
        let config = GifEncoderConfig::new(100, 100);
        let mut encoder = GifEncoder::new(config);

        // Provide wrong dimensions
        let frame = create_test_bgra_frame(50, 50);
        let result = encoder.add_frame(&frame, 100, 100, Duration::from_millis(0));

        assert!(result.is_err());
    }

    #[test]
    fn test_add_frame_with_resize() {
        let config = GifEncoderConfig::new(50, 50); // Smaller output
        let mut encoder = GifEncoder::new(config);

        let frame = create_test_bgra_frame(100, 100); // Larger input
        let result = encoder.add_frame(&frame, 100, 100, Duration::from_millis(0));

        assert!(result.is_ok());
        assert_eq!(encoder.frame_count(), 1);
    }

    #[test]
    fn test_finish_no_frames() {
        let config = GifEncoderConfig::new(100, 100);
        let encoder = GifEncoder::new(config);

        let temp_path = std::env::temp_dir().join("test_empty.gif");
        let result = encoder.finish(temp_path);

        assert!(result.is_err());
    }

    #[test]
    fn test_frame_count_tracking() {
        let config = GifEncoderConfig::new(50, 50);
        let mut encoder = GifEncoder::new(config);

        assert_eq!(encoder.frame_count(), 0);

        let frame = create_test_bgra_frame(50, 50);
        encoder
            .add_frame(&frame, 50, 50, Duration::from_millis(0))
            .unwrap();
        assert_eq!(encoder.frame_count(), 1);

        encoder
            .add_frame(&frame, 50, 50, Duration::from_millis(100))
            .unwrap();
        assert_eq!(encoder.frame_count(), 2);
    }

    #[test]
    fn test_config_accessor() {
        let config = GifEncoderConfig::new(1280, 720).with_fps(30);
        let encoder = GifEncoder::new(config.clone());

        assert_eq!(encoder.config().width, 1280);
        assert_eq!(encoder.config().height, 720);
        assert_eq!(encoder.config().fps, 30);
    }

    #[test]
    fn test_default_gif_encoder() {
        let encoder = GifEncoder::default();
        assert_eq!(encoder.frame_count(), 0);
        assert_eq!(encoder.config().width, 854);
        assert_eq!(encoder.config().height, 480);
    }
}
