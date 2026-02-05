pub mod auto_save;
pub mod capture;
pub mod effects;
pub mod encoder;
pub mod error;
pub mod project;

pub use auto_save::{AutoSaveConfig, AutoSaveService, RecoveryService};
pub use error::{
    EncodingStage, ErrorContext, ErrorHandler, ErrorSeverity, FrameError, FrameResult,
    IntoFrameError, PermissionAction, RecoveryAction, ResourceType,
};
pub use project::{Export, ExportFormat, Project, ProjectSettings, Recording, Resolution, RecordingState};
