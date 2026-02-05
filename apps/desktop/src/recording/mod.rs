//! Recording service that manages capture lifecycle with auto-save support

use frame_core::auto_save::AutoSaveService;
use frame_core::capture::{create_capture, CaptureArea, CaptureConfig, ScreenCapture};
use frame_core::encoder::{Encoder, EncoderConfig, VideoCodec};
use frame_core::project::{Recording, RecordingState};
use frame_core::{FrameError, FrameResult};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::info;

/// Recording configuration
#[derive(Debug, Clone)]
pub struct RecordingConfig {
    pub capture_area: CaptureArea,
    pub capture_cursor: bool,
    pub capture_audio: bool,
    pub frame_rate: u32,
    pub output_path: PathBuf,
    pub project_name: Option<String>,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            capture_area: CaptureArea::FullScreen,
            capture_cursor: true,
            capture_audio: true,
            frame_rate: 30,
            output_path: PathBuf::from("recording.mp4"),
            project_name: None,
        }
    }
}

/// Recording session state
#[allow(dead_code)] // Enum variants for state machine completeness
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecordingSessionState {
    Idle,
    Initializing,
    Recording,
    Stopping,
    Error,
}

/// Recording service that manages the full recording lifecycle with auto-save
pub struct RecordingService {
    state: RecordingSessionState,
    capture: Option<Arc<Mutex<Box<dyn ScreenCapture>>>>,
    encoder: Option<Encoder>,
    config: RecordingConfig,
    frame_count: u64,
    auto_save: AutoSaveService,
    start_time: Option<Instant>,
    recording_id: Option<String>,
}

impl RecordingService {
    pub fn new() -> Self {
        Self {
            state: RecordingSessionState::Idle,
            capture: None,
            encoder: None,
            config: RecordingConfig::default(),
            frame_count: 0,
            auto_save: AutoSaveService::new(),
            start_time: None,
            recording_id: None,
        }
    }

    /// Check if screen recording permission is granted
    pub async fn check_screen_permission() -> bool {
        #[cfg(target_os = "macos")]
        {
            frame_core::capture::create_capture().is_ok()
        }
        #[cfg(not(target_os = "macos"))]
        {
            true
        }
    }

    /// Request screen recording permission
    pub async fn request_screen_permission() {
        #[cfg(target_os = "macos")]
        {
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
            let _ = std::process::Command::new("open")
                .args([
                    "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
                ])
                .spawn();
        }
    }

    /// Get current recording state
    #[allow(dead_code)] // Reserved for future use
    pub fn state(&self) -> RecordingSessionState {
        self.state
    }

    /// Get frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get recording duration
    #[allow(dead_code)] // Reserved for future use
    pub fn duration(&self) -> Option<Duration> {
        self.start_time.map(|t| t.elapsed())
    }

    /// Get the current project ID if recording
    #[allow(dead_code)] // Reserved for future use
    pub fn current_project_id(&self) -> Option<&str> {
        self.auto_save.current_project().map(|p| p.id.as_str())
    }

    /// Check if auto-save is enabled
    #[allow(dead_code)] // Reserved for future use
    pub fn auto_save_enabled(&self) -> bool {
        self.auto_save.is_enabled()
    }

    /// Start recording with auto-save
    pub async fn start_recording(&mut self, config: RecordingConfig) -> FrameResult<String> {
        if self.state != RecordingSessionState::Idle {
            return Err(FrameError::RecordingInProgress);
        }

        self.state = RecordingSessionState::Initializing;
        self.config = config;
        self.frame_count = 0;
        self.start_time = Some(Instant::now());
        self.recording_id = Some(uuid::Uuid::new_v4().to_string());

        // Start auto-save for this project
        let project_name = self.config.project_name.clone().unwrap_or_else(|| {
            format!(
                "Recording {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M")
            )
        });

        let project = self.auto_save.start_project(&project_name)?;

        // Mark as incomplete for crash recovery
        self.auto_save.mark_incomplete()?;

