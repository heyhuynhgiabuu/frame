//! Export preset management for Frame
//!
//! This module provides preset configurations for video export,
//! including codec selection, quality presets, and resolution settings.
//! Presets are stored as JSON in the user's config directory.

use crate::error::{FrameError, FrameResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Video codec options for export
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VideoCodec {
    /// H.264/AVC - Widely compatible, good compression
    #[default]
    H264,
    /// H.265/HEVC - Better compression than H.264
    H265,
    /// VP9 - Open format, good for web
    Vp9,
    /// Apple ProRes - Professional editing codec
    ProRes,
}

impl std::fmt::Display for VideoCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VideoCodec::H264 => write!(f, "H.264"),
            VideoCodec::H265 => write!(f, "H.265"),
            VideoCodec::Vp9 => write!(f, "VP9"),
            VideoCodec::ProRes => write!(f, "ProRes"),
        }
    }
}

/// Output format for exported videos
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportOutputFormat {
    /// MP4 container
    #[default]
    Mp4,
    /// MOV container (QuickTime)
    Mov,
    /// WebM container
    Webm,
}

impl std::fmt::Display for ExportOutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportOutputFormat::Mp4 => write!(f, "MP4"),
            ExportOutputFormat::Mov => write!(f, "MOV"),
            ExportOutputFormat::Webm => write!(f, "WebM"),
        }
    }
}

/// Quality preset for quick configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QualityPreset {
    /// Low quality, small file size
    Low,
    /// Medium quality, balanced
    #[default]
    Medium,
    /// High quality, larger file size
    High,
    /// Lossless quality, maximum file size
    Lossless,
    /// Custom quality settings
    Custom,
}

impl std::fmt::Display for QualityPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualityPreset::Low => write!(f, "Low"),
            QualityPreset::Medium => write!(f, "Medium"),
            QualityPreset::High => write!(f, "High"),
            QualityPreset::Lossless => write!(f, "Lossless"),
            QualityPreset::Custom => write!(f, "Custom"),
        }
    }
}

impl QualityPreset {
    /// Get recommended bitrate for this quality preset (in kbps)
    /// Returns bitrate based on 1080p resolution
    pub fn recommended_bitrate(&self) -> u32 {
        match self {
            QualityPreset::Low => 2500,
            QualityPreset::Medium => 5000,
            QualityPreset::High => 10000,
            QualityPreset::Lossless => 0, // Uses CRF 0 or equivalent
            QualityPreset::Custom => 5000,
        }
    }

    /// Get recommended CRF (Constant Rate Factor) value
    /// Lower is better quality, 0 is lossless
    pub fn recommended_crf(&self) -> u8 {
        match self {
            QualityPreset::Low => 28,
            QualityPreset::Medium => 23,
            QualityPreset::High => 18,
            QualityPreset::Lossless => 0,
            QualityPreset::Custom => 23,
        }
    }
}

/// Complete export preset configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportPreset {
    /// Unique identifier for the preset
    pub id: String,
    /// Display name for the preset
    pub name: String,
    /// Video codec to use
    pub codec: VideoCodec,
    /// Output container format
    pub format: ExportOutputFormat,
    /// Output width in pixels
    pub width: u32,
    /// Output height in pixels
    pub height: u32,
    /// Target bitrate in kbps (0 for lossless/CRF based)
    pub bitrate: u32,
    /// Target frame rate
    pub fps: u32,
    /// Quality preset selection
    pub quality: QualityPreset,
    /// Custom CRF value (if quality is Custom)
    pub custom_crf: Option<u8>,
    /// Whether this is a built-in preset (non-deletable)
    #[serde(default)]
    pub is_built_in: bool,
}

impl Default for ExportPreset {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Default".to_string(),
            codec: VideoCodec::default(),
            format: ExportOutputFormat::default(),
            width: 1920,
            height: 1080,
            bitrate: 5000,
            fps: 30,
            quality: QualityPreset::Medium,
            custom_crf: None,
            is_built_in: false,
        }
    }
}

