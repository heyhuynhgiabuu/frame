//! Stub screen capture for UI testing without real ScreenCaptureKit
//!
//! Enabled with `--features stub-capture` and generates fake test frames.

use crate::capture::{AudioBuffer, CaptureArea, CaptureConfig, Frame, PixelFormat, ScreenCapture};
use crate::{FrameError, FrameResult};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Stub screen capture that generates test pattern frames
pub struct StubScreenCapture {
    config: Option<CaptureConfig>,
    is_recording: Arc<AtomicBool>,
    start_time: Option<Instant>,
    frame_count: AtomicU64,
}

impl StubScreenCapture {
    pub fn new() -> FrameResult<Self> {
        Ok(Self {
            config: None,
            is_recording: Arc::new(AtomicBool::new(false)),
            start_time: None,
            frame_count: AtomicU64::new(0),
        })
    }

    /// Always returns true (no permission needed for stub)
    pub fn check_permission() -> FrameResult<bool> {
        Ok(true)
    }

    /// Returns a fake display
    pub fn get_displays() -> FrameResult<Vec<DisplayInfo>> {
        Ok(vec![DisplayInfo {
            id: 1,
            width: 1920,
            height: 1080,
            frame_rate: 60,
        }])
    }

    /// Returns empty window list
    pub fn get_windows() -> FrameResult<Vec<WindowInfo>> {
        Ok(vec![])
    }

    /// Generate a test pattern frame (gradient with frame counter)
    fn generate_test_frame(&self, width: u32, height: u32, frame_num: u64) -> Vec<u8> {
        let mut data = vec![0u8; (width * height * 4) as usize];

        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;

                // Create a moving gradient pattern
                let r = ((x as f32 / width as f32) * 255.0) as u8;
                let g = ((y as f32 / height as f32) * 255.0) as u8;
                let b = (((frame_num % 256) as f32 / 255.0) * 128.0 + 64.0) as u8;

                // BGRA format
                data[idx] = b; // B
                data[idx + 1] = g; // G
                data[idx + 2] = r; // R
                data[idx + 3] = 255; // A
            }
        }

        data
    }
}

impl Default for StubScreenCapture {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[async_trait]
impl ScreenCapture for StubScreenCapture {
    async fn start(&mut self, config: CaptureConfig) -> FrameResult<()> {
        tracing::info!("Starting STUB screen capture (test mode)");

        self.config = Some(config);
        self.start_time = Some(Instant::now());
        self.frame_count.store(0, Ordering::SeqCst);
        self.is_recording.store(true, Ordering::SeqCst);

        Ok(())
    }

    async fn stop(&mut self) -> FrameResult<()> {
        tracing::info!("Stopping STUB screen capture");
        self.is_recording.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn next_frame(&mut self) -> FrameResult<Option<Frame>> {
        if !self.is_recording.load(Ordering::SeqCst) {
            return Ok(None);
        }

        let config = self
            .config
            .as_ref()
            .ok_or_else(|| FrameError::CaptureError("Capture not configured".into()))?;

        // Simulate frame rate
        let frame_interval = Duration::from_secs_f64(1.0 / config.frame_rate as f64);
        tokio::time::sleep(frame_interval).await;

        let frame_num = self.frame_count.fetch_add(1, Ordering::SeqCst);
        let (width, height) = match &config.capture_area {
            CaptureArea::FullScreen { .. } => (1920, 1080),
            CaptureArea::Window { .. } => (1280, 720),
            CaptureArea::Region { width, height, .. } => (*width, *height),
        };

        let timestamp = self.start_time.map(|t| t.elapsed()).unwrap_or_default();

        let data = self.generate_test_frame(width, height, frame_num);

        Ok(Some(Frame {
            data,
            width,
            height,
            timestamp,
            format: PixelFormat::Bgra,
        }))
    }

    async fn next_audio_buffer(&mut self) -> FrameResult<Option<AudioBuffer>> {
        // Generate silent audio
        if !self.is_recording.load(Ordering::SeqCst) {
            return Ok(None);
        }

        let config = self
            .config
            .as_ref()
            .ok_or_else(|| FrameError::CaptureError("Capture not configured".into()))?;

        if !config.capture_audio {
            return Ok(None);
        }

        // Simulate ~20ms of silence at 48kHz
        tokio::time::sleep(Duration::from_millis(20)).await;

        let samples = vec![0.0f32; 48000 / 50 * 2]; // 20ms of stereo silence

        Ok(Some(AudioBuffer {
            samples,
            sample_rate: 48000,
            channels: 2,
            timestamp: self.start_time.map(|t| t.elapsed()).unwrap_or_default(),
        }))
    }
}

/// Information about an available display
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub frame_rate: u32,
}

/// Information about an available window
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: u64,
    pub title: String,
    pub app_name: String,
    pub width: u32,
    pub height: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stub_capture_creates() {
        let capture = StubScreenCapture::new();
        assert!(capture.is_ok());
    }

    #[tokio::test]
    async fn test_stub_capture_permission() {
        assert!(StubScreenCapture::check_permission().unwrap());
    }

    #[tokio::test]
    async fn test_stub_capture_displays() {
        let displays = StubScreenCapture::get_displays().unwrap();
        assert_eq!(displays.len(), 1);
        assert_eq!(displays[0].width, 1920);
    }
}
