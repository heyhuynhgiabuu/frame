//! Video encoder using ffmpeg-sidecar for MP4 output
//!
//! Provides H.264/H.265 encoding via bundled ffmpeg binary

use crate::EditHistory;
use std::time::Duration;

/// Video frame for encoding (standalone type, doesn't require capture feature)
#[derive(Debug, Clone)]
pub struct VideoFrame {
    /// Raw pixel data (BGRA format)
    pub data: Vec<u8>,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Frame timestamp
    pub timestamp: Duration,
}

/// Audio samples for encoding
#[derive(Debug, Clone)]
pub struct AudioSamples {
    /// Interleaved audio samples (f32)
    pub samples: Vec<f32>,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u16,
}

/// Helper to check if a timestamp should be included based on edit operations
pub struct EditFilter<'a> {
    edit_history: &'a EditHistory,
    original_duration: Duration,
}

impl<'a> EditFilter<'a> {
    /// Create a new edit filter
    pub fn new(edit_history: &'a EditHistory, original_duration: Duration) -> Self {
        Self {
            edit_history,
            original_duration,
        }
    }

    /// Check if a timestamp should be included in the export
    ///
    /// Returns `None` if the timestamp should be excluded (trimmed/cut),
    /// or `Some(adjusted_timestamp)` if it should be included with an adjusted time.
    pub fn filter_timestamp(&self, timestamp: Duration) -> Option<Duration> {
        use crate::EditOperation;

        let mut current_time = timestamp;
        let mut time_offset = Duration::ZERO;

        for op in self.edit_history.applied_operations() {
            match op {
                EditOperation::Trim { start, end } => {
                    // If outside trim range, exclude
                    if current_time < *start || current_time > *end {
                        return None;
                    }
                    // Adjust time relative to trim start
                    current_time = current_time.saturating_sub(*start);
                }
                EditOperation::Cut { from, to } => {
                    // If inside cut range, exclude
                    if current_time >= *from && current_time <= *to {
                        return None;
                    }
                    // If after cut, shift time earlier
                    if current_time > *to {
                        let cut_duration = to.saturating_sub(*from);
                        time_offset += cut_duration;
                    }
                }
                EditOperation::Split { .. } => {
                    // Splits don't affect filtering
                }
            }
        }

        // Apply accumulated time offset from cuts
        Some(current_time.saturating_sub(time_offset))
    }

    /// Get the effective duration after all edits
    pub fn effective_duration(&self) -> Duration {
        self.edit_history.effective_duration(self.original_duration)
    }
}

#[cfg(feature = "encoding")]
mod sidecar_encoder {
    use super::{AudioSamples, VideoFrame};
    use crate::{FrameError, FrameResult};
    use ffmpeg_sidecar::command::FfmpegCommand;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::{Child, ChildStdin, Stdio};

    /// Video codec to use for encoding
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub enum VideoCodec {
        /// H.264/AVC - widely compatible
        #[default]
        H264,
        /// H.265/HEVC - better compression, newer
        H265,
    }

