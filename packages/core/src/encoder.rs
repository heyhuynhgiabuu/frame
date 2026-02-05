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
