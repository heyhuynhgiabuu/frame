//! Recording service that manages capture lifecycle

use frame_core::capture::{create_capture, CaptureArea, CaptureConfig, ScreenCapture};
use frame_core::encoder::{Encoder, EncoderConfig, VideoCodec};
use frame_core::{FrameError, FrameResult};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Recording configuration
#[derive(Debug, Clone)]
pub struct RecordingConfig {
    pub capture_area: CaptureArea,
    pub capture_cursor: bool,
    pub capture_audio: bool,
    pub frame_rate: u32,
    pub output_path: PathBuf,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            capture_area: CaptureArea::FullScreen,
            capture_cursor: true,
            capture_audio: true,
            frame_rate: 30,
            output_path: PathBuf::from("recording.mp4"),
        }
    }
}

/// Recording session state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecordingState {
    Idle,
    Initializing,
    Recording,
    Stopping,
    Error,
}

/// Recording service that manages the full recording lifecycle
pub struct RecordingService {
    state: RecordingState,
    capture: Option<Arc<Mutex<Box<dyn ScreenCapture>>>>,
    encoder: Option<Encoder>,
    config: RecordingConfig,
    frame_count: u64,
}

impl RecordingService {
    pub fn new() -> Self {
        Self {
            state: RecordingState::Idle,
            capture: None,
            encoder: None,
            config: RecordingConfig::default(),
            frame_count: 0,
        }
    }

    /// Check if screen recording permission is granted
    pub async fn check_screen_permission() -> bool {
        // On macOS, try to create a capture instance to test permission
        // The ScreenCaptureKit APIs will fail if permission is not granted
        #[cfg(target_os = "macos")]
        {
            // Try to create a capture instance - this will fail if no permission
            frame_core::capture::create_capture().is_ok()
        }
        #[cfg(not(target_os = "macos"))]
        {
            true
        }
    }

    /// Request screen recording permission
    /// On macOS, this opens System Preferences and returns immediately
    /// User needs to manually grant permission and restart the app
    pub async fn request_screen_permission() {
        #[cfg(target_os = "macos")]
        {
            // Open System Preferences to Screen Recording
            let _ = std::process::Command::new("open")
                .args([
                    "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenRecording",
                ])
                .spawn();
        }
    }

    /// Check microphone permission
    pub async fn check_microphone_permission() -> bool {
        #[cfg(target_os = "macos")]
        {
            // For now, assume microphone permission is granted
            // In production, use AVFoundation to check
            true
        }
        #[cfg(not(target_os = "macos"))]
        {
            true
        }
    }

    /// Request microphone permission
    pub async fn request_microphone_permission() {
        #[cfg(target_os = "macos")]
        {
            // Open System Preferences to Microphone
            let _ = std::process::Command::new("open")
                .args([
                    "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
                ])
                .spawn();
        }
    }

    /// Get current recording state
    pub fn state(&self) -> RecordingState {
        self.state
    }

    /// Get frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Start recording
    pub async fn start_recording(&mut self, config: RecordingConfig) -> FrameResult<()> {
        if self.state != RecordingState::Idle {
            return Err(FrameError::RecordingInProgress);
        }

        self.state = RecordingState::Initializing;
        self.config = config;
        self.frame_count = 0;

        // Create capture instance
        let capture = create_capture()?;
        let capture = Arc::new(Mutex::new(capture));

        // Create encoder
        let encoder_config = EncoderConfig {
            video_codec: VideoCodec::H264,
            frame_rate: self.config.frame_rate,
            hardware_accel: true,
            ..Default::default()
        };
        let mut encoder = Encoder::new(encoder_config)?;
        encoder.init(&self.config.output_path)?;

        // Start capture
        let capture_config = CaptureConfig {
            capture_area: self.config.capture_area.clone(),
            capture_cursor: self.config.capture_cursor,
            capture_audio: self.config.capture_audio,
            frame_rate: self.config.frame_rate,
        };

        {
            let mut cap = capture.lock().await;
            cap.start(capture_config).await?;
        }

        self.capture = Some(capture);
        self.encoder = Some(encoder);
        self.state = RecordingState::Recording;

        // Start capture loop
        self.run_capture_loop().await;

        Ok(())
    }

    /// Run the capture loop (spawns async task)
    async fn run_capture_loop(&mut self) {
        // This would spawn a task to continuously capture frames
        // For now, simplified version
    }

    /// Stop recording
    pub async fn stop_recording(&mut self) -> FrameResult<PathBuf> {
        if self.state != RecordingState::Recording {
            return Err(FrameError::NoRecordingInProgress);
        }

        self.state = RecordingState::Stopping;

        // Stop capture
        if let Some(capture) = &self.capture {
            let mut cap = capture.lock().await;
            cap.stop().await?;
        }

        // Finalize encoder
        if let Some(mut encoder) = self.encoder.take() {
            encoder.finalize()?;
        }

        self.capture = None;
        self.state = RecordingState::Idle;

        Ok(self.config.output_path.clone())
    }

    /// Capture a single frame (for preview)
    pub async fn capture_preview_frame(
        &mut self,
    ) -> FrameResult<Option<frame_core::capture::Frame>> {
        if let Some(capture) = &self.capture {
            let mut cap = capture.lock().await;
            cap.next_frame().await
        } else {
            Ok(None)
        }
    }
}

impl Default for RecordingService {
    fn default() -> Self {
        Self::new()
    }
}