    impl VideoCodec {
        fn as_encoder(&self, hardware_accel: bool) -> &'static str {
            match (self, hardware_accel) {
                // macOS VideoToolbox hardware encoding
                (VideoCodec::H264, true) => "h264_videotoolbox",
                (VideoCodec::H265, true) => "hevc_videotoolbox",
                // Software encoding
                (VideoCodec::H264, false) => "libx264",
                (VideoCodec::H265, false) => "libx265",
            }
        }
    }

    /// Audio codec to use for encoding
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub enum AudioCodec {
        /// AAC - widely compatible
        #[default]
        Aac,
        /// Opus - better quality at low bitrates
        Opus,
    }

    impl AudioCodec {
        fn as_encoder(&self) -> &'static str {
            match self {
                AudioCodec::Aac => "aac",
                AudioCodec::Opus => "libopus",
            }
        }
    }

    /// Encoder configuration
    #[derive(Debug, Clone)]
    pub struct EncoderConfig {
        /// Video codec
        pub video_codec: VideoCodec,
        /// Audio codec
        pub audio_codec: AudioCodec,
        /// Video bitrate in bits per second
        pub video_bitrate: u64,
        /// Audio bitrate in bits per second
        pub audio_bitrate: u64,
        /// Frame rate
        pub frame_rate: u32,
        /// Output width (0 = use input width)
        pub width: u32,
        /// Output height (0 = use input height)
        pub height: u32,
        /// Use hardware acceleration if available
        pub hardware_accel: bool,
        /// Preset (ultrafast, fast, medium, slow, veryslow)
        pub preset: String,
        /// CRF quality (0-51, lower = better quality, default 23)
        pub crf: u32,
    }

    impl Default for EncoderConfig {
        fn default() -> Self {
            Self {
                video_codec: VideoCodec::H264,
                audio_codec: AudioCodec::Aac,
                video_bitrate: 8_000_000, // 8 Mbps
                audio_bitrate: 192_000,   // 192 kbps
                frame_rate: 60,
                width: 0,
                height: 0,
                hardware_accel: true, // Default to hardware accel on macOS
                preset: "medium".to_string(),
                crf: 23,
            }
        }
    }

    /// Progress callback type
    pub type ProgressCallback = Box<dyn Fn(EncoderProgress) + Send + 'static>;

    /// Encoder progress information
    #[derive(Debug, Clone)]
    pub struct EncoderProgress {
        pub frame: u64,
        pub fps: f64,
        pub bitrate_kbps: f64,
        pub time_seconds: f64,
        pub speed: f64,
    }

    /// Video/Audio encoder using ffmpeg-sidecar
    pub struct Encoder {
        config: EncoderConfig,
        output_path: Option<PathBuf>,
        ffmpeg_process: Option<Child>,
        video_stdin: Option<ChildStdin>,
        frame_count: u64,
        input_width: u32,
        input_height: u32,
        initialized: bool,
        // Audio is written to a separate temp file and muxed at the end
        audio_temp_path: Option<PathBuf>,
        audio_samples: Vec<f32>,
        audio_sample_rate: u32,
        audio_channels: u16,
    }

    impl Encoder {
        /// Create a new encoder with the given configuration
        pub fn new(config: EncoderConfig) -> FrameResult<Self> {
            Ok(Self {
                config,
                output_path: None,
                ffmpeg_process: None,
                video_stdin: None,
                frame_count: 0,
                input_width: 0,
                input_height: 0,
                initialized: false,
                audio_temp_path: None,
                audio_samples: Vec::new(),
                audio_sample_rate: 48000,
                audio_channels: 2,
            })
        }

        /// Ensure ffmpeg is available, download if needed
        pub fn ensure_ffmpeg() -> FrameResult<()> {
            use ffmpeg_sidecar::download::auto_download;

            // Check if ffmpeg is already available
            if ffmpeg_sidecar::command::ffmpeg_is_installed() {
                tracing::debug!("FFmpeg is already installed");
                return Ok(());
            }

            tracing::info!("FFmpeg not found, downloading...");
            auto_download().map_err(|e| {
                FrameError::EncodingError(format!("Failed to download FFmpeg: {}", e))
            })?;

            tracing::info!("FFmpeg downloaded successfully");
            Ok(())
        }

        /// Initialize the encoder with the output path
        pub fn init(&mut self, output_path: &Path) -> FrameResult<()> {
            // Ensure ffmpeg is available
            Self::ensure_ffmpeg()?;

            self.output_path = Some(output_path.to_path_buf());

            // Create temp path for audio if we'll have audio
            let audio_temp = output_path.with_extension("audio.wav");
            self.audio_temp_path = Some(audio_temp);

            Ok(())
        }

        /// Start the ffmpeg process for video encoding
        fn start_video_encoder(&mut self, width: u32, height: u32) -> FrameResult<()> {
            let output_path = self
                .output_path
                .as_ref()
                .ok_or_else(|| FrameError::EncodingError("Output path not set".to_string()))?;

            let output_width = if self.config.width > 0 {
                self.config.width
            } else {
                width
            };
            let output_height = if self.config.height > 0 {
                self.config.height
            } else {
                height
            };

            self.input_width = width;
            self.input_height = height;

            // Build ffmpeg command for raw video input
            let mut cmd = FfmpegCommand::new();

            // Input format: raw BGRA frames from stdin
            cmd.args([
                "-f",
                "rawvideo",
                "-pixel_format",
                "bgra",
                "-video_size",
                &format!("{}x{}", width, height),
                "-framerate",
                &self.config.frame_rate.to_string(),
                "-i",
                "pipe:0", // Read from stdin
            ]);

            // Video filters (scale if needed)
            if output_width != width || output_height != height {
                cmd.args([
                    "-vf",
                    &format!("scale={}:{}", output_width, output_height),
                ]);
            }

            // Video codec settings
            let video_encoder = self
                .config
                .video_codec
                .as_encoder(self.config.hardware_accel);
            cmd.args(["-c:v", video_encoder]);

            // Codec-specific options
            if !self.config.hardware_accel {
                // Software encoding options
                cmd.args(["-preset", &self.config.preset]);
                cmd.args(["-crf", &self.config.crf.to_string()]);
            } else {
                // Hardware encoding (VideoToolbox) options
                cmd.args(["-q:v", &(self.config.crf * 2).to_string()]); // VT uses different quality scale
            }

            // Pixel format
            cmd.args(["-pix_fmt", "yuv420p"]);

            // Bitrate (as fallback/max)
            cmd.args([
                "-maxrate",
                &format!("{}k", self.config.video_bitrate / 1000),
            ]);
            cmd.args([
                "-bufsize",
                &format!("{}k", self.config.video_bitrate / 500),
            ]);

            // No audio in video-only pass
            cmd.args(["-an"]);

            // Output format and path
            // Use temp video file, we'll mux with audio later
            let video_temp = output_path.with_extension("video.mp4");
            cmd.args(["-y"]); // Overwrite output
            cmd.arg(video_temp.to_str().unwrap());

            // Spawn the process
            let mut child = cmd
                .as_inner_mut()
                .stdin(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| FrameError::EncodingError(format!("Failed to start FFmpeg: {}", e)))?;

            // Take stdin for writing frames
            let stdin = child.stdin.take().ok_or_else(|| {
                FrameError::EncodingError("Failed to get FFmpeg stdin".to_string())
            })?;

            self.video_stdin = Some(stdin);
            self.ffmpeg_process = Some(child);
            self.initialized = true;

            tracing::info!(
                "Video encoder started: {}x{} @ {} fps, codec: {}",
                width,
                height,
                self.config.frame_rate,
                video_encoder
            );

            Ok(())
        }

        /// Encode a video frame
        pub fn encode_frame(&mut self, frame: &VideoFrame) -> FrameResult<()> {
            // Start encoder on first frame
            if !self.initialized {
                self.start_video_encoder(frame.width, frame.height)?;
            }

            let stdin = self
                .video_stdin
                .as_mut()
                .ok_or_else(|| FrameError::EncodingError("Encoder not started".to_string()))?;

            // Write raw BGRA frame data to ffmpeg stdin
            stdin.write_all(&frame.data).map_err(|e| {
                FrameError::EncodingError(format!("Failed to write frame: {}", e))
            })?;

            self.frame_count += 1;

            if self.frame_count % 100 == 0 {
                tracing::debug!("Encoded {} frames", self.frame_count);
            }

            Ok(())
        }

        /// Encode an audio buffer (accumulates for later muxing)
        pub fn encode_audio(&mut self, buffer: &AudioSamples) -> FrameResult<()> {
            // Store audio parameters
            self.audio_sample_rate = buffer.sample_rate;
            self.audio_channels = buffer.channels;

            // Accumulate audio samples
            self.audio_samples.extend_from_slice(&buffer.samples);

            Ok(())
        }

        /// Write accumulated audio to temp file
        fn write_audio_file(&self) -> FrameResult<Option<PathBuf>> {
            if self.audio_samples.is_empty() {
                return Ok(None);
            }

            let audio_path = self.audio_temp_path.as_ref().ok_or_else(|| {
                FrameError::EncodingError("Audio temp path not set".to_string())
            })?;

            // Write raw PCM to a WAV file
            let spec = hound::WavSpec {
                channels: self.audio_channels,
                sample_rate: self.audio_sample_rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };

            let mut writer = hound::WavWriter::create(audio_path, spec).map_err(|e| {
                FrameError::EncodingError(format!("Failed to create audio file: {}", e))
            })?;

            for sample in &self.audio_samples {
                writer.write_sample(*sample).map_err(|e| {
                    FrameError::EncodingError(format!("Failed to write audio sample: {}", e))
                })?;
            }

            writer.finalize().map_err(|e| {
                FrameError::EncodingError(format!("Failed to finalize audio file: {}", e))
            })?;

            tracing::info!(
                "Audio file written: {} samples at {} Hz",
                self.audio_samples.len(),
                self.audio_sample_rate
            );

            Ok(Some(audio_path.clone()))
        }

        /// Mux video and audio into final output
        fn mux_video_audio(&self, video_path: &Path, audio_path: Option<&Path>) -> FrameResult<()> {
            let output_path = self
                .output_path
                .as_ref()
                .ok_or_else(|| FrameError::EncodingError("Output path not set".to_string()))?;

            let mut cmd = FfmpegCommand::new();

            // Input video
            cmd.args(["-i", video_path.to_str().unwrap()]);

            if let Some(audio) = audio_path {
                // Input audio
                cmd.args(["-i", audio.to_str().unwrap()]);

                // Map both streams
                cmd.args(["-map", "0:v:0", "-map", "1:a:0"]);

                // Audio codec
                cmd.args(["-c:a", self.config.audio_codec.as_encoder()]);
                cmd.args([
                    "-b:a",
                    &format!("{}k", self.config.audio_bitrate / 1000),
                ]);
            } else {
                // No audio, just copy video
                cmd.args(["-map", "0:v:0"]);
            }

            // Copy video (already encoded)
            cmd.args(["-c:v", "copy"]);

            // Output
            cmd.args(["-y"]);
            cmd.arg(output_path.to_str().unwrap());

            // Run muxing
            let output = cmd
                .as_inner_mut()
                .output()
                .map_err(|e| FrameError::EncodingError(format!("Muxing failed: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(FrameError::EncodingError(format!(
                    "Muxing failed: {}",
                    stderr
                )));
            }

            tracing::info!("Muxing complete: {:?}", output_path);

            Ok(())
        }

        /// Finalize the video file
        pub fn finalize(&mut self) -> FrameResult<()> {
            let output_path = self
                .output_path
                .as_ref()
                .ok_or_else(|| FrameError::EncodingError("Output path not set".to_string()))?
                .clone();

            // Close video stdin to signal end of input
            if let Some(stdin) = self.video_stdin.take() {
                drop(stdin);
            }

            // Wait for ffmpeg to finish
            if let Some(mut process) = self.ffmpeg_process.take() {
                let status = process.wait().map_err(|e| {
                    FrameError::EncodingError(format!("FFmpeg process failed: {}", e))
                })?;

                if !status.success() {
                    return Err(FrameError::EncodingError(format!(
                        "FFmpeg exited with status: {}",
                        status
                    )));
                }
            }

            let video_temp = output_path.with_extension("video.mp4");

            // Write audio file if we have audio
            let audio_path = self.write_audio_file()?;

            // Mux video and audio
            if audio_path.is_some() {
                self.mux_video_audio(&video_temp, audio_path.as_deref())?;

                // Clean up temp files
                let _ = std::fs::remove_file(&video_temp);
                if let Some(audio) = audio_path {
                    let _ = std::fs::remove_file(&audio);
                }
            } else {
                // No audio, just rename video file
                std::fs::rename(&video_temp, &output_path).map_err(|e| {
                    FrameError::EncodingError(format!("Failed to rename output: {}", e))
                })?;
            }

            tracing::info!(
                "Encoding finalized: {} frames to {:?}",
                self.frame_count,
                output_path
            );

            Ok(())
        }

        /// Get encoding progress (0.0 - 1.0, based on frame count)
        pub fn progress(&self, estimated_total_frames: u64) -> f32 {
            if estimated_total_frames == 0 {
                return 0.0;
            }
            (self.frame_count as f32 / estimated_total_frames as f32).min(1.0)
        }

        /// Get the number of frames encoded so far
        pub fn frame_count(&self) -> u64 {
            self.frame_count
        }
    }

    impl Drop for Encoder {
        fn drop(&mut self) {
            // Clean up stdin
            if let Some(stdin) = self.video_stdin.take() {
                drop(stdin);
            }

            // Kill ffmpeg process if still running
            if let Some(mut process) = self.ffmpeg_process.take() {
                let _ = process.kill();
            }

            // Clean up temp files
            if let Some(output_path) = &self.output_path {
                let video_temp = output_path.with_extension("video.mp4");
                let _ = std::fs::remove_file(&video_temp);
            }
            if let Some(audio_temp) = &self.audio_temp_path {
                let _ = std::fs::remove_file(audio_temp);
            }
        }
    }
}

