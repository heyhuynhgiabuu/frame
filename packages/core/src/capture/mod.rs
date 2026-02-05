use crate::FrameResult;
use serde::{Deserialize, Serialize};

pub mod platform;

#[cfg(feature = "capture")]
pub mod audio;

#[cfg(feature = "webcam")]
pub mod webcam;

/// Represents a captured frame
#[derive(Debug, Clone)]
pub struct Frame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: std::time::Duration,
    pub format: PixelFormat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PixelFormat {
    Rgba,
    Bgra,
    Yuv420,
    Yuv422,
}

/// Audio sample buffer
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub timestamp: std::time::Duration,
}

/// Represents a capture region (sub-area of the screen)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CaptureRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl CaptureRegion {
    /// Create a new capture region
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if the region is valid (non-zero dimensions)
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// Get the region as a tuple (x, y, width, height)
    pub fn to_tuple(&self) -> (u32, u32, u32, u32) {
        (self.x, self.y, self.width, self.height)
    }
}

/// Configuration for screen capture
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub capture_area: CaptureArea,
    pub capture_cursor: bool,
    pub capture_audio: bool,
    pub frame_rate: u32,
    /// Optional capture region for partial screen recording
    /// When Some, captures only the specified region; when None, captures full screen
    pub capture_region: Option<CaptureRegion>,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            capture_area: CaptureArea::FullScreen,
            capture_cursor: true,
            capture_audio: true,
            frame_rate: 60,
            capture_region: None,
        }
    }
}

impl CaptureConfig {
    /// Create a new capture config with the specified region
    pub fn with_region(mut self, region: CaptureRegion) -> Self {
        self.capture_region = Some(region);
        self
    }

    /// Create a new capture config for full screen capture (no region)
    pub fn full_screen() -> Self {
        Self::default()
    }

    /// Check if this config has a valid capture region set
    pub fn has_region(&self) -> bool {
        self.capture_region.is_some_and(|r| r.is_valid())
    }
}

#[derive(Debug, Clone)]
pub enum CaptureArea {
    FullScreen,
    Window {
        window_id: u64,
    },
    Region {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
}

/// Trait for platform-specific capture implementations
#[async_trait::async_trait]
pub trait ScreenCapture: Send + Sync {
    async fn start(&mut self, config: CaptureConfig) -> FrameResult<()>;
    async fn stop(&mut self) -> FrameResult<()>;
    async fn next_frame(&mut self) -> FrameResult<Option<Frame>>;
    async fn next_audio_buffer(&mut self) -> FrameResult<Option<AudioBuffer>>;
}

// Re-export platform-specific implementations
#[cfg(all(target_os = "macos", feature = "capture"))]
pub use platform::macos::{DisplayInfo, MacOSScreenCapture, WindowInfo};

// Re-export audio capture
#[cfg(feature = "capture")]
pub use audio::microphone::{AudioConfig, AudioDeviceInfo, MicrophoneCapture};
#[cfg(feature = "capture")]
pub use audio::mixer::AudioMixer;
#[cfg(feature = "capture")]
pub use audio::resampler::SampleRateConverter;

// Re-export webcam capture
#[cfg(feature = "webcam")]
pub use webcam::{WebcamCapture, WebcamConfig, WebcamDevice};

#[cfg(feature = "capture")]
pub fn create_capture() -> FrameResult<Box<dyn ScreenCapture>> {
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(MacOSScreenCapture::new()?))
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(crate::FrameError::PlatformNotSupported(
            std::env::consts::OS.to_string(),
        ))
    }
}

#[cfg(not(feature = "capture"))]
pub fn create_capture() -> FrameResult<Box<dyn ScreenCapture>> {
    Err(crate::FrameError::PlatformNotSupported(
        "capture feature not enabled".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_region_new() {
        let region = CaptureRegion::new(100, 200, 800, 600);
        assert_eq!(region.x, 100);
        assert_eq!(region.y, 200);
        assert_eq!(region.width, 800);
        assert_eq!(region.height, 600);
    }

    #[test]
    fn test_capture_region_is_valid() {
        // Valid region
        let valid = CaptureRegion::new(0, 0, 100, 100);
        assert!(valid.is_valid());

        // Invalid: zero width
        let invalid_width = CaptureRegion::new(0, 0, 0, 100);
        assert!(!invalid_width.is_valid());

        // Invalid: zero height
        let invalid_height = CaptureRegion::new(0, 0, 100, 0);
        assert!(!invalid_height.is_valid());

        // Invalid: both zero
        let invalid_both = CaptureRegion::new(0, 0, 0, 0);
        assert!(!invalid_both.is_valid());
    }

    #[test]
    fn test_capture_region_to_tuple() {
        let region = CaptureRegion::new(50, 100, 1920, 1080);
        let tuple = region.to_tuple();
        assert_eq!(tuple, (50, 100, 1920, 1080));
    }

    #[test]
    fn test_capture_config_default() {
        let config = CaptureConfig::default();
        assert!(matches!(config.capture_area, CaptureArea::FullScreen));
        assert!(config.capture_cursor);
        assert!(config.capture_audio);
        assert_eq!(config.frame_rate, 60);
        assert!(config.capture_region.is_none());
        assert!(!config.has_region());
    }

    #[test]
    fn test_capture_config_full_screen() {
        let config = CaptureConfig::full_screen();
        assert!(matches!(config.capture_area, CaptureArea::FullScreen));
        assert!(config.capture_region.is_none());
        assert!(!config.has_region());
    }

    #[test]
    fn test_capture_config_with_region() {
        let region = CaptureRegion::new(100, 200, 800, 600);
        let config = CaptureConfig::default().with_region(region);

        assert!(config.capture_region.is_some());
        assert!(config.has_region());

        let stored_region = config.capture_region.unwrap();
        assert_eq!(stored_region.x, 100);
        assert_eq!(stored_region.y, 200);
        assert_eq!(stored_region.width, 800);
        assert_eq!(stored_region.height, 600);
    }

    #[test]
    fn test_capture_config_has_region_with_invalid() {
        let config = CaptureConfig {
            capture_region: Some(CaptureRegion::new(0, 0, 0, 100)), // Invalid: zero width
            ..Default::default()
        };

        // has_region should return false for invalid regions
        assert!(!config.has_region());
    }

    #[test]
    fn test_capture_config_serialization() {
        // Test that CaptureRegion serializes/deserializes correctly
        let region = CaptureRegion::new(50, 100, 1920, 1080);
        let json = serde_json::to_string(&region).unwrap();
        let deserialized: CaptureRegion = serde_json::from_str(&json).unwrap();
        assert_eq!(region, deserialized);
    }

    #[test]
    fn test_capture_region_serialization_roundtrip() {
        let region = CaptureRegion {
            x: 100,
            y: 200,
            width: 800,
            height: 600,
        };

        let json = serde_json::to_string(&region).expect("Failed to serialize");
        let decoded: CaptureRegion = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(region.x, decoded.x);
        assert_eq!(region.y, decoded.y);
        assert_eq!(region.width, decoded.width);
        assert_eq!(region.height, decoded.height);
    }
}