impl ExportPreset {
    /// Create a new preset with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Create a preset for a specific resolution
    pub fn for_resolution(name: impl Into<String>, width: u32, height: u32) -> Self {
        let mut preset = Self::new(name);
        preset.width = width;
        preset.height = height;
        // Adjust bitrate based on resolution
        let pixel_count = width * height;
        let base_pixels = 1920 * 1080;
        preset.bitrate = ((5000.0 * pixel_count as f64) / base_pixels as f64) as u32;
        preset
    }

    /// Apply a quality preset to update bitrate and CRF
    pub fn with_quality(mut self, quality: QualityPreset) -> Self {
        self.quality = quality;
        self.bitrate = quality.recommended_bitrate();
        if matches!(quality, QualityPreset::Custom) {
            self.custom_crf = Some(quality.recommended_crf());
        }
        self
    }

    /// Set the codec and automatically adjust format if needed
    pub fn with_codec(mut self, codec: VideoCodec) -> Self {
        self.codec = codec;
        // Adjust format for codec compatibility
        match codec {
            VideoCodec::Vp9 => {
                if !matches!(self.format, ExportOutputFormat::Webm) {
                    self.format = ExportOutputFormat::Webm;
                }
            }
            VideoCodec::ProRes => {
                if !matches!(self.format, ExportOutputFormat::Mov) {
                    self.format = ExportOutputFormat::Mov;
                }
            }
            _ => {}
        }
        self
    }

    /// Get the effective CRF value
    pub fn effective_crf(&self) -> u8 {
        match self.quality {
            QualityPreset::Custom => self.custom_crf.unwrap_or(23),
            _ => self.quality.recommended_crf(),
        }
    }

    /// Check if this preset uses constant rate factor instead of bitrate
    pub fn uses_crf(&self) -> bool {
        self.bitrate == 0 || matches!(self.quality, QualityPreset::Lossless)
    }

    /// Validate the preset configuration
    pub fn validate(&self) -> FrameResult<()> {
        if self.name.is_empty() {
            return Err(FrameError::Configuration(
                "Preset name cannot be empty".to_string(),
            ));
        }
        if self.width == 0 || self.height == 0 {
            return Err(FrameError::Configuration(
                "Resolution must be greater than 0".to_string(),
            ));
        }
        if self.fps == 0 {
            return Err(FrameError::Configuration(
                "Frame rate must be greater than 0".to_string(),
            ));
        }
        if self.quality == QualityPreset::Custom && self.custom_crf.is_none() {
            return Err(FrameError::Configuration(
                "Custom quality requires a CRF value".to_string(),
            ));
        }
        Ok(())
    }

    /// Mark this preset as built-in
    pub fn built_in(mut self) -> Self {
        self.is_built_in = true;
        self
    }
}

/// Built-in preset types for common export scenarios
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltInPreset {
    /// Web optimized: H.264, 1080p, 8Mbps, MP4, 30fps
    Web,
    /// YouTube optimized: H.264, 4K, 20Mbps, MP4, 30fps
    YouTube,
    /// Twitter/X optimized: H.264, 720p, 5Mbps, MP4, 30fps (max 2:20)
    Twitter,
    /// Instagram optimized: H.264, 1080x1080 square, 8Mbps, MP4, 30fps
    Instagram,
    /// GIF Preview placeholder: 640x480, 10fps
    GifPreview,
    /// Discord optimized: H.264, 720p, 6Mbps, MP4, 30fps (8MB limit note)
    Discord,
    /// Lossless quality: ProRes, original resolution, MOV
    Lossless,
}