// Re-export based on feature
#[cfg(feature = "encoding")]
pub use sidecar_encoder::*;

// Stub implementation when encoding feature is not enabled
#[cfg(not(feature = "encoding"))]
mod stub_encoder {
    use super::{AudioSamples, VideoFrame};
    use crate::{FrameError, FrameResult};
    use std::path::Path;

    /// Video codec (stub)
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub enum VideoCodec {
        #[default]
        H264,
        H265,
    }

    /// Audio codec (stub)
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    pub enum AudioCodec {
        #[default]
        Aac,
        Opus,
    }

    /// Encoder configuration (stub)
    #[derive(Debug, Clone, Default)]
    pub struct EncoderConfig {
        pub video_codec: VideoCodec,
        pub audio_codec: AudioCodec,
        pub video_bitrate: u64,
        pub audio_bitrate: u64,
        pub frame_rate: u32,
        pub width: u32,
        pub height: u32,
        pub hardware_accel: bool,
        pub preset: String,
        pub crf: u32,
    }

    /// Encoder stub (encoding feature not enabled)
    pub struct Encoder {
        _config: EncoderConfig,
    }

    impl Encoder {
        pub fn new(config: EncoderConfig) -> FrameResult<Self> {
            Ok(Self { _config: config })
        }

        pub fn ensure_ffmpeg() -> FrameResult<()> {
            Err(FrameError::EncodingError(
                "Encoding feature not enabled. Rebuild with --features encoding".to_string(),
            ))
        }

