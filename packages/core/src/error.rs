use thiserror::Error;

#[derive(Error, Debug)]
pub enum FrameError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Capture error: {0}")]
    CaptureError(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Project error: {0}")]
    ProjectError(String),

    #[error("Audio error: {0}")]
    AudioError(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("Feature not available in free tier: {0}")]
    ProFeature(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Recording already in progress")]
    RecordingInProgress,

    #[error("No recording in progress")]
    NoRecordingInProgress,

    #[error("Insufficient disk space: {0}")]
    InsufficientDiskSpace(String),
}

pub type FrameResult<T> = Result<T, FrameError>;