impl BuiltInPreset {
    /// Get the unique ID for this built-in preset
    pub fn id(&self) -> &'static str {
        match self {
            BuiltInPreset::Web => "builtin-web",
            BuiltInPreset::YouTube => "builtin-youtube",
            BuiltInPreset::Twitter => "builtin-twitter",
            BuiltInPreset::Instagram => "builtin-instagram",
            BuiltInPreset::GifPreview => "builtin-gif-preview",
            BuiltInPreset::Discord => "builtin-discord",
            BuiltInPreset::Lossless => "builtin-lossless",
        }
    }

    /// Get the display name for this built-in preset
    pub fn name(&self) -> &'static str {
        match self {
            BuiltInPreset::Web => "Web (1080p)",
            BuiltInPreset::YouTube => "YouTube (4K)",
            BuiltInPreset::Twitter => "Twitter/X (720p)",
            BuiltInPreset::Instagram => "Instagram (Square)",
            BuiltInPreset::GifPreview => "GIF Preview",
            BuiltInPreset::Discord => "Discord (720p)",
            BuiltInPreset::Lossless => "Lossless (ProRes)",
        }
    }

    /// Create an ExportPreset from this built-in type
    pub fn to_preset(&self) -> ExportPreset {
        let mut preset = match self {
            BuiltInPreset::Web => ExportPreset {
                id: self.id().to_string(),
                name: self.name().to_string(),
                codec: VideoCodec::H264,
                format: ExportOutputFormat::Mp4,
                width: 1920,
                height: 1080,
                bitrate: 8000, // 8 Mbps
                fps: 30,
                quality: QualityPreset::High,
                custom_crf: None,
                is_built_in: true,
            },
            BuiltInPreset::YouTube => ExportPreset {
                id: self.id().to_string(),
                name: self.name().to_string(),
                codec: VideoCodec::H264,
                format: ExportOutputFormat::Mp4,
                width: 3840,
                height: 2160,
                bitrate: 20000, // 20 Mbps
                fps: 30,
                quality: QualityPreset::High,
                custom_crf: None,
                is_built_in: true,
            },
            BuiltInPreset::Twitter => ExportPreset {
                id: self.id().to_string(),
                name: self.name().to_string(),
                codec: VideoCodec::H264,
                format: ExportOutputFormat::Mp4,
                width: 1280,
                height: 720,
                bitrate: 5000, // 5 Mbps
                fps: 30,
                quality: QualityPreset::Medium,
                custom_crf: None,
                is_built_in: true,
            },
            BuiltInPreset::Instagram => ExportPreset {
                id: self.id().to_string(),
                name: self.name().to_string(),
                codec: VideoCodec::H264,
                format: ExportOutputFormat::Mp4,
                width: 1080,
                height: 1080,  // Square format
                bitrate: 8000, // 8 Mbps
                fps: 30,
                quality: QualityPreset::High,
                custom_crf: None,
                is_built_in: true,
            },
            BuiltInPreset::GifPreview => ExportPreset {
                id: self.id().to_string(),
                name: self.name().to_string(),
                codec: VideoCodec::H264, // Placeholder, actual GIF uses different encoder
                format: ExportOutputFormat::Mp4,
                width: 640,
                height: 480,
                bitrate: 2000, // 2 Mbps for small preview
                fps: 10,       // Low fps for GIF-like feel
                quality: QualityPreset::Low,
                custom_crf: None,
                is_built_in: true,
            },
            BuiltInPreset::Discord => ExportPreset {
                id: self.id().to_string(),
                name: self.name().to_string(),
                codec: VideoCodec::H264,
                format: ExportOutputFormat::Mp4,
                width: 1280,
                height: 720,
                bitrate: 6000, // 6 Mbps - balanced for 8MB limit
                fps: 30,
                quality: QualityPreset::Medium,
                custom_crf: None,
                is_built_in: true,
            },
            BuiltInPreset::Lossless => ExportPreset {
                id: self.id().to_string(),
                name: self.name().to_string(),
                codec: VideoCodec::ProRes,
                format: ExportOutputFormat::Mov,
                width: 1920, // Uses source resolution
                height: 1080,
                bitrate: 0, // CRF-based
                fps: 30,
                quality: QualityPreset::Lossless,
                custom_crf: None,
                is_built_in: true,
            },
        };
        preset.is_built_in = true;
        preset
    }

    /// Get all built-in preset variants
    pub fn all() -> Vec<BuiltInPreset> {
        vec![
            BuiltInPreset::Web,
            BuiltInPreset::YouTube,
            BuiltInPreset::Twitter,
            BuiltInPreset::Instagram,
            BuiltInPreset::GifPreview,
            BuiltInPreset::Discord,
            BuiltInPreset::Lossless,
        ]
    }
}

/// Manager for loading, saving, and organizing export presets
#[derive(Debug, Clone)]
pub struct PresetManager {
    presets_dir: PathBuf,
    presets: HashMap<String, ExportPreset>,
}

