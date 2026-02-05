use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Represents a Frame recording project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub settings: ProjectSettings,
    pub recordings: Vec<Recording>,
    pub exports: Vec<Export>,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            created_at: now,
            updated_at: now,
            settings: ProjectSettings::default(),
            recordings: Vec::new(),
            exports: Vec::new(),
        }
    }

    pub fn project_dir(&self) -> PathBuf {
        let data_dir = directories::ProjectDirs::from("app", "frame", "Frame")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir().join("frame"));

        data_dir.join("projects").join(&self.id)
    }

    pub fn save(&self) -> crate::FrameResult<()> {
        let project_file = self.project_dir().join("project.json");
        std::fs::create_dir_all(project_file.parent().unwrap())?;
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(project_file, json)?;
        Ok(())
    }

    pub fn load(project_id: &str) -> crate::FrameResult<Self> {
        let data_dir = directories::ProjectDirs::from("app", "frame", "Frame")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir().join("frame"));

        let project_file = data_dir
            .join("projects")
            .join(project_id)
            .join("project.json");
        let json = std::fs::read_to_string(project_file)?;
        let project: Project = serde_json::from_str(&json)?;
        Ok(project)
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
