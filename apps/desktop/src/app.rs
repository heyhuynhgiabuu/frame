//! Main application with recording controls and permission handling

use crate::recording::{RecordingConfig, RecordingService};
use crate::ui::main_view;
use frame_core::capture::CaptureArea;
use frame_core::{FrameError, RecoveryAction};
use frame_ui::error_dialog::{ErrorDialog, ErrorDialogMessage};
use frame_ui::export_dialog::{ExportDialog, ExportDialogMessage};
use frame_ui::timeline::Timeline;
use iced::{executor, time, Application, Command, Element, Subscription, Theme};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tracing::{debug, error, info};

const AUTO_SAVE_INTERVAL_SECS: u64 = 10;

pub struct FrameApp {
    pub state: AppState,
    pub theme: Theme,
    pub recording_service: RecordingService,
    pub recording_config: RecordingConfig,
    pub frame_count: u64,
    pub start_time: Option<Instant>,
    pub permissions: Permissions,
    pub timeline: Option<Timeline>,
    pub export_dialog: ExportDialog,
    pub error_dialog: ErrorDialog,
    pub current_project_id: Option<String>,
    pub incomplete_recordings: Vec<PathBuf>,
    pub auto_save_status: AutoSaveStatus,
}

#[derive(Debug, Clone, Default)]
pub struct AutoSaveStatus {
    pub last_save: Option<Instant>,
    pub is_saving: bool,
    pub save_count: u32,
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
    /// Configuring export settings
    ExportConfiguring { project_id: String, path: PathBuf },
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
    RecordingStarted {
        project_id: String,
    },
    RecordingStopped {
        project_id: String,
        path: PathBuf,
    },
    RecordingError(String),
    UpdateRecordingStats,

    // Auto-save
    AutoSaveTick,
    AutoSaveComplete,

    // Recovery
    CheckIncompleteRecordings,
    IncompleteRecordingsFound(Vec<PathBuf>),
    RecoverRecording(PathBuf),
    DeleteIncompleteRecording(PathBuf),
    RecoveryComplete(PathBuf),

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

    // Timeline
    TimelinePositionChanged(Duration),

    // Export Dialog
    ExportDialogMessage(ExportDialogMessage),

