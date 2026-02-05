use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::effects::EffectsConfig;
use crate::FrameResult;

/// Current project format version
pub const PROJECT_FORMAT_VERSION: u32 = 1;

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
    // Version 1 is current, no migrations needed yet
    // Future migrations would be added here:
    // if from_version < 2 { project = migrate_v1_to_v2(project); }

    project.version = PROJECT_FORMAT_VERSION;
    let _ = from_version; // Silence unused warning until we have migrations
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
}
