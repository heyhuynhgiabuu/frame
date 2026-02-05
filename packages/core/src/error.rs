//! Comprehensive error handling for Frame
//!
//! This module provides detailed error types with recovery strategies,
//! user-friendly messages, and structured error reporting.

use thiserror::Error;

/// Main error type for Frame operations
#[derive(Error, Debug, Clone)]
pub enum FrameError {
    /// I/O errors (file system, network)
    #[error("I/O error: {0}")]
    Io(String),

    /// Serialization errors
    #[error("Failed to parse {0}: {1}")]
    Serialization(String, String),

    /// Screen capture errors
    #[error("Capture failed: {0}")]
    CaptureError(String),

    /// Video encoding errors
    #[error("Encoding failed: {0}")]
    EncodingError(String),

    /// Project management errors
    #[error("Project error: {0}")]
    ProjectError(String),

    /// Audio capture/processing errors
    #[error("Audio error: {0}")]
    AudioError(String),

    /// Platform-specific errors
    #[error("Platform error: {0}")]
    PlatformError(String),

    /// Permission errors
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Resource exhaustion errors
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// State errors
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// External tool errors (FFmpeg, etc.)
    #[error("{0} error: {1}")]
    ExternalTool(String, String),

    /// Network errors
    #[error("Network error: {0}")]
    Network(String),

    /// User cancellation
    #[error("Operation cancelled by user")]
    Cancelled,

    /// Timeout errors
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Recording already in progress
    #[error("Recording already in progress")]
    RecordingInProgress,

    /// No recording in progress
    #[error("No recording in progress")]
    NoRecordingInProgress,

    /// Platform not supported
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    /// Pro feature
    #[error("Feature not available in free tier: {0}")]
    ProFeature(String),

    /// Unknown/unexpected errors
    #[error("Unexpected error: {0}")]
    Unknown(String),
}

/// Stage of encoding where error occurred
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingStage {
    Setup,
    Initialization,
    Encoding,
    Finalization,
    Muxing,
}

/// Types of resources that can be exhausted
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    DiskSpace,
    Memory,
    Cpu,
    FileDescriptors,
    NetworkConnections,
    Battery,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceType::DiskSpace => write!(f, "disk space"),
            ResourceType::Memory => write!(f, "memory"),
            ResourceType::Cpu => write!(f, "CPU"),
            ResourceType::FileDescriptors => write!(f, "file descriptors"),
            ResourceType::NetworkConnections => write!(f, "network connections"),
            ResourceType::Battery => write!(f, "battery"),
        }
    }
}

/// Actions that require permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionAction {
    RecordScreen,
    RecordAudio,
    WriteFile,
    ReadFile,
    AccessCamera,
}

impl std::fmt::Display for PermissionAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionAction::RecordScreen => write!(f, "record screen"),
            PermissionAction::RecordAudio => write!(f, "record audio"),
            PermissionAction::WriteFile => write!(f, "write files"),
            PermissionAction::ReadFile => write!(f, "read files"),
            PermissionAction::AccessCamera => write!(f, "access camera"),
        }
    }
}

/// Result type alias
pub type FrameResult<T> = Result<T, FrameError>;

impl From<std::io::Error> for FrameError {
    fn from(error: std::io::Error) -> Self {
        FrameError::Io(format!("{} ({:?})", error, error.kind()))
    }
}

impl From<serde_json::Error> for FrameError {
    fn from(error: serde_json::Error) -> Self {
        FrameError::Serialization("JSON".to_string(), error.to_string())
    }
}

/// Error severity level for UI display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Informational, operation can continue
    Info,
    /// Warning, operation may be affected
    Warning,
    /// Error, operation failed but app can continue
    Error,
    /// Critical, app may need to restart
    Critical,
}

/// Error with context for user-friendly display
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub error: FrameError,
    pub severity: ErrorSeverity,
    pub action: Option<String>,
    pub recovery: Option<RecoveryAction>,
}

/// Possible recovery actions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Retry the operation
    Retry,
    /// Open settings/preferences
    OpenSettings,
    /// Request permissions again
    RequestPermissions,
    /// Free up disk space
    FreeDiskSpace,
    /// Save work and restart
    SaveAndRestart,
    /// Ignore and continue
    Ignore,
}