        pub fn init(&mut self, _output_path: &Path) -> FrameResult<()> {
            Err(FrameError::EncodingError(
                "Encoding feature not enabled. Rebuild with --features encoding".to_string(),
            ))
        }

        pub fn encode_frame(&mut self, _frame: &VideoFrame) -> FrameResult<()> {
            Err(FrameError::EncodingError(
                "Encoding feature not enabled".to_string(),
            ))
        }

        pub fn encode_audio(&mut self, _buffer: &AudioSamples) -> FrameResult<()> {
            Err(FrameError::EncodingError(
                "Encoding feature not enabled".to_string(),
            ))
        }

        pub fn finalize(&mut self) -> FrameResult<()> {
            Err(FrameError::EncodingError(
                "Encoding feature not enabled".to_string(),
            ))
        }

        pub fn progress(&self, _estimated_total_frames: u64) -> f32 {
            0.0
        }

        pub fn frame_count(&self) -> u64 {
            0
        }
    }
}

#[cfg(not(feature = "encoding"))]
pub use stub_encoder::*;

// Legacy ffmpeg-next encoder (optional, behind encoding-libav feature)
#[cfg(feature = "encoding-libav")]
pub mod libav_encoder;

// GIF encoder (optional, behind gif feature)
#[cfg(feature = "gif")]
pub mod gif;

// Aspect ratio filter for applying aspect ratio transformations during export
pub use aspect_ratio_filter::*;

mod aspect_ratio_filter {
    use super::VideoFrame;
    use crate::effects::aspect_ratio::{
        calculate_letterbox, AspectRatio, ContentAlignment,
    };
    use crate::effects::Color;

    /// Filter for applying aspect ratio transformations to video frames
    ///
    /// Wraps frame processing to add letterboxing/pillarboxing while
    /// preserving content aspect ratio and applying background fills.
    #[derive(Debug, Clone)]
    pub struct AspectRatioFilter {
        /// Target aspect ratio for output frames
        target_ratio: AspectRatio,
        /// Content alignment within the output frame
        alignment: ContentAlignment,
        /// Background color for letterbox/pillarbox areas
        bg_color: Color,
        /// Target output dimensions (width, height)
        output_dimensions: (u32, u32),
    }