    // Error Dialog
    ErrorDialogMessage(ErrorDialogMessage),
    /// Show error dialog with error
    ShowError(String),
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
                timeline: None,
                export_dialog: ExportDialog::default(),
                error_dialog: ErrorDialog::new(),
                current_project_id: None,
                incomplete_recordings: Vec::new(),
                auto_save_status: AutoSaveStatus::default(),
            },
            Command::batch([
                Command::perform(async {}, |_| Message::CheckPermissions),
                Command::perform(async {}, |_| Message::CheckIncompleteRecordings),
            ]),
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
            AppState::ExportConfiguring { project_id, .. } => {
                format!("Frame - Export Config: {}", project_id)
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
                info!("Starting recording session with auto-save");
                self.state = AppState::Recording;
                self.start_time = Some(Instant::now());
                self.frame_count = 0;

                let config = self.recording_config.clone();
                Command::perform(
                    async move {
                        let mut service = RecordingService::new();
                        match service.start_recording(config).await {
                            Ok(project_id) => Message::RecordingStarted { project_id },
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
                            Ok((project_id, path)) => {
                                Message::RecordingStopped { project_id, path }
                            }
                            Err(e) => Message::RecordingError(e.to_string()),
                        }
                    },
                    |msg| msg,
                )
            }
            Message::RecordingStarted { project_id } => {
                debug!(
                    "Recording started with auto-save: project_id={}",
                    project_id
                );
                self.current_project_id = Some(project_id);
                Command::none()
            }
            Message::RecordingStopped { project_id, path } => {
                info!("Recording stopped, project: {} at {:?}", project_id, path);
                self.current_project_id = None;
                self.auto_save_status.last_save = None;

                // Create timeline for the recording
                let mut timeline = Timeline::new(Duration::from_secs(30)); // TODO: Get actual duration
                                                                           // Add a clip representing the full recording
                timeline.add_clip(frame_ui::timeline::Clip {
                    start: Duration::ZERO,
                    end: Duration::from_secs(30),
                    color: iced::Color::from_rgb(0.3, 0.6, 1.0),
                    label: Some("Recording".to_string()),
                });
                self.timeline = Some(timeline);
                self.state = AppState::Previewing { project_id, path };
                self.start_time = None;
                Command::none()
            }

            // Auto-save
            Message::AutoSaveTick => {
                if matches!(self.state, AppState::Recording) {
                    self.auto_save_status.is_saving = true;
                    Command::perform(async {}, |_| Message::AutoSaveComplete)
                } else {
                    Command::none()
                }
            }
            Message::AutoSaveComplete => {
                self.auto_save_status.is_saving = false;
                self.auto_save_status.last_save = Some(Instant::now());
                self.auto_save_status.save_count += 1;
                debug!(
                    "Auto-save completed, count: {}",
                    self.auto_save_status.save_count
                );
                Command::none()
            }

            // Recovery
            Message::CheckIncompleteRecordings => Command::perform(
                async {
                    match RecordingService::check_for_incomplete_recordings() {
                        Ok(incomplete) => Message::IncompleteRecordingsFound(incomplete),
                        Err(_) => Message::IncompleteRecordingsFound(Vec::new()),
                    }
                },
                |msg| msg,
            ),
            Message::IncompleteRecordingsFound(incomplete) => {
                if !incomplete.is_empty() {
                    info!("Found {} incomplete recordings", incomplete.len());
                    self.incomplete_recordings = incomplete;
                    // TODO: Show recovery dialog
                }
                Command::none()
            }
            Message::RecoverRecording(path) => Command::perform(
                async move {
                    match RecordingService::recover_incomplete_recording(&path) {
                        Ok(Some(recovered_path)) => Message::RecoveryComplete(recovered_path),
                        Ok(None) => {
                            Message::RecordingError("No recording found to recover".to_string())
                        }
                        Err(e) => Message::RecordingError(e.to_string()),
                    }
                },
                |msg| msg,
            ),
            Message::DeleteIncompleteRecording(path) => {
                if let Err(e) = RecordingService::delete_incomplete_recording(&path) {
                    error!("Failed to delete incomplete recording: {}", e);
                }
                self.incomplete_recordings.retain(|p| p != &path);
                Command::none()
            }
            Message::RecoveryComplete(path) => {
                info!("Successfully recovered recording at {:?}", path);
                // Remove from incomplete list
                self.incomplete_recordings.retain(|p| !path.starts_with(p));
                // Offer to open the recovered recording
                let project_id = uuid::Uuid::new_v4().to_string();
                self.state = AppState::Previewing { project_id, path };
                Command::none()
            }
            Message::RecordingError(error) => {
                error!("Recording error: {}", error);
                // Open error dialog instead of just setting error state
                self.error_dialog
                    .open(frame_core::FrameError::Unknown(error));
                self.state = AppState::Idle;
                self.start_time = None;
                Command::none()
            }

            // Error Dialog
            Message::ErrorDialogMessage(dialog_msg) => {
                if let Some(recovery_action) = self.error_dialog.update(dialog_msg) {
                    match recovery_action {
                        frame_core::RecoveryAction::Retry => {
                            // Retry the last recording operation
                            return Command::perform(async {}, |_| Message::StartRecording);
                        }
                        frame_core::RecoveryAction::RequestPermissions => {
                            return Command::perform(async {}, |_| {
                                Message::RequestScreenPermission
                            });
                        }
                        frame_core::RecoveryAction::OpenSettings => {
                            // Open system settings (platform-specific)
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("open")
                                    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
                                    .spawn();
                            }
                        }
                        _ => {}
                    }
                }
                Command::none()
            }
            Message::ShowError(error_msg) => {
                self.error_dialog
                    .open(frame_core::FrameError::Unknown(error_msg));
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
                // Open export dialog instead of starting export directly
                if let AppState::Previewing { project_id, path } = &self.state {
                    self.export_dialog.open();
                    self.state = AppState::ExportConfiguring {
                        project_id: project_id.clone(),
                        path: path.clone(),
                    };
                }
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

            // Timeline
            Message::TimelinePositionChanged(position) => {
                if let Some(timeline) = &mut self.timeline {
                    timeline.set_position(position);
                }
                Command::none()
            }

            // Export Dialog
            Message::ExportDialogMessage(dialog_msg) => {
                match dialog_msg {
                    ExportDialogMessage::StartExport => {
                        self.export_dialog.close();
                        // Transition to exporting state with the configured settings
                        if let AppState::ExportConfiguring { project_id, .. } = &self.state {
                            self.state = AppState::Exporting {
                                project_id: project_id.clone(),
                                progress: 0.0,
                            };
                            // Start the actual export process here with config
                            info!(
                                "Starting export with config: {:?}",
                                self.export_dialog.config
                            );
                        }
                    }
                    ExportDialogMessage::CancelExport => {
                        self.export_dialog.close();
                        // Return to preview state
                        if let AppState::ExportConfiguring { project_id, path } = &self.state {
                            self.state = AppState::Previewing {
                                project_id: project_id.clone(),
                                path: path.clone(),
                            };
                        }
                    }
                    _ => {
                        self.export_dialog.update(dialog_msg);
                    }
                }
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
        let mut subscriptions = Vec::new();

        // Update recording stats every second while recording
        if matches!(self.state, AppState::Recording) {
            subscriptions
                .push(time::every(Duration::from_secs(1)).map(|_| Message::UpdateRecordingStats));

            // Auto-save tick every 10 seconds while recording
            subscriptions.push(
                time::every(Duration::from_secs(AUTO_SAVE_INTERVAL_SECS))
                    .map(|_| Message::AutoSaveTick),
            );
        }

        Subscription::batch(subscriptions)
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
