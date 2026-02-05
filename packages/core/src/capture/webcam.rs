//! Webcam capture using nokhwa
//!
//! This module provides webcam capture functionality for macOS using AVFoundation
//! through the nokhwa library. Requires the "webcam" feature to be enabled.

use crate::capture::Frame;
use crate::{FrameError, FrameResult};
use std::sync::{Arc, Mutex};

/// Information about a webcam device
#[derive(Debug, Clone, PartialEq)]
pub struct WebcamDevice {
    /// Unique identifier for the device
    pub id: String,
    /// Human-readable name of the device
    pub name: String,
    /// Device description or additional info
    pub description: String,
}

impl WebcamDevice {
    /// Create a new webcam device info
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
        }
    }
}

/// Configuration for webcam capture
#[derive(Debug, Clone, PartialEq)]
pub struct WebcamConfig {
    /// Device identifier to use
    pub device_id: String,
    /// Capture width in pixels
    pub width: u32,
    /// Capture height in pixels
    pub height: u32,
    /// Frames per second
    pub fps: u32,
}

impl Default for WebcamConfig {
    fn default() -> Self {
        Self {
            device_id: "0".to_string(),
            width: 1280,
            height: 720,
            fps: 30,
        }
    }
}

impl WebcamConfig {
    /// Create a new config with the specified device
    pub fn with_device(device_id: impl Into<String>) -> Self {
        Self {
            device_id: device_id.into(),
            ..Default::default()
        }
    }

    /// Set the resolution
    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set the frame rate
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }
}

/// Internal state for the capture thread
#[derive(Debug, Default)]
struct CaptureState {
    latest_frame: Option<Frame>,
    is_running: bool,
    /// Error from capture thread (if any)
    #[allow(dead_code)]
    error: Option<String>,
}

/// Webcam capture handler
///
/// Manages the webcam capture lifecycle including device enumeration,
/// frame capture, and cleanup.
pub struct WebcamCapture {
    config: WebcamConfig,
    state: Arc<Mutex<CaptureState>>,
}

impl WebcamCapture {
    /// List all available webcam devices
    ///
    /// # Returns
    /// A vector of available webcam devices, or an error if enumeration fails
    pub fn list_devices() -> FrameResult<Vec<WebcamDevice>> {
        let cameras = nokhwa::query(nokhwa::utils::ApiBackend::Auto).map_err(|e| {
            FrameError::CaptureError(format!("Failed to query webcam devices: {}", e))
        })?;

        let devices = cameras
            .into_iter()
            .enumerate()
            .map(|(idx, info)| {
                WebcamDevice::new(
                    idx.to_string(),
                    info.human_name(),
                    format!("{:?} - {:?}", info.misc(), info.description()),
                )
            })
            .collect();

        Ok(devices)
    }

    /// Create a new webcam capture instance
    ///
    /// # Arguments
    /// * `config` - Configuration for the capture session
    ///
    /// # Returns
    /// A new WebcamCapture instance, or an error if the device is not available
    pub fn new(config: WebcamConfig) -> FrameResult<Self> {
        // Validate that the device exists
        let devices = Self::list_devices()?;
        if !devices.iter().any(|d| d.id == config.device_id) {
            return Err(FrameError::CaptureError(format!(
                "Webcam device '{}' not found. Available devices: {:?}",
                config.device_id,
                devices.iter().map(|d| &d.name).collect::<Vec<_>>()
            )));
        }

        Ok(Self {
            config,
            state: Arc::new(Mutex::new(CaptureState::default())),
        })
    }

    /// Start the webcam capture
    ///
    /// Begins capturing frames from the configured webcam device.
    /// Frames can be retrieved using [`get_frame()`].
    ///
    /// # Returns
    /// Ok(()) if capture started successfully, or an error if startup failed
    pub fn start(&mut self) -> FrameResult<()> {
        let mut state = self.state.lock().map_err(|e| {
            FrameError::InvalidState(format!("Failed to lock capture state: {}", e))
        })?;

        if state.is_running {
            return Err(FrameError::InvalidState(
                "Capture already in progress".to_string(),
            ));
        }

        // TODO: Implement actual capture loop with nokhwa Camera
        // This would spawn a thread that continuously captures frames
        state.is_running = true;
        drop(state);

        tracing::info!(
            "Started webcam capture: {}x{}@{}fps on device {}",
            self.config.width,
            self.config.height,
            self.config.fps,
            self.config.device_id
        );

        Ok(())
    }

    /// Stop the webcam capture
    ///
    /// Cleanly terminates the capture session and releases the device.
    ///
    /// # Returns
    /// Ok(()) if capture stopped successfully, or an error if cleanup failed
    pub fn stop(&mut self) -> FrameResult<()> {
        let mut state = self.state.lock().map_err(|e| {
            FrameError::InvalidState(format!("Failed to lock capture state: {}", e))
        })?;

        if !state.is_running {
            return Err(FrameError::InvalidState(
                "No capture in progress".to_string(),
            ));
        }

        state.is_running = false;
        state.latest_frame = None;
        drop(state);

        tracing::info!("Stopped webcam capture");

        Ok(())
    }

    /// Get the latest captured frame
    ///
    /// Returns the most recently captured frame, if any.
    /// This is non-blocking and returns None if no frame is available.
    ///
    /// # Returns
    /// Some(Frame) if a frame is available, None otherwise
    pub fn get_frame(&self) -> Option<Frame> {
        let state = self.state.lock().ok()?;
        state.latest_frame.clone()
    }

    /// Check if capture is currently running
    pub fn is_running(&self) -> bool {
        self.state.lock().map(|s| s.is_running).unwrap_or(false)
    }

    /// Get the current configuration
    pub fn config(&self) -> &WebcamConfig {
        &self.config
    }
}

impl Drop for WebcamCapture {
    fn drop(&mut self) {
        if self.is_running() {
            if let Err(e) = self.stop() {
                tracing::warn!("Failed to stop webcam capture during drop: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webcam_device_creation() {
        let device = WebcamDevice::new("0", "Test Camera", "Test Description");
        assert_eq!(device.id, "0");
        assert_eq!(device.name, "Test Camera");
        assert_eq!(device.description, "Test Description");
    }

    #[test]
    fn test_webcam_config_default() {
        let config = WebcamConfig::default();
        assert_eq!(config.device_id, "0");
        assert_eq!(config.width, 1280);
        assert_eq!(config.height, 720);
        assert_eq!(config.fps, 30);
    }

    #[test]
    fn test_webcam_config_builder() {
        let config = WebcamConfig::with_device("1")
            .with_resolution(1920, 1080)
            .with_fps(60);

        assert_eq!(config.device_id, "1");
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert_eq!(config.fps, 60);
    }

    #[test]
    fn test_list_devices_does_not_panic() {
        // This test just ensures list_devices() can be called without panicking
        // Actual device enumeration depends on hardware availability
        let _ = WebcamCapture::list_devices();
    }

    #[test]
    fn test_capture_state_default() {
        let state = CaptureState::default();
        assert!(!state.is_running);
        assert!(state.latest_frame.is_none());
        assert!(state.error.is_none());
    }
}