    impl AspectRatioFilter {
        /// Create a new aspect ratio filter
        ///
        /// # Arguments
        /// * `target_ratio` - The desired output aspect ratio
        /// * `alignment` - How to align content within the frame
        /// * `bg_color` - Background color for padding areas
        /// * `output_width` - Output frame width (0 = calculate from input)
        /// * `output_height` - Output frame height (0 = calculate from input)
        pub fn new(
            target_ratio: AspectRatio,
            alignment: ContentAlignment,
            bg_color: Color,
            output_width: u32,
            output_height: u32,
        ) -> Self {
            Self {
                target_ratio,
                alignment,
                bg_color,
                output_dimensions: (output_width, output_height),
            }
        }

        /// Create a filter with default settings (16:9, center alignment, black background)
        pub fn default_with_dimensions(width: u32, height: u32) -> Self {
            Self {
                target_ratio: AspectRatio::default(),
                alignment: ContentAlignment::default(),
                bg_color: Color::BLACK,
                output_dimensions: (width, height),
            }
        }

        /// Set the target aspect ratio
        pub fn with_target_ratio(mut self, ratio: AspectRatio) -> Self {
            self.target_ratio = ratio;
            self
        }

        /// Set the content alignment
        pub fn with_alignment(mut self, alignment: ContentAlignment) -> Self {
            self.alignment = alignment;
            self
        }

        /// Set the background color
        pub fn with_bg_color(mut self, color: Color) -> Self {
            self.bg_color = color;
            self
        }

        /// Set output dimensions
        pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
            self.output_dimensions = (width, height);
            self
        }

        /// Calculate output dimensions based on input and target ratio
        ///
        /// If output dimensions are specified (non-zero), use them.
        /// Otherwise, calculate based on input dimensions and target ratio.
        fn calculate_output_dimensions(&self, input_width: u32, input_height: u32) -> (u32, u32) {
            let (specified_width, specified_height) = self.output_dimensions;

            if specified_width > 0 && specified_height > 0 {
                // Use specified dimensions
                (specified_width, specified_height)
            } else if specified_width > 0 {
                // Calculate height from width and target ratio
                let height =
                    (specified_width as f32 / self.target_ratio.ratio_f32()).round() as u32;
                (specified_width, height)
            } else if specified_height > 0 {
                // Calculate width from height and target ratio
                let width =
                    (specified_height as f32 * self.target_ratio.ratio_f32()).round() as u32;
                (width, specified_height)
            } else {
                // Calculate output dimensions to fit input within target ratio
                // This maintains input resolution while changing aspect ratio
                let input_ratio = input_width as f32 / input_height as f32;
                let target_ratio_f = self.target_ratio.ratio_f32();

                if input_ratio > target_ratio_f {
                    // Input is wider - use input width, calculate height
                    let height = (input_width as f32 / target_ratio_f).round() as u32;
                    (input_width, height)
                } else {
                    // Input is taller - use input height, calculate width
                    let width = (input_height as f32 * target_ratio_f).round() as u32;
                    (width, input_height)
                }
            }
        }

        /// Apply aspect ratio transformation to a frame
        ///
        /// This function:
        /// 1. Calculates letterbox/pillarbox padding using calculate_letterbox()
        /// 2. Creates a new frame with target dimensions
        /// 3. Fills the frame with background color
        /// 4. Copies the source content to the appropriate position based on alignment
        /// 5. Preserves content aspect ratio (no stretching/squishing)
        ///
        /// # Arguments
        /// * `frame` - The input video frame (BGRA format)
        /// * `target` - Target aspect ratio
        /// * `alignment` - Content alignment within the output
        /// * `bg_color` - Background color for padding areas
        ///
        /// # Returns
        /// A new VideoFrame with the target aspect ratio applied
        pub fn apply_aspect_ratio(
            &self,
            frame: &VideoFrame,
            target: AspectRatio,
            alignment: ContentAlignment,
            bg_color: Color,
        ) -> VideoFrame {
            // Calculate output dimensions
            let (output_width, output_height) =
                self.calculate_output_dimensions(frame.width, frame.height);

            // Handle edge cases
            if frame.width == 0
                || frame.height == 0
                || output_width == 0
                || output_height == 0
            {
                return frame.clone();
            }

            // Calculate letterbox/pillarbox padding
            let letterbox = calculate_letterbox(
                frame.width,
                frame.height,
                target,
                alignment,
            );

            // Create new frame data filled with background color
            let mut output_data = vec![0u8; (output_width * output_height * 4) as usize];

            // Fill with background color (BGRA format)
            fill_with_color(&mut output_data, output_width, output_height, bg_color);

            // Calculate content region within output
            let (content_x, content_y, content_width, content_height) =
                letterbox.content_rect(output_width, output_height);

            // Copy source frame to content region
            // Scale if necessary to fit within content region while preserving aspect ratio
            if content_width == frame.width && content_height == frame.height {
                // No scaling needed - direct copy
                copy_frame_bgra(
                    &frame.data,
                    frame.width,
                    frame.height,
                    &mut output_data,
                    output_width,
                    output_height,
                    content_x,
                    content_y,
                );
            } else {
                // Scale content to fit within content region
                scale_and_copy_bgra(
                    &frame.data,
                    frame.width,
                    frame.height,
                    &mut output_data,
                    output_width,
                    output_height,
                    content_x,
                    content_y,
                    content_width,
                    content_height,
                );
            }

            VideoFrame {
                data: output_data,
                width: output_width,
                height: output_height,
                timestamp: frame.timestamp,
            }
        }

