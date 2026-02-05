use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

use crate::effects::EffectsConfig;
use crate::FrameResult;

/// Current project format version (bumped for edit operations support)
pub const PROJECT_FORMAT_VERSION: u32 = 2;

/// Minimum supported version for migration
pub const MIN_SUPPORTED_VERSION: u32 = 1;

/// Magic bytes for .frame file identification
pub const FRAME_MAGIC: &[u8; 5] = b"FRAME";

/// Represents a Frame recording project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Format version for forward/backward compatibility
    #[serde(default = "default_version")]
    pub version: u32,
    pub id: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub settings: ProjectSettings,
    /// Effects configuration (zoom, keyboard, background)
    #[serde(default)]
    pub effects: EffectsConfig,
    pub recordings: Vec<Recording>,
    pub exports: Vec<Export>,
    /// Current recording state for auto-save tracking
    #[serde(default)]
    pub recording_state: RecordingState,
    /// Edit history for non-destructive timeline editing
    #[serde(default)]
    pub edit_history: EditHistory,
}

fn default_version() -> u32 {
    PROJECT_FORMAT_VERSION
}

/// Recording state for auto-save tracking
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum RecordingState {
    #[default]
    Idle,
    Recording,
    Paused,
    Completed,
    Error,
}

/// Represents a non-destructive edit operation on the timeline
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EditOperation {
    /// Remove content from the beginning and/or end of a recording
    /// - start: new start time (content before this is trimmed)
    /// - end: new end time (content after this is trimmed)
    Trim {
        #[serde(with = "duration_millis")]
        start: Duration,
        #[serde(with = "duration_millis")]
        end: Duration,
    },
    /// Remove a section from the middle of a recording
    /// Content after `to` is shifted earlier by (to - from)
    Cut {
        #[serde(with = "duration_millis")]
        from: Duration,
        #[serde(with = "duration_millis")]
        to: Duration,
    },
    /// Divide a clip into multiple segments at a point
    Split {
        #[serde(with = "duration_millis")]
        at: Duration,
    },
}

/// Serialization module for Duration as milliseconds (u64)
mod duration_millis {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

/// Maximum number of operations in undo history to prevent unbounded memory growth
pub const MAX_UNDO_HISTORY: usize = 50;

/// Edit history with undo/redo support
///
/// Operations are stored in order of application. `current_index` points to
/// the position after the last applied operation. Undo decrements the index,
/// redo increments it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditHistory {
    /// All operations (applied and undone)
    operations: Vec<EditOperation>,
    /// Index pointing after the last applied operation
    /// - 0 means no operations applied
    /// - operations.len() means all operations applied
    current_index: usize,
}

impl EditHistory {
    /// Create a new empty edit history
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new operation, clearing any undone operations and enforcing max history
    pub fn push(&mut self, operation: EditOperation) {
        // Clear any undone operations (redo history)
        self.operations.truncate(self.current_index);

        // Add the new operation
        self.operations.push(operation);
        self.current_index = self.operations.len();

        // Enforce max history limit (remove oldest operations)
        if self.operations.len() > MAX_UNDO_HISTORY {
            let excess = self.operations.len() - MAX_UNDO_HISTORY;
            self.operations.drain(..excess);
            self.current_index = self.operations.len();
        }
    }

    /// Undo the last operation, returning it if successful
    pub fn undo(&mut self) -> Option<&EditOperation> {
        if self.current_index > 0 {
            self.current_index -= 1;
            Some(&self.operations[self.current_index])
        } else {
            None
        }
    }