        // Update project recording state
        if let Some(proj) = self.auto_save.current_project() {
            let mut proj = proj.clone();
            proj.recording_state = RecordingState::Recording;
            proj.save()?;
        }

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
            capture_region: None, // Phase 5: Region selection will be handled via capture_area
        };

        {
            let mut cap = capture.lock().await;
            cap.start(capture_config).await?;
        }

        self.capture = Some(capture);
        self.encoder = Some(encoder);
        self.state = RecordingSessionState::Recording;

        // Note: Auto-save runs in the background via the service
        // We keep the service reference for stop_recording to access

        info!(
            "Recording started with auto-save: project_id={}",
            project.id
        );
        Ok(project.id)
    }

    /// Stop recording and finalize auto-save
    pub async fn stop_recording(&mut self) -> FrameResult<(String, PathBuf)> {
        if self.state != RecordingSessionState::Recording {
            return Err(FrameError::NoRecordingInProgress);
        }

        self.state = RecordingSessionState::Stopping;

        // Stop capture
        if let Some(capture) = &self.capture {
            let mut cap = capture.lock().await;
            cap.stop().await?;
        }

        // Finalize encoder
        if let Some(mut encoder) = self.encoder.take() {
            encoder.finalize()?;
        }

        // Create recording entry
        let recording = Recording {
            id: self.recording_id.take().unwrap_or_default(),
            started_at: self
                .start_time
                .map(|t| {
                    chrono::DateTime::from_timestamp(t.elapsed().as_secs() as i64, 0)
                        .unwrap_or_else(chrono::Utc::now)
                })
                .unwrap_or_else(chrono::Utc::now),
            duration_ms: self
                .start_time
                .map(|t| t.elapsed().as_millis() as u64)
                .unwrap_or(0),
            file_path: self.config.output_path.clone(),
            has_video: true,
            has_audio: self.config.capture_audio,
            resolution: frame_core::project::Resolution::Hd1080,
            frame_rate: self.config.frame_rate,
        };

        // Add recording to project and finalize
        // Note: We need to create a new auto-save service since we moved it
        let mut finalizer = AutoSaveService::new();
        if let Some(project) = self.auto_save.current_project() {
            finalizer.current_project = Some(project.clone());
            finalizer.add_recording(recording).await?;

            if let Some(project) = finalizer.finalize_project().await? {
                info!("Recording stopped and project finalized: {}", project.id);
            }
        }

        self.capture = None;
        self.state = RecordingSessionState::Idle;

        let project_id = finalizer
            .current_project()
            .map(|p| p.id.clone())
            .unwrap_or_default();

        Ok((project_id, self.config.output_path.clone()))
    }

    /// Capture a single frame (for preview)
    #[allow(dead_code)] // Reserved for preview implementation
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

    /// Check for incomplete recordings and return recovery info
    pub fn check_for_incomplete_recordings() -> FrameResult<Vec<PathBuf>> {
        use frame_core::auto_save::RecoveryService;
        RecoveryService::find_incomplete_projects()
    }

    /// Recover an incomplete recording
    pub fn recover_incomplete_recording(project_dir: &PathBuf) -> FrameResult<Option<PathBuf>> {
        use frame_core::auto_save::RecoveryService;

        if let Some(project) = RecoveryService::load_incomplete_project(project_dir)? {
            info!(
                "Found incomplete recording: {} at {:?}",
                project.id, project_dir
            );

            // Get the last recording file path
            if let Some(recording) = project.recordings.last() {
                let path = recording.file_path.clone();
                RecoveryService::mark_recovered(project_dir)?;
                return Ok(Some(path));
            }
        }

        Ok(None)
    }

    /// Delete an incomplete recording
    pub fn delete_incomplete_recording(project_dir: &PathBuf) -> FrameResult<()> {
        use frame_core::auto_save::RecoveryService;
        RecoveryService::delete_incomplete_project(project_dir)
    }
}

impl Default for RecordingService {
    fn default() -> Self {
        Self::new()
    }
}