        /// Process a frame through the filter using current settings
        ///
        /// This is a convenience method that uses the filter's configured
        /// target ratio, alignment, and background color.
        pub fn process(&self, frame: &VideoFrame) -> VideoFrame {
            self.apply_aspect_ratio(frame, self.target_ratio, self.alignment, self.bg_color)
        }

        /// Get the target aspect ratio
        pub fn target_ratio(&self) -> AspectRatio {
            self.target_ratio
        }

        /// Get the content alignment
        pub fn alignment(&self) -> ContentAlignment {
            self.alignment
        }

        /// Get the background color
        pub fn bg_color(&self) -> Color {
            self.bg_color
        }

        /// Get the output dimensions
        pub fn output_dimensions(&self) -> (u32, u32) {
            self.output_dimensions
        }
    }

    impl Default for AspectRatioFilter {
        fn default() -> Self {
            Self {
                target_ratio: AspectRatio::default(),
                alignment: ContentAlignment::default(),
                bg_color: Color::BLACK,
                output_dimensions: (0, 0),
            }
        }
    }

    /// Fill frame data with a solid color (BGRA format)
    fn fill_with_color(data: &mut [u8], width: u32, height: u32, color: Color) {
        let r = (color.r * 255.0).round() as u8;
        let g = (color.g * 255.0).round() as u8;
        let b = (color.b * 255.0).round() as u8;
        let a = (color.a * 255.0).round() as u8;

        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                if idx + 3 < data.len() {
                    data[idx] = b;     // B
                    data[idx + 1] = g; // G
                    data[idx + 2] = r; // R
                    data[idx + 3] = a; // A
                }
            }
        }
    }

    /// Copy frame data without scaling (BGRA format)
    #[allow(clippy::too_many_arguments)]
    fn copy_frame_bgra(
        src: &[u8],
        src_width: u32,
        src_height: u32,
        dst: &mut [u8],
        dst_width: u32,
        _dst_height: u32,
        dst_x: u32,
        dst_y: u32,
    ) {
        for y in 0..src_height {
            for x in 0..src_width {
                let src_idx = ((y * src_width + x) * 4) as usize;
                let dst_idx =
                    (((dst_y + y) * dst_width + (dst_x + x)) * 4) as usize;

                if src_idx + 3 < src.len() && dst_idx + 3 < dst.len() {
                    dst[dst_idx] = src[src_idx];
                    dst[dst_idx + 1] = src[src_idx + 1];
                    dst[dst_idx + 2] = src[src_idx + 2];
                    dst[dst_idx + 3] = src[src_idx + 3];
                }
            }
        }
    }

    /// Scale and copy frame data (BGRA format with bilinear interpolation)
    #[allow(clippy::too_many_arguments)]
    fn scale_and_copy_bgra(
        src: &[u8],
        src_width: u32,
        src_height: u32,
        dst: &mut [u8],
        dst_width: u32,
        _dst_height: u32,
        dst_x: u32,
        dst_y: u32,
        target_width: u32,
        target_height: u32,
    ) {
        if target_width == 0 || target_height == 0 {
            return;
        }

        let x_ratio = src_width as f32 / target_width as f32;
        let y_ratio = src_height as f32 / target_height as f32;

        for y in 0..target_height {
            for x in 0..target_width {
                // Calculate source position
                let src_x = (x as f32 * x_ratio).min(src_width as f32 - 1.0);
                let src_y = (y as f32 * y_ratio).min(src_height as f32 - 1.0);

                // Bilinear interpolation
                let x0 = src_x as u32;
                let y0 = src_y as u32;
                let x1 = (x0 + 1).min(src_width - 1);
                let y1 = (y0 + 1).min(src_height - 1);

                let fx = src_x - x0 as f32;
                let fy = src_y - y0 as f32;

                // Sample four corners
                let idx00 = ((y0 * src_width + x0) * 4) as usize;
                let idx10 = ((y0 * src_width + x1) * 4) as usize;
                let idx01 = ((y1 * src_width + x0) * 4) as usize;
                let idx11 = ((y1 * src_width + x1) * 4) as usize;

                if idx11 + 3 >= src.len() {
                    continue;
                }

                // Interpolate each channel
                for c in 0..4 {
                    let v00 = src[idx00 + c] as f32;
                    let v10 = src[idx10 + c] as f32;
                    let v01 = src[idx01 + c] as f32;
                    let v11 = src[idx11 + c] as f32;

                    let v0 = v00 + fx * (v10 - v00);
                    let v1 = v01 + fx * (v11 - v01);
                    let v = v0 + fy * (v1 - v0);

                    let dst_idx = (((dst_y + y) * dst_width + (dst_x + x)) * 4) as usize;
                    if dst_idx + c < dst.len() {
                        dst[dst_idx + c] = v.round() as u8;
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::Duration;

        fn create_test_frame(width: u32, height: u32) -> VideoFrame {
            // Create a test frame with a gradient pattern
            let mut data = vec![0u8; (width * height * 4) as usize];
            for y in 0..height {
                for x in 0..width {
                    let idx = ((y * width + x) * 4) as usize;
                    data[idx] = (x % 256) as u8;     // B
                    data[idx + 1] = (y % 256) as u8; // G
                    data[idx + 2] = 128;             // R
                    data[idx + 3] = 255;             // A
                }
            }
            VideoFrame {
                data,
                width,
                height,
                timestamp: Duration::from_millis(0),
            }
        }

        #[test]
        fn test_aspect_ratio_filter_creation() {
            let filter = AspectRatioFilter::new(
                AspectRatio::Square,
                ContentAlignment::Center,
                Color::WHITE,
                1080,
                1080,
            );

            assert_eq!(filter.target_ratio(), AspectRatio::Square);
            assert_eq!(filter.alignment(), ContentAlignment::Center);
            assert_eq!(filter.bg_color(), Color::WHITE);
            assert_eq!(filter.output_dimensions(), (1080, 1080));
        }

        #[test]
        fn test_aspect_ratio_filter_builder_methods() {
            let filter = AspectRatioFilter::default()
                .with_target_ratio(AspectRatio::Vertical9x16)
                .with_alignment(ContentAlignment::Top)
                .with_bg_color(Color::BLACK)
                .with_dimensions(1080, 1920);

            assert_eq!(filter.target_ratio(), AspectRatio::Vertical9x16);
            assert_eq!(filter.alignment(), ContentAlignment::Top);
            assert_eq!(filter.bg_color(), Color::BLACK);
            assert_eq!(filter.output_dimensions(), (1080, 1920));
        }

        #[test]
        fn test_apply_aspect_ratio_letterbox() {
            // 16:9 content (1920x1080) into 1:1 square (1080x1080)
            // Should add letterbox bars (top/bottom padding)
            let frame = create_test_frame(1920, 1080);
            let filter = AspectRatioFilter::default_with_dimensions(1080, 1080)
                .with_target_ratio(AspectRatio::Square);

            let result = filter.process(&frame);

            assert_eq!(result.width, 1080);
            assert_eq!(result.height, 1080);
            assert_eq!(result.data.len(), 1080 * 1080 * 4);

            // Verify content is centered (check that we have black bars on top/bottom)
            // Top-left pixel should be black (background color)
            let top_left_idx = 0;
            assert_eq!(result.data[top_left_idx], 0);     // B
            assert_eq!(result.data[top_left_idx + 1], 0); // G
            assert_eq!(result.data[top_left_idx + 2], 0); // R
        }

        #[test]
        fn test_apply_aspect_ratio_pillarbox() {
            // 1:1 content (1080x1080) into 16:9 container (1920x1080)
            // Should add pillarbox bars (left/right padding)
            let frame = create_test_frame(1080, 1080);
            let filter = AspectRatioFilter::default_with_dimensions(1920, 1080);

            let result = filter.process(&frame);

            assert_eq!(result.width, 1920);
            assert_eq!(result.height, 1080);
            assert_eq!(result.data.len(), 1920 * 1080 * 4);

            // Top-left pixel should be black (pillarbox on left)
            let top_left_idx = 0;
            assert_eq!(result.data[top_left_idx], 0);     // B
            assert_eq!(result.data[top_left_idx + 1], 0); // G
            assert_eq!(result.data[top_left_idx + 2], 0); // R
        }

        #[test]
        fn test_apply_aspect_ratio_no_padding() {
            // 16:9 content into 16:9 container - no padding needed
            let frame = create_test_frame(1920, 1080);
            let filter = AspectRatioFilter::default_with_dimensions(1920, 1080)
                .with_target_ratio(AspectRatio::Horizontal16x9);

            let result = filter.process(&frame);

            assert_eq!(result.width, 1920);
            assert_eq!(result.height, 1080);

            // Content should be preserved (no black bars)
            // Check a pixel in the middle - should not be black
            let mid_idx = ((540 * 1920 + 960) * 4) as usize;
            // The pixel should have non-zero values from our gradient
            let is_black = result.data[mid_idx] == 0
                && result.data[mid_idx + 1] == 0
                && result.data[mid_idx + 2] == 0;
            assert!(!is_black, "Content should not have black bars when aspect ratios match");
        }

        #[test]
        fn test_apply_aspect_ratio_alignment_top() {
            // 16:9 content into 1:1 square with top alignment
            let frame = create_test_frame(1920, 1080);
            let filter = AspectRatioFilter::default_with_dimensions(1080, 1080)
                .with_target_ratio(AspectRatio::Square)
                .with_alignment(ContentAlignment::Top);

            let result = filter.process(&frame);

            assert_eq!(result.width, 1080);
            assert_eq!(result.height, 1080);

            // With top alignment, the top should have content, not black
            // Check a pixel near the top - should have gradient content
            let top_idx = (100 * 1080 * 4) as usize; // Row 100
            let has_content = result.data[top_idx] != 0
                || result.data[top_idx + 1] != 0
                || result.data[top_idx + 2] != 0;
            assert!(has_content, "Top-aligned content should appear at top");
        }

        #[test]
        fn test_apply_aspect_ratio_alignment_bottom() {
            // 16:9 content into 1:1 square with bottom alignment
            let frame = create_test_frame(1920, 1080);
            let filter = AspectRatioFilter::default_with_dimensions(1080, 1080)
                .with_target_ratio(AspectRatio::Square)
                .with_alignment(ContentAlignment::Bottom);

            let result = filter.process(&frame);

            // With bottom alignment, the bottom should have content
            let bottom_idx = ((1000 * 1080 + 100) * 4) as usize; // Near bottom
            let has_content = result.data[bottom_idx] != 0
                || result.data[bottom_idx + 1] != 0
                || result.data[bottom_idx + 2] != 0;
            assert!(has_content, "Bottom-aligned content should appear at bottom");
        }

        #[test]
        fn test_apply_aspect_ratio_custom_bg_color() {
            let frame = create_test_frame(1920, 1080);
            let red = Color::rgb(1.0, 0.0, 0.0);
            let filter = AspectRatioFilter::default_with_dimensions(1080, 1080)
                .with_target_ratio(AspectRatio::Square)
                .with_bg_color(red);

            let result = filter.process(&frame);

            // Top-left pixel should be red (background color)
            let top_left_idx = 0;
            assert_eq!(result.data[top_left_idx], 0);     // B = 0
            assert_eq!(result.data[top_left_idx + 1], 0); // G = 0
            assert_eq!(result.data[top_left_idx + 2], 255); // R = 255
        }

        #[test]
        fn test_apply_aspect_ratio_preserves_timestamp() {
            let mut frame = create_test_frame(100, 100);
            frame.timestamp = Duration::from_millis(1234);

            let filter = AspectRatioFilter::default_with_dimensions(200, 200);
            let result = filter.process(&frame);

            assert_eq!(result.timestamp, Duration::from_millis(1234));
        }

        #[test]
        fn test_apply_aspect_ratio_zero_input() {
            // Zero dimensions should return clone of input
            let frame = create_test_frame(0, 1080);
            let filter = AspectRatioFilter::default_with_dimensions(1080, 1080);

            let result = filter.process(&frame);

            assert_eq!(result.width, 0);
            assert_eq!(result.height, 1080);
        }

        #[test]
        fn test_calculate_output_dimensions() {
            // Both dimensions specified
            let filter = AspectRatioFilter::default()
                .with_dimensions(1920, 1080);
            let (w, h) = filter.calculate_output_dimensions(100, 100);
            assert_eq!(w, 1920);
            assert_eq!(h, 1080);

            // Only width specified - calculate height from target ratio
            let filter = AspectRatioFilter::default()
                .with_target_ratio(AspectRatio::Horizontal16x9)
                .with_dimensions(1920, 0);
            let (w, h) = filter.calculate_output_dimensions(100, 100);
            assert_eq!(w, 1920);
            assert_eq!(h, 1080); // 1920 / (16/9) = 1080

            // Only height specified - calculate width from target ratio
            let filter = AspectRatioFilter::default()
                .with_target_ratio(AspectRatio::Horizontal16x9)
                .with_dimensions(0, 1080);
            let (w, h) = filter.calculate_output_dimensions(100, 100);
            assert_eq!(w, 1920); // 1080 * (16/9) = 1920
            assert_eq!(h, 1080);

            // Neither specified - derive from input
            let filter = AspectRatioFilter::default()
                .with_target_ratio(AspectRatio::Square)
                .with_dimensions(0, 0);
            // Wider input
            let (w, h) = filter.calculate_output_dimensions(1920, 1080);
            assert_eq!(w, 1920);
            assert_eq!(h, 1920); // Match input width, calculate height for square
        }

        #[test]
        fn test_fill_with_color() {
            let mut data = vec![0u8; 100 * 100 * 4];
            let color = Color::rgba_u8(255, 128, 64, 200);

            fill_with_color(&mut data, 100, 100, color);

            // Check a pixel in the middle
            let idx = (50 * 100 + 50) * 4;
            assert_eq!(data[idx], 64);     // B
            assert_eq!(data[idx + 1], 128); // G
            assert_eq!(data[idx + 2], 255); // R
            assert_eq!(data[idx + 3], 200); // A
        }

        #[test]
        fn test_copy_frame_bgra() {
            let src = vec![
                10, 20, 30, 255, // Pixel 0: BGRA
                40, 50, 60, 255, // Pixel 1
                70, 80, 90, 255, // Pixel 2
                100, 110, 120, 255, // Pixel 3
            ];
            let mut dst = vec![0u8; 4 * 4 * 4]; // 4x4 destination

            copy_frame_bgra(&src, 2, 2, &mut dst, 4, 4, 1, 1);

            // Check pixel at (1,1) in destination
            let idx = (1 * 4 + 1) * 4;
            assert_eq!(dst[idx], 10);     // B from src pixel 0
            assert_eq!(dst[idx + 1], 20); // G
            assert_eq!(dst[idx + 2], 30); // R
            assert_eq!(dst[idx + 3], 255); // A
        }

        #[test]
        fn test_default_aspect_ratio_filter() {
            let filter = AspectRatioFilter::default();

            assert_eq!(filter.target_ratio(), AspectRatio::Horizontal16x9);
            assert_eq!(filter.alignment(), ContentAlignment::Center);
            assert_eq!(filter.bg_color(), Color::BLACK);
            assert_eq!(filter.output_dimensions(), (0, 0));
        }
    }
}
