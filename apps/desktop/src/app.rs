//! Main application with recording controls and permission handling

use crate::recording::{RecordingConfig, RecordingService};
use crate::ui::main_view;
use frame_core::capture::CaptureArea;
use iced::{executor, time, Application, Command, Element, Subscription, Theme};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tracing::{debug, error, info};

pub struct FrameApp {
    pub state: AppState,
    pub theme: Theme,
    pub recording_service: RecordingService,
    pub recording_config: RecordingConfig,
    pub frame_count: u64,
    pub start_time: Option<Instant>,
    pub permissions: Permissions,
}

/// Permission states
#[derive(Debug, Clone, Default)]
pub struct Permissions {
    pub screen_granted: bool,
    pub microphone_granted: bool,
    pub checking: bool,
}

#[derive(Debug, Clone)]
pub enum AppState {
    /// Initial permission check
    CheckingPermissions,
    /// Permission denied, needs user action
    PermissionRequired { screen: bool, microphone: bool },
    /// Ready to record
    Idle,
    /// Recording in progress
    Recording,
    /// Previewing recorded video
    Previewing { project_id: String, path: PathBuf },
    /// Exporting video
    Exporting { project_id: String, progress: f32 },
    /// Error state
    Error(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    // Permission handling
    CheckPermissions,
    PermissionsChecked {
        screen: bool,
        microphone: bool,
    },
    RequestScreenPermission,
    RequestMicrophonePermission,
    PermissionGranted,
    PermissionDenied,

    // Recording controls
    StartRecording,
    StopRecording,
    RecordingStarted,
    RecordingStopped {
        project_id: String,
        path: PathBuf,
    },
    RecordingError(String),
    UpdateRecordingStats,

    // Export
    ExportProject(String),
    ExportProgress(f32),
    ExportComplete,

    // UI
    ThemeChanged(Theme),
    SettingsOpened,
    ConfigureRecording {
        capture_area: CaptureArea,
        capture_audio: bool,
    },
}

impl Application for FrameApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        info!("Initializing Frame application");

        (
            Self {
                state: AppState::CheckingPermissions,
                theme: Theme::Dark,
                recording_service: RecordingService::new(),
                recording_config: RecordingConfig::default(),
                frame_count: 0,
                start_time: None,
                permissions: Permissions::default(),
            },
            Command::perform(async {}, |_| Message::CheckPermissions),
        )
    }

    fn title(&self) -> String {
        match &self.state {
            AppState::CheckingPermissions => "Frame - Checking Permissions...".to_string(),
            AppState::PermissionRequired { .. } => "Frame - Permission Required".to_string(),
            AppState::Idle => "Frame - Screen Recorder".to_string(),
            AppState::Recording => {
                let duration = self
                    .start_time
                    .map(|t| format_duration(t.elapsed()))
                    .unwrap_or_default();
                format!("Frame - Recording {}", duration)
            }
            AppState::Previewing { project_id, .. } => {
                format!("Frame - Preview: {}", project_id)
            }
            AppState::Exporting {
                project_id,
                progress,
            } => {
                format!(
                    "Frame - Exporting: {} ({:.0}%)",
                    project_id,
                    progress * 100.0
                )
            }
            AppState::Error(msg) => format!("Frame - Error: {}", msg),
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        debug!("Handling message: {:?}", message);

        match message {
            // Permission handling
            Message::CheckPermissions => {
                self.permissions.checking = true;
                Command::perform(
                    async {
                        let screen = RecordingService::check_screen_permission().await;
                        let microphone = RecordingService::check_microphone_permission().await;
                        Message::PermissionsChecked { screen, microphone }
                    },
                    |msg| msg,
                )
            }
            Message::PermissionsChecked { screen, microphone } => {
                self.permissions.checking = false;
                self.permissions.screen_granted = screen;
                self.permissions.microphone_granted = microphone;

                if screen && microphone {
                    self.state = AppState::Idle;
                } else {
                    self.state = AppState::PermissionRequired {
                        screen: !screen,
                        microphone: !microphone,
                    };
                }
                Command::none()
            }
            Message::RequestScreenPermission => Command::perform(
                async {
                    RecordingService::request_screen_permission().await;
                    Message::CheckPermissions
                },
                |msg| msg,
            ),
            Message::RequestMicrophonePermission => Command::perform(
                async {
                    RecordingService::request_microphone_permission().await;
                    Message::CheckPermissions
                },
                |msg| msg,
            ),
            Message::PermissionGranted | Message::PermissionDenied => {
                // Refresh permissions
                Command::perform(async {}, |_| Message::CheckPermissions)
            }

            // Recording controls
            Message::StartRecording => {
                info!("Starting recording session");
                self.state = AppState::Recording;
                self.start_time = Some(Instant::now());
                self.frame_count = 0;

                let config = self.recording_config.clone();
                Command::perform(
                    async move {
                        let mut service = RecordingService::new();
                        match service.start_recording(config).await {
                            Ok(()) => Message::RecordingStarted,
                            Err(e) => Message::RecordingError(e.to_string()),
                        }
                    },
                    |msg| msg,
                )
            }
            Message::StopRecording => {
                info!("Stopping recording session");
                Command::perform(
                    async move {
                        let mut service = RecordingService::new();
                        match service.stop_recording().await {
                            Ok(path) => {
                                let project_id = uuid::Uuid::new_v4().to_string();
                                Message::RecordingStopped { project_id, path }
                            }
                            Err(e) => Message::RecordingError(e.to_string()),
                        }
                    },
                    |msg| msg,
                )
            }
            Message::RecordingStarted => {
                debug!("Recording started successfully");
                Command::none()
            }
            Message::RecordingStopped { project_id, path } => {
                info!("Recording stopped, project: {} at {:?}", project_id, path);
                self.state = AppState::Previewing { project_id, path };
                self.start_time = None;
                Command::none()
            }
            Message::RecordingError(error) => {
                error!("Recording error: {}", error);
                self.state = AppState::Error(error);
                self.start_time = None;
                Command::none()
            }
            Message::UpdateRecordingStats => {
                if matches!(self.state, AppState::Recording) {
                    self.frame_count = self.recording_service.frame_count();
                }
                Command::none()
            }

            // Export
            Message::ExportProject(project_id) => {
                self.state = AppState::Exporting {
                    project_id,
                    progress: 0.0,
                };
                Command::none()
            }
            Message::ExportProgress(progress) => {
                if let AppState::Exporting { project_id, .. } = &self.state {
                    self.state = AppState::Exporting {
                        project_id: project_id.clone(),
                        progress,
                    };
                }
                Command::none()
            }
            Message::ExportComplete => {
                self.state = AppState::Idle;
                Command::none()
            }

            // UI
            Message::ThemeChanged(theme) => {
                self.theme = theme;
                Command::none()
            }
            Message::SettingsOpened => {
                // Open settings dialog
                Command::none()
            }
            Message::ConfigureRecording {
                capture_area,
                capture_audio,
            } => {
                self.recording_config.capture_area = capture_area;
                self.recording_config.capture_audio = capture_audio;
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        main_view(self)
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<Message> {
        // Update recording stats every second while recording
        match self.state {
            AppState::Recording => {
                time::every(Duration::from_secs(1)).map(|_| Message::UpdateRecordingStats)
            }
            _ => Subscription::none(),
        }
    }
}

/// Format duration as MM:SS
fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let mins = secs / 60;
    let secs = secs % 60;
    format!("{:02}:{:02}", mins, secs)
}

impl Default for FrameApp {
    fn default() -> Self {
        let (app, _) = Self::new(());
        app
    }
}