    /// Redo the last undone operation, returning it if successful
    pub fn redo(&mut self) -> Option<&EditOperation> {
        if self.current_index < self.operations.len() {
            let op = &self.operations[self.current_index];
            self.current_index += 1;
            Some(op)
        } else {
            None
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.current_index > 0
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        self.current_index < self.operations.len()
    }

    /// Get all currently applied operations (excludes undone operations)
    pub fn applied_operations(&self) -> &[EditOperation] {
        &self.operations[..self.current_index]
    }

    /// Get the number of applied operations
    pub fn len(&self) -> usize {
        self.current_index
    }

    /// Check if history is empty (no applied operations)
    pub fn is_empty(&self) -> bool {
        self.current_index == 0
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.operations.clear();
        self.current_index = 0;
    }

    /// Calculate the effective duration after applying all edit operations
    ///
    /// This is used for timeline display and export to determine the final length.
    pub fn effective_duration(&self, original_duration: Duration) -> Duration {
        let mut duration = original_duration;

        for op in self.applied_operations() {
            match op {
                EditOperation::Trim { start, end } => {
                    // Trim reduces to the range [start, end]
                    let end_clamped = (*end).min(duration);
                    let start_clamped = (*start).min(end_clamped);
                    duration = end_clamped.saturating_sub(start_clamped);
                }
                EditOperation::Cut { from, to } => {
                    // Cut removes [from, to], shifting content after `to` earlier
                    let cut_duration = to.saturating_sub(*from);
                    duration = duration.saturating_sub(cut_duration);
                }
                EditOperation::Split { .. } => {
                    // Split doesn't change duration, just creates segment boundaries
                }
            }
        }

        duration
    }

    /// Validate that a trim operation won't result in empty content
    ///
    /// Returns Ok(()) if valid, Err with reason if invalid.
    pub fn validate_trim(
        &self,
        original_duration: Duration,
        start: Duration,
        end: Duration,
    ) -> Result<(), &'static str> {
        if start >= end {
            return Err("Trim start must be before end");
        }

        if start >= original_duration {
            return Err("Trim start is beyond the recording duration");
        }

        // Calculate effective duration with this trim applied
        let trim_duration = end.min(original_duration).saturating_sub(start);

        // Minimum duration: 500ms
        const MIN_DURATION: Duration = Duration::from_millis(500);
        if trim_duration < MIN_DURATION {
            return Err("Trim would result in a video shorter than 0.5 seconds");
        }

        Ok(())
    }

    /// Validate that a cut operation won't result in empty content
    pub fn validate_cut(
        &self,
        original_duration: Duration,
        from: Duration,
        to: Duration,
    ) -> Result<(), &'static str> {
        if from >= to {
            return Err("Cut start must be before end");
        }

        if from >= original_duration {
            return Err("Cut start is beyond the recording duration");
        }

        // Calculate remaining duration after this cut
        let cut_duration = to.saturating_sub(from);
        let current_effective = self.effective_duration(original_duration);
        let remaining = current_effective.saturating_sub(cut_duration);

        // Minimum duration: 500ms
        const MIN_DURATION: Duration = Duration::from_millis(500);
        if remaining < MIN_DURATION {
            return Err("Cut would result in a video shorter than 0.5 seconds");
        }