impl Default for PresetManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PresetManager {
    /// Create a new preset manager with default config directory
    pub fn new() -> Self {
        let presets_dir = Self::default_presets_dir();
        Self {
            presets_dir,
            presets: HashMap::new(),
        }
    }

    /// Create a preset manager with a custom directory
    pub fn with_directory(path: impl Into<PathBuf>) -> Self {
        Self {
            presets_dir: path.into(),
            presets: HashMap::new(),
        }
    }

    /// Get the default presets directory
    fn default_presets_dir() -> PathBuf {
        directories::ProjectDirs::from("com", "frame", "Frame")
            .map(|dirs| dirs.config_dir().join("presets"))
            .unwrap_or_else(|| PathBuf::from("./presets"))
    }

    /// Ensure the presets directory exists
    async fn ensure_directory(&self) -> FrameResult<()> {
        if !self.presets_dir.exists() {
            tokio::fs::create_dir_all(&self.presets_dir)
                .await
                .map_err(|e| {
                    FrameError::Io(format!(
                        "Failed to create presets directory: {} ({:?})",
                        e,
                        e.kind()
                    ))
                })?;
        }
        Ok(())
    }

    /// Load all presets from disk, including built-in presets
    pub async fn load_presets(&mut self) -> FrameResult<()> {
        self.ensure_directory().await?;
        self.presets.clear();

        // First, add all built-in presets
        for preset in Self::builtin_presets() {
            self.presets.insert(preset.id.clone(), preset);
        }

        let mut entries = tokio::fs::read_dir(&self.presets_dir)
            .await
            .map_err(|e| FrameError::Io(format!("Failed to read presets directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| FrameError::Io(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                match Self::load_preset_file(&path).await {
                    Ok(preset) => {
                        // User presets override built-ins if same ID
                        self.presets.insert(preset.id.clone(), preset);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load preset from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get all built-in presets as ExportPresets
    pub fn builtin_presets() -> Vec<ExportPreset> {
        BuiltInPreset::all()
            .into_iter()
            .map(|p| p.to_preset())
            .collect()
    }

    /// Load a single preset from a file
    async fn load_preset_file(path: &Path) -> FrameResult<ExportPreset> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| FrameError::Io(format!("Failed to read preset file: {}", e)))?;

        let preset: ExportPreset = serde_json::from_str(&content)
            .map_err(|e| FrameError::Serialization("preset JSON".to_string(), e.to_string()))?;

        preset.validate()?;
        Ok(preset)
    }

    /// Save a preset to disk
    pub async fn save_preset(&mut self, preset: &ExportPreset) -> FrameResult<()> {
        preset.validate()?;

        self.ensure_directory().await?;

        let filename = format!("{}.json", preset.id);
        let path = self.presets_dir.join(&filename);

        let content = serde_json::to_string_pretty(preset)
            .map_err(|e| FrameError::Serialization("preset".to_string(), e.to_string()))?;

        tokio::fs::write(&path, content)
            .await
            .map_err(|e| FrameError::Io(format!("Failed to write preset file: {}", e)))?;

        self.presets.insert(preset.id.clone(), preset.clone());

        tracing::info!("Saved preset '{}' to {:?}", preset.name, path);
        Ok(())
    }

    /// Delete a preset by ID
    /// Returns Ok(false) if preset doesn't exist, Err if it's a built-in preset
    pub async fn delete_preset(&mut self, id: &str) -> FrameResult<bool> {
        // Check if this is a built-in preset
        if let Some(preset) = self.presets.get(id) {
            if preset.is_built_in {
                return Err(FrameError::Configuration(
                    "Cannot delete built-in presets".to_string(),
                ));
            }
        }

        let filename = format!("{}.json", id);
        let path = self.presets_dir.join(&filename);

        if path.exists() {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| FrameError::Io(format!("Failed to delete preset file: {}", e)))?;
        }

        Ok(self.presets.remove(id).is_some())
    }

    /// Check if a preset is built-in
    pub fn is_built_in_preset(&self, id: &str) -> bool {
        self.presets.get(id).map(|p| p.is_built_in).unwrap_or(false)
    }

    /// Get only user-created (non-built-in) presets
    pub fn get_user_presets(&self) -> Vec<&ExportPreset> {
        self.presets.values().filter(|p| !p.is_built_in).collect()
    }

    /// Get only built-in presets
    pub fn get_builtin_presets(&self) -> Vec<&ExportPreset> {
        self.presets.values().filter(|p| p.is_built_in).collect()
    }

    /// Get a preset by ID
    pub fn get_preset(&self, id: &str) -> Option<&ExportPreset> {
        self.presets.get(id)
    }

    /// Get all presets
    pub fn get_all_presets(&self) -> Vec<&ExportPreset> {
        self.presets.values().collect()
    }

    /// Get presets sorted by name
    pub fn get_presets_sorted(&self) -> Vec<&ExportPreset> {
        let mut presets: Vec<_> = self.presets.values().collect();
        presets.sort_by(|a, b| a.name.cmp(&b.name));
        presets
    }

    /// Check if a preset exists
    pub fn has_preset(&self, id: &str) -> bool {
        self.presets.contains_key(id)
    }

    /// Get the number of presets
    pub fn preset_count(&self) -> usize {
        self.presets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_codec_display() {
        assert_eq!(format!("{}", VideoCodec::H264), "H.264");
        assert_eq!(format!("{}", VideoCodec::H265), "H.265");
        assert_eq!(format!("{}", VideoCodec::Vp9), "VP9");
        assert_eq!(format!("{}", VideoCodec::ProRes), "ProRes");
    }

    #[test]
    fn test_export_output_format_display() {
        assert_eq!(format!("{}", ExportOutputFormat::Mp4), "MP4");
        assert_eq!(format!("{}", ExportOutputFormat::Mov), "MOV");
        assert_eq!(format!("{}", ExportOutputFormat::Webm), "WebM");
    }

    #[test]
    fn test_quality_preset_values() {
        assert_eq!(QualityPreset::Low.recommended_bitrate(), 2500);
        assert_eq!(QualityPreset::Medium.recommended_bitrate(), 5000);
        assert_eq!(QualityPreset::High.recommended_bitrate(), 10000);
        assert_eq!(QualityPreset::Lossless.recommended_bitrate(), 0);

        assert_eq!(QualityPreset::Low.recommended_crf(), 28);
        assert_eq!(QualityPreset::Medium.recommended_crf(), 23);
        assert_eq!(QualityPreset::High.recommended_crf(), 18);
        assert_eq!(QualityPreset::Lossless.recommended_crf(), 0);
    }

    #[test]
    fn test_export_preset_default() {
        let preset = ExportPreset::default();
        assert!(!preset.id.is_empty());
        assert_eq!(preset.name, "Default");
        assert_eq!(preset.codec, VideoCodec::H264);
        assert_eq!(preset.format, ExportOutputFormat::Mp4);
        assert_eq!(preset.width, 1920);
        assert_eq!(preset.height, 1080);
        assert_eq!(preset.fps, 30);
        assert_eq!(preset.quality, QualityPreset::Medium);
    }

    #[test]
    fn test_export_preset_new() {
        let preset = ExportPreset::new("My Preset");
        assert_eq!(preset.name, "My Preset");
        assert!(!preset.id.is_empty());
    }

    #[test]
    fn test_export_preset_for_resolution() {
        let preset = ExportPreset::for_resolution("4K", 3840, 2160);
        assert_eq!(preset.width, 3840);
        assert_eq!(preset.height, 2160);
        assert!(preset.bitrate > 5000); // Should be higher than 1080p
    }

    #[test]
    fn test_export_preset_with_quality() {
        let preset = ExportPreset::new("High Quality").with_quality(QualityPreset::High);
        assert_eq!(preset.quality, QualityPreset::High);
        assert_eq!(preset.bitrate, 10000);
    }

    #[test]
    fn test_export_preset_with_codec() {
        let preset = ExportPreset::new("VP9 Export").with_codec(VideoCodec::Vp9);
        assert_eq!(preset.codec, VideoCodec::Vp9);
        assert_eq!(preset.format, ExportOutputFormat::Webm);

        let preset = ExportPreset::new("ProRes Export").with_codec(VideoCodec::ProRes);
        assert_eq!(preset.codec, VideoCodec::ProRes);
        assert_eq!(preset.format, ExportOutputFormat::Mov);
    }

    #[test]
    fn test_export_preset_effective_crf() {
        let preset = ExportPreset::new("Test").with_quality(QualityPreset::High);
        assert_eq!(preset.effective_crf(), 18);

        let preset = ExportPreset::new("Custom")
            .with_quality(QualityPreset::Custom)
            .with_custom_crf(15);
        assert_eq!(preset.effective_crf(), 15);
    }

    #[test]
    fn test_export_preset_uses_crf() {
        let preset = ExportPreset::new("Test").with_quality(QualityPreset::Lossless);
        assert!(preset.uses_crf());

        let preset = ExportPreset::new("Test").with_quality(QualityPreset::Medium);
        assert!(!preset.uses_crf());
    }

    #[test]
    fn test_export_preset_validate() {
        let preset = ExportPreset::default();
        assert!(preset.validate().is_ok());

        let invalid = ExportPreset::new("");
        assert!(invalid.validate().is_err());

        let mut invalid = ExportPreset::new("Test");
        invalid.width = 0;
        assert!(invalid.validate().is_err());

        let mut invalid = ExportPreset::new("Test");
        invalid.fps = 0;
        assert!(invalid.validate().is_err());

        let mut invalid = ExportPreset::new("Test");
        invalid.quality = QualityPreset::Custom;
        invalid.custom_crf = None;
        assert!(invalid.validate().is_err());
    }

    #[tokio::test]
    async fn test_preset_manager_load_save_delete() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = PresetManager::with_directory(temp_dir.path());

        // Load should work even with empty directory
        manager.load_presets().await.unwrap();
        assert_eq!(manager.preset_count(), 0);

        // Save a preset
        let preset = ExportPreset::new("Test Preset");
        let id = preset.id.clone();
        manager.save_preset(&preset).await.unwrap();
        assert!(manager.has_preset(&id));

        // Reload and verify
        manager.load_presets().await.unwrap();
        assert_eq!(manager.preset_count(), 1);

        let loaded = manager.get_preset(&id).unwrap();
        assert_eq!(loaded.name, "Test Preset");

        // Delete the preset
        let deleted = manager.delete_preset(&id).await.unwrap();
        assert!(deleted);
        assert!(!manager.has_preset(&id));

        // Deleting non-existent should return false
        let deleted = manager.delete_preset("nonexistent").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_preset_manager_get_all_sorted() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = PresetManager::with_directory(temp_dir.path());

        let preset1 = ExportPreset::new("Zebra");
        let preset2 = ExportPreset::new("Alpha");
        let preset3 = ExportPreset::new("Beta");

        manager.save_preset(&preset1).await.unwrap();
        manager.save_preset(&preset2).await.unwrap();
        manager.save_preset(&preset3).await.unwrap();

        let all = manager.get_all_presets();
        assert_eq!(all.len(), 3);

        let sorted = manager.get_presets_sorted();
        assert_eq!(sorted[0].name, "Alpha");
        assert_eq!(sorted[1].name, "Beta");
        assert_eq!(sorted[2].name, "Zebra");
    }

    #[test]
    fn test_serialization() {
        let preset = ExportPreset::new("Test")
            .with_codec(VideoCodec::H265)
            .with_quality(QualityPreset::High);

        let json = serde_json::to_string(&preset).unwrap();
        assert!(json.contains("H265"));
        assert!(json.contains("high"));

        let deserialized: ExportPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(preset, deserialized);
    }

    #[test]
    fn test_builtin_preset_ids() {
        assert_eq!(BuiltInPreset::Web.id(), "builtin-web");
        assert_eq!(BuiltInPreset::YouTube.id(), "builtin-youtube");
        assert_eq!(BuiltInPreset::Twitter.id(), "builtin-twitter");
        assert_eq!(BuiltInPreset::Instagram.id(), "builtin-instagram");
        assert_eq!(BuiltInPreset::GifPreview.id(), "builtin-gif-preview");
        assert_eq!(BuiltInPreset::Discord.id(), "builtin-discord");
        assert_eq!(BuiltInPreset::Lossless.id(), "builtin-lossless");
    }

    #[test]
    fn test_builtin_preset_names() {
        assert_eq!(BuiltInPreset::Web.name(), "Web (1080p)");
        assert_eq!(BuiltInPreset::YouTube.name(), "YouTube (4K)");
        assert_eq!(BuiltInPreset::Twitter.name(), "Twitter/X (720p)");
        assert_eq!(BuiltInPreset::Instagram.name(), "Instagram (Square)");
        assert_eq!(BuiltInPreset::GifPreview.name(), "GIF Preview");
        assert_eq!(BuiltInPreset::Discord.name(), "Discord (720p)");
        assert_eq!(BuiltInPreset::Lossless.name(), "Lossless (ProRes)");
    }

    #[test]
    fn test_builtin_preset_to_preset_web() {
        let preset = BuiltInPreset::Web.to_preset();
        assert_eq!(preset.id, "builtin-web");
        assert_eq!(preset.name, "Web (1080p)");
        assert_eq!(preset.codec, VideoCodec::H264);
        assert_eq!(preset.format, ExportOutputFormat::Mp4);
        assert_eq!(preset.width, 1920);
        assert_eq!(preset.height, 1080);
        assert_eq!(preset.bitrate, 8000);
        assert_eq!(preset.fps, 30);
        assert!(preset.is_built_in);
    }

    #[test]
    fn test_builtin_preset_to_preset_youtube() {
        let preset = BuiltInPreset::YouTube.to_preset();
        assert_eq!(preset.id, "builtin-youtube");
        assert_eq!(preset.name, "YouTube (4K)");
        assert_eq!(preset.codec, VideoCodec::H264);
        assert_eq!(preset.width, 3840);
        assert_eq!(preset.height, 2160);
        assert_eq!(preset.bitrate, 20000);
    }

    #[test]
    fn test_builtin_preset_to_preset_twitter() {
        let preset = BuiltInPreset::Twitter.to_preset();
        assert_eq!(preset.id, "builtin-twitter");
        assert_eq!(preset.name, "Twitter/X (720p)");
        assert_eq!(preset.width, 1280);
        assert_eq!(preset.height, 720);
        assert_eq!(preset.bitrate, 5000);
    }

    #[test]
    fn test_builtin_preset_to_preset_instagram() {
        let preset = BuiltInPreset::Instagram.to_preset();
        assert_eq!(preset.id, "builtin-instagram");
        assert_eq!(preset.name, "Instagram (Square)");
        assert_eq!(preset.width, 1080);
        assert_eq!(preset.height, 1080); // Square format
    }

    #[test]
    fn test_builtin_preset_to_preset_gif_preview() {
        let preset = BuiltInPreset::GifPreview.to_preset();
        assert_eq!(preset.id, "builtin-gif-preview");
        assert_eq!(preset.width, 640);
        assert_eq!(preset.height, 480);
        assert_eq!(preset.fps, 10);
    }

    #[test]
    fn test_builtin_preset_to_preset_discord() {
        let preset = BuiltInPreset::Discord.to_preset();
        assert_eq!(preset.id, "builtin-discord");
        assert_eq!(preset.name, "Discord (720p)");
        assert_eq!(preset.bitrate, 6000);
    }

    #[test]
    fn test_builtin_preset_to_preset_lossless() {
        let preset = BuiltInPreset::Lossless.to_preset();
        assert_eq!(preset.id, "builtin-lossless");
        assert_eq!(preset.codec, VideoCodec::ProRes);
        assert_eq!(preset.format, ExportOutputFormat::Mov);
        assert_eq!(preset.quality, QualityPreset::Lossless);
        assert_eq!(preset.bitrate, 0);
    }

    #[test]
    fn test_builtin_preset_all() {
        let all = BuiltInPreset::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&BuiltInPreset::Web));
        assert!(all.contains(&BuiltInPreset::YouTube));
        assert!(all.contains(&BuiltInPreset::Twitter));
        assert!(all.contains(&BuiltInPreset::Instagram));
        assert!(all.contains(&BuiltInPreset::GifPreview));
        assert!(all.contains(&BuiltInPreset::Discord));
        assert!(all.contains(&BuiltInPreset::Lossless));
    }