impl FrameError {
    /// Check if the error is recoverable (can retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            FrameError::CaptureError(_)
                | FrameError::Network(_)
                | FrameError::ResourceExhausted(_)
                | FrameError::Timeout(_)
                | FrameError::Cancelled
                | FrameError::ExternalTool(_, _)
        )
    }

    /// Get the severity level for this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            FrameError::Cancelled => ErrorSeverity::Info,
            FrameError::CaptureError(_) => ErrorSeverity::Warning,
            FrameError::AudioError(_) => ErrorSeverity::Warning,
            FrameError::PermissionDenied(_) => ErrorSeverity::Warning,
            FrameError::Network(_) => ErrorSeverity::Warning,
            FrameError::Timeout(_) => ErrorSeverity::Warning,
            FrameError::Io(_) => ErrorSeverity::Error,
            FrameError::EncodingError(_) => ErrorSeverity::Error,
            FrameError::InvalidState(_) => ErrorSeverity::Error,
            FrameError::ExternalTool(_, _) => ErrorSeverity::Error,
            FrameError::Unknown(_) => ErrorSeverity::Critical,
            _ => ErrorSeverity::Error,
        }
    }

    /// Get suggested recovery action
    pub fn recovery_action(&self) -> Option<RecoveryAction> {
        match self {
            FrameError::PermissionDenied(_) => Some(RecoveryAction::RequestPermissions),
            FrameError::ResourceExhausted(msg) if msg.contains("disk") => {
                Some(RecoveryAction::FreeDiskSpace)
            }
            FrameError::Network(_) => Some(RecoveryAction::Retry),
            FrameError::Timeout(_) => Some(RecoveryAction::Retry),
            FrameError::ExternalTool(_, _) => Some(RecoveryAction::Retry),
            FrameError::InvalidState(_) => Some(RecoveryAction::SaveAndRestart),
            _ => None,
        }
    }

    /// Create error context with all metadata
    pub fn into_context(self) -> ErrorContext {
        ErrorContext {
            severity: self.severity(),
            action: self.suggested_action_text(),
            recovery: self.recovery_action(),
            error: self,
        }
    }

    /// Get user-friendly action text
    fn suggested_action_text(&self) -> Option<String> {
        match self {
            FrameError::PermissionDenied(msg) => Some(format!("Grant permission to {}", msg)),
            FrameError::ResourceExhausted(msg) if msg.contains("disk") => {
                Some("Free up disk space and try again".to_string())
            }
            FrameError::Network(_) => Some("Check your connection and retry".to_string()),
            FrameError::EncodingError(_) => Some("Try with different export settings".to_string()),
            FrameError::CaptureError(_) => Some("Try recording again".to_string()),
            _ => None,
        }
    }

    /// Create an audio error with details
    pub fn audio(details: impl Into<String>) -> Self {
        FrameError::AudioError(details.into())
    }

    /// Create a capture error with details
    pub fn capture(details: impl Into<String>) -> Self {
        FrameError::CaptureError(details.into())
    }

    /// Create an encoding error with details
    pub fn encoding(details: impl Into<String>) -> Self {
        FrameError::EncodingError(details.into())
    }

    /// Create a permission denied error
    pub fn permission(resource: impl Into<String>) -> Self {
        FrameError::PermissionDenied(resource.into())
    }

    /// Create a project error
    pub fn project(details: impl Into<String>) -> Self {
        FrameError::ProjectError(details.into())
    }
}

/// Extension trait for converting standard errors
pub trait IntoFrameError<T> {
    /// Convert to FrameError with context
    fn into_frame_error(self, context: impl Into<String>) -> FrameResult<T>;
}

impl<T> IntoFrameError<T> for std::io::Result<T> {
    fn into_frame_error(self, context: impl Into<String>) -> FrameResult<T> {
        self.map_err(|e| FrameError::Io(format!("{}: {} ({:?})", context.into(), e, e.kind())))
    }
}

impl<T> IntoFrameError<T> for Result<T, serde_json::Error> {
    fn into_frame_error(self, context: impl Into<String>) -> FrameResult<T> {
        self.map_err(|e| FrameError::Serialization(context.into(), e.to_string()))
    }
}

/// Global error handler for logging and reporting
pub struct ErrorHandler;

impl ErrorHandler {
    /// Log an error with full context
    pub fn log_error(error: &FrameError, context: Option<&str>) {
        let msg = format!("{}", error);
        let recoverable = error.is_recoverable();

        match error.severity() {
            ErrorSeverity::Info => {
                if let Some(ctx) = context {
                    tracing::info!(error = %msg, context = ctx, recoverable, "Frame error occurred");
                } else {
                    tracing::info!(error = %msg, recoverable, "Frame error occurred");
                }
            }
            ErrorSeverity::Warning => {
                if let Some(ctx) = context {
                    tracing::warn!(error = %msg, context = ctx, recoverable, "Frame error occurred");
                } else {
                    tracing::warn!(error = %msg, recoverable, "Frame error occurred");
                }
            }
            ErrorSeverity::Error | ErrorSeverity::Critical => {
                if let Some(ctx) = context {
                    tracing::error!(error = %msg, context = ctx, recoverable, "Frame error occurred");
                } else {
                    tracing::error!(error = %msg, recoverable, "Frame error occurred");
                }
            }
        }
    }

    /// Handle errors that should be reported to the user
    pub fn handle_user_facing(error: FrameError) -> ErrorContext {
        let context = error.into_context();
        Self::log_error(&context.error, None);
        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity() {
        let cancelled = FrameError::Cancelled;
        assert_eq!(cancelled.severity(), ErrorSeverity::Info);

        let io_err = FrameError::Io("test".to_string());
        assert_eq!(io_err.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_error_recoverable() {
        let network = FrameError::Network("timeout".to_string());
        assert!(network.is_recoverable());

        let config = FrameError::Configuration("bad".to_string());
        assert!(!config.is_recoverable());
    }

    #[test]
    fn test_recovery_action() {
        let perm = FrameError::PermissionDenied("screen".to_string());
        assert_eq!(
            perm.recovery_action(),
            Some(RecoveryAction::RequestPermissions)
        );

        let disk = FrameError::ResourceExhausted("disk full".to_string());
        assert_eq!(disk.recovery_action(), Some(RecoveryAction::FreeDiskSpace));
    }

    #[test]
    fn test_error_conversion() {
        let io_result: std::io::Result<()> = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));

        let frame_result: FrameResult<()> = io_result.into_frame_error("reading config");
        assert!(frame_result.is_err());
    }
}