        Ok(())
    }

    /// Push a trim operation with validation
    ///
    /// Returns Ok(()) if applied, Err with reason if validation failed.
    pub fn push_trim(
        &mut self,
        original_duration: Duration,
        start: Duration,
        end: Duration,
    ) -> Result<(), &'static str> {
        self.validate_trim(original_duration, start, end)?;
        self.push(EditOperation::Trim { start, end });
        Ok(())
    }

    /// Push a cut operation with validation
    ///
    /// Returns Ok(()) if applied, Err with reason if validation failed.
    pub fn push_cut(
        &mut self,
        original_duration: Duration,
        from: Duration,
        to: Duration,
    ) -> Result<(), &'static str> {
        self.validate_cut(original_duration, from, to)?;
        self.push(EditOperation::Cut { from, to });
        Ok(())
    }
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            version: PROJECT_FORMAT_VERSION,
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            created_at: now,
            updated_at: now,
            settings: ProjectSettings::default(),
            effects: EffectsConfig::default(),
            recordings: Vec::new(),
            exports: Vec::new(),
            recording_state: RecordingState::Idle,
            edit_history: EditHistory::default(),
        }
    }

    pub fn project_dir(&self) -> PathBuf {
        let data_dir = directories::ProjectDirs::from("app", "frame", "Frame")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir().join("frame"));

        data_dir.join("projects").join(&self.id)
    }

    /// Save project as .frame file (binary format with JSON payload)
    pub fn save(&self) -> FrameResult<()> {
        let project_file = self.project_dir().join("project.frame");
        std::fs::create_dir_all(project_file.parent().unwrap())?;
        self.save_to_file(&project_file)
    }

    /// Save project to a specific path
    pub fn save_to_file(&self, path: &Path) -> FrameResult<()> {
        let json = serde_json::to_string_pretty(self)?;
        let mut data = Vec::with_capacity(FRAME_MAGIC.len() + 4 + json.len());

        // Magic bytes
        data.extend_from_slice(FRAME_MAGIC);
        // Version (u32 little endian)
        data.extend_from_slice(&self.version.to_le_bytes());
        // JSON payload
        data.extend_from_slice(json.as_bytes());

        std::fs::write(path, data)?;
        Ok(())
    }

    /// Load project by ID from default location
    pub fn load(project_id: &str) -> FrameResult<Self> {
        let data_dir = directories::ProjectDirs::from("app", "frame", "Frame")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir().join("frame"));

        // Try new .frame format first, then fall back to legacy .json
        let frame_file = data_dir
            .join("projects")
            .join(project_id)
            .join("project.frame");

        if frame_file.exists() {
            return Self::load_from_file(&frame_file);
        }

        // Legacy JSON format
        let json_file = data_dir
            .join("projects")
            .join(project_id)
            .join("project.json");
        let json = std::fs::read_to_string(json_file)?;
        let mut project: Project = serde_json::from_str(&json)?;
        project.version = PROJECT_FORMAT_VERSION;
        Ok(project)
    }

    /// Load project from a .frame file
    pub fn load_from_file(path: &Path) -> FrameResult<Self> {
        let data = std::fs::read(path)?;

        // Check minimum size for header
        if data.len() < FRAME_MAGIC.len() + 4 {
            return Err(crate::FrameError::project(
                "File too small to be a valid .frame file",
            ));
        }

        // Verify magic bytes
        if &data[..FRAME_MAGIC.len()] != FRAME_MAGIC {
            return Err(crate::FrameError::project(
                "Invalid .frame file magic bytes",
            ));
        }

        // Read version
        let version_bytes: [u8; 4] = data[FRAME_MAGIC.len()..FRAME_MAGIC.len() + 4]
            .try_into()
            .map_err(|_| crate::FrameError::project("Failed to read version"))?;
        let file_version = u32::from_le_bytes(version_bytes);

        // Check version compatibility
        if file_version > PROJECT_FORMAT_VERSION {
            return Err(crate::FrameError::project(format!(
                "Project file version {} is newer than supported version {}. Please update Frame.",
                file_version, PROJECT_FORMAT_VERSION
            )));
        }

        if file_version < MIN_SUPPORTED_VERSION {
            return Err(crate::FrameError::project(format!(
                "Project file version {} is too old. Minimum supported: {}",
                file_version, MIN_SUPPORTED_VERSION
            )));
        }

        // Parse JSON payload
        let json_start = FRAME_MAGIC.len() + 4;
        let json = std::str::from_utf8(&data[json_start..]).map_err(|e| {
            crate::FrameError::project(format!("Invalid UTF-8 in project file: {}", e))
        })?;

        let mut project: Project = serde_json::from_str(json)?;

        // Run migrations if needed
        project = migrate_project(project, file_version)?;

        Ok(project)
    }

    /// Check if project has unsaved changes (simple flag-based tracking)
    pub fn mark_modified(&mut self) {
        self.updated_at = chrono::Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub resolution: Resolution,
    pub frame_rate: u32,
    pub video_codec: VideoCodec,
    pub audio_codec: AudioCodec,
    pub quality: Quality,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            resolution: Resolution::Hd1080,
            frame_rate: 60,
            video_codec: VideoCodec::H264,
            audio_codec: AudioCodec::Aac,
            quality: Quality::High,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Resolution {
    Hd720,
    Hd1080,
    QuadHd,
    Uhd4k,
}

impl Resolution {
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Resolution::Hd720 => (1280, 720),
            Resolution::Hd1080 => (1920, 1080),
            Resolution::QuadHd => (2560, 1440),
            Resolution::Uhd4k => (3840, 2160),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
    ProRes,
    #[cfg(feature = "pro")]
    Av1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AudioCodec {
    Aac,
    Opus,
    #[cfg(feature = "pro")]
    Flac,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Quality {
    Low,
    Medium,
    High,
    #[cfg(feature = "pro")]
    Lossless,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub id: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: u64,
    pub file_path: PathBuf,
    pub has_video: bool,
    pub has_audio: bool,
    pub resolution: Resolution,
    pub frame_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Export {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub file_path: PathBuf,
    pub format: ExportFormat,
    pub resolution: Resolution,
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExportFormat {
    Mp4,
    Mov,
    Gif,
    #[cfg(feature = "pro")]
    WebM,
}

/// Migrate project from older format versions to current
fn migrate_project(mut project: Project, from_version: u32) -> FrameResult<Project> {
    // v1 → v2: Add edit_history field (handled by serde default)
    // No explicit migration needed since EditHistory::default() is used

    if from_version < 2 {
        // Ensure edit_history is properly initialized for v1 projects
        // serde default handles this, but we can explicitly set for clarity
        if project.edit_history.operations.is_empty() {
            project.edit_history = EditHistory::default();
        }
    }

    project.version = PROJECT_FORMAT_VERSION;
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_project_new() {
        let project = Project::new("Test Project");
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.version, PROJECT_FORMAT_VERSION);
        assert!(project.recordings.is_empty());
        assert!(project.edit_history.is_empty());
    }

    #[test]
    fn test_project_roundtrip() {
        let file = NamedTempFile::new().unwrap();
        let project = Project::new("Roundtrip Test");

        project.save_to_file(file.path()).unwrap();
        let loaded = Project::load_from_file(file.path()).unwrap();

        assert_eq!(loaded.name, project.name);
        assert_eq!(loaded.id, project.id);
        assert_eq!(loaded.version, PROJECT_FORMAT_VERSION);
    }

    #[test]
    fn test_invalid_magic() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"NOT_FRAME_FILE_DATA").unwrap();

        let result = Project::load_from_file(file.path());
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("magic bytes"));
        }
    }

    #[test]
    fn test_version_too_new() {
        let mut file = NamedTempFile::new().unwrap();

        // Write header with version 999
        file.write_all(FRAME_MAGIC).unwrap();
        file.write_all(&999u32.to_le_bytes()).unwrap();
        file.write_all(b"{}").unwrap();

        let result = Project::load_from_file(file.path());
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("newer than supported"));
        }
    }

    #[test]
    fn test_effects_config_default() {
        let project = Project::new("Effects Test");
        assert!(project.effects.zoom.enabled);
        assert!(project.effects.keyboard.enabled);
    }

    // Edit Operations Tests

    #[test]
    fn test_edit_operation_trim_serialization() {
        let trim = EditOperation::Trim {
            start: Duration::from_millis(5000),
            end: Duration::from_millis(30000),
        };

        let json = serde_json::to_string(&trim).unwrap();
        let loaded: EditOperation = serde_json::from_str(&json).unwrap();

        assert_eq!(trim, loaded);
    }

    #[test]
    fn test_edit_operation_cut_serialization() {
        let cut = EditOperation::Cut {
            from: Duration::from_millis(10000),
            to: Duration::from_millis(20000),
        };

        let json = serde_json::to_string(&cut).unwrap();
        let loaded: EditOperation = serde_json::from_str(&json).unwrap();

        assert_eq!(cut, loaded);
    }

    #[test]
    fn test_edit_operation_split_serialization() {
        let split = EditOperation::Split {
            at: Duration::from_millis(15000),
        };

        let json = serde_json::to_string(&split).unwrap();
        let loaded: EditOperation = serde_json::from_str(&json).unwrap();

        assert_eq!(split, loaded);
    }

    // EditHistory Tests

    #[test]
    fn test_edit_history_push_and_undo_redo() {
        let mut history = EditHistory::new();

        // Initially empty
        assert!(history.is_empty());
        assert!(!history.can_undo());
        assert!(!history.can_redo());

        // Push operations
        let op1 = EditOperation::Trim {
            start: Duration::from_secs(0),
            end: Duration::from_secs(30),
        };
        let op2 = EditOperation::Cut {
            from: Duration::from_secs(10),
            to: Duration::from_secs(15),
        };
        let op3 = EditOperation::Split {
            at: Duration::from_secs(20),
        };

        history.push(op1.clone());
        assert_eq!(history.len(), 1);
        assert!(history.can_undo());
        assert!(!history.can_redo());

        history.push(op2.clone());
        history.push(op3.clone());
        assert_eq!(history.len(), 3);

        // Undo all three
        let undone = history.undo().cloned();
        assert_eq!(undone, Some(op3.clone()));
        assert_eq!(history.len(), 2);
        assert!(history.can_redo());

        let undone = history.undo().cloned();
        assert_eq!(undone, Some(op2.clone()));
        assert_eq!(history.len(), 1);

        let undone = history.undo().cloned();
        assert_eq!(undone, Some(op1.clone()));
        assert_eq!(history.len(), 0);

        // Can't undo further
        assert!(!history.can_undo());
        assert!(history.undo().is_none());

        // Redo two
        let redone = history.redo().cloned();
        assert_eq!(redone, Some(op1.clone()));
        assert_eq!(history.len(), 1);

        let redone = history.redo().cloned();
        assert_eq!(redone, Some(op2.clone()));
        assert_eq!(history.len(), 2);

        // State matches after undo/redo cycle
        assert_eq!(history.applied_operations(), &[op1.clone(), op2.clone()]);
    }

    #[test]
    fn test_edit_history_push_clears_redo() {
        let mut history = EditHistory::new();

        let op1 = EditOperation::Split {
            at: Duration::from_secs(10),
        };
        let op2 = EditOperation::Split {
            at: Duration::from_secs(20),
        };
        let op3 = EditOperation::Split {
            at: Duration::from_secs(5),
        };

        history.push(op1.clone());
        history.push(op2.clone());
        history.undo(); // Undo op2

        // Push new operation - should clear redo history
        history.push(op3.clone());

        assert_eq!(history.len(), 2);
        assert!(!history.can_redo());
        assert_eq!(history.applied_operations(), &[op1, op3]);
    }

    #[test]
    fn test_edit_history_max_limit() {
        let mut history = EditHistory::new();

        // Push more than MAX_UNDO_HISTORY operations
        for i in 0..(MAX_UNDO_HISTORY + 10) {
            history.push(EditOperation::Split {
                at: Duration::from_millis(i as u64 * 1000),
            });
        }

        // Should be capped at MAX_UNDO_HISTORY
        assert_eq!(history.len(), MAX_UNDO_HISTORY);
        assert_eq!(history.operations.len(), MAX_UNDO_HISTORY);
    }

    #[test]
    fn test_effective_duration_trim() {
        let mut history = EditHistory::new();
        let original = Duration::from_secs(60);

        // Trim to 10-40 seconds = 30s duration
        history.push(EditOperation::Trim {
            start: Duration::from_secs(10),
            end: Duration::from_secs(40),
        });

        assert_eq!(
            history.effective_duration(original),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn test_effective_duration_cut() {
        let mut history = EditHistory::new();
        let original = Duration::from_secs(60);

        // Cut 10-20 seconds = removes 10s
        history.push(EditOperation::Cut {
            from: Duration::from_secs(10),
            to: Duration::from_secs(20),
        });

        assert_eq!(
            history.effective_duration(original),
            Duration::from_secs(50)
        );
    }

    #[test]
    fn test_effective_duration_split() {
        let mut history = EditHistory::new();
        let original = Duration::from_secs(60);

        // Split doesn't change duration
        history.push(EditOperation::Split {
            at: Duration::from_secs(30),
        });

        assert_eq!(
            history.effective_duration(original),
            Duration::from_secs(60)
        );
    }

    #[test]
    fn test_effective_duration_combined() {
        let mut history = EditHistory::new();
        let original = Duration::from_secs(60);

        // Trim to 5-50 seconds = 45s
        history.push(EditOperation::Trim {
            start: Duration::from_secs(5),
            end: Duration::from_secs(50),
        });

        // Cut 10-20 from the trimmed result = removes 10s, now 35s
        history.push(EditOperation::Cut {
            from: Duration::from_secs(10),
            to: Duration::from_secs(20),
        });

        // Split doesn't change duration
        history.push(EditOperation::Split {
            at: Duration::from_secs(15),
        });

        assert_eq!(
            history.effective_duration(original),
            Duration::from_secs(35)
        );
    }

    #[test]
    fn test_project_with_edit_history_roundtrip() {
        let file = NamedTempFile::new().unwrap();
        let mut project = Project::new("Edit History Test");

        // Add some edit operations
        project.edit_history.push(EditOperation::Trim {
            start: Duration::from_secs(5),
            end: Duration::from_secs(55),
        });
        project.edit_history.push(EditOperation::Cut {
            from: Duration::from_secs(20),
            to: Duration::from_secs(30),
        });
        project.edit_history.push(EditOperation::Split {
            at: Duration::from_secs(25),
        });

        // Save and reload
        project.save_to_file(file.path()).unwrap();
        let loaded = Project::load_from_file(file.path()).unwrap();

        // Verify edit history persisted
        assert_eq!(loaded.edit_history.len(), 3);
        assert_eq!(
            loaded.edit_history.applied_operations(),
            project.edit_history.applied_operations()
        );
    }
}