    #[test]
    fn test_preset_manager_builtin_presets() {
        let builtins = PresetManager::builtin_presets();
        assert_eq!(builtins.len(), 7);

        // Check that all have is_built_in = true
        for preset in &builtins {
            assert!(preset.is_built_in, "{} should be built-in", preset.name);
        }

        // Check specific IDs exist
        let ids: Vec<_> = builtins.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"builtin-web"));
        assert!(ids.contains(&"builtin-youtube"));
        assert!(ids.contains(&"builtin-lossless"));
    }

    #[tokio::test]
    async fn test_preset_manager_load_includes_builtins() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = PresetManager::with_directory(temp_dir.path());

        manager.load_presets().await.unwrap();

        // Should have all built-in presets
        assert!(manager.has_preset("builtin-web"));
        assert!(manager.has_preset("builtin-youtube"));
        assert!(manager.has_preset("builtin-twitter"));
        assert!(manager.has_preset("builtin-instagram"));
        assert!(manager.has_preset("builtin-gif-preview"));
        assert!(manager.has_preset("builtin-discord"));
        assert!(manager.has_preset("builtin-lossless"));

        // Total should be 7 (built-ins only, no user presets yet)
        assert_eq!(manager.preset_count(), 7);
    }

    #[tokio::test]
    async fn test_preset_manager_cannot_delete_builtin() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = PresetManager::with_directory(temp_dir.path());

        manager.load_presets().await.unwrap();

        // Attempt to delete a built-in preset should fail
        let result = manager.delete_preset("builtin-web").await;
        assert!(result.is_err());
        assert!(manager.has_preset("builtin-web"));
    }

    #[tokio::test]
    async fn test_preset_manager_is_built_in_preset() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = PresetManager::with_directory(temp_dir.path());

        manager.load_presets().await.unwrap();

        assert!(manager.is_built_in_preset("builtin-web"));
        assert!(manager.is_built_in_preset("builtin-youtube"));
        assert!(!manager.is_built_in_preset("nonexistent"));

        // Add a user preset
        let user_preset = ExportPreset::new("User Preset");
        let user_id = user_preset.id.clone();
        manager.save_preset(&user_preset).await.unwrap();

        assert!(!manager.is_built_in_preset(&user_id));
    }

    #[tokio::test]
    async fn test_preset_manager_get_user_and_builtin_presets() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = PresetManager::with_directory(temp_dir.path());

        manager.load_presets().await.unwrap();

        // Initially only built-ins
        let builtins = manager.get_builtin_presets();
        let users = manager.get_user_presets();
        assert_eq!(builtins.len(), 7);
        assert_eq!(users.len(), 0);

        // Add a user preset
        let user_preset = ExportPreset::new("My Custom Preset");
        manager.save_preset(&user_preset).await.unwrap();

        let builtins = manager.get_builtin_presets();
        let users = manager.get_user_presets();
        assert_eq!(builtins.len(), 7);
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].name, "My Custom Preset");
    }

    #[test]
    fn test_export_preset_built_in_builder() {
        let preset = ExportPreset::new("Test").built_in();
        assert!(preset.is_built_in);
    }

    #[test]
    fn test_builtin_preset_serialization_roundtrip() {
        let preset = BuiltInPreset::YouTube.to_preset();
        let json = serde_json::to_string(&preset).unwrap();
        let deserialized: ExportPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(preset.id, deserialized.id);
        assert_eq!(preset.name, deserialized.name);
        assert_eq!(preset.is_built_in, deserialized.is_built_in);
        assert!(deserialized.is_built_in);
    }
}

/// Extension trait for ExportPreset
pub trait ExportPresetExt {
    /// Set custom CRF value
    fn with_custom_crf(self, crf: u8) -> Self;
}

impl ExportPresetExt for ExportPreset {
    fn with_custom_crf(mut self, crf: u8) -> Self {
        self.custom_crf = Some(crf);
        self
    }
}
