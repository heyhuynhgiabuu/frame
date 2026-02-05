use crate::FrameResult;

pub mod platform;

#[cfg(feature = "capture")]
pub mod audio;

/// Represents a captured frame
#[derive(Debug, Clone)]
pub struct Frame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub timestamp: std::time::Duration,
    pub format: PixelFormat,
}

#[derive(Debug, Clone, Copy)]
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

/// Configuration for screen capture
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub capture_area: CaptureArea,
    pub capture_cursor: bool,
    pub capture_audio: bool,
    pub frame_rate: u32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            capture_area: CaptureArea::FullScreen,
            capture_cursor: true,
            capture_audio: true,
            frame_rate: 60,
        }
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
