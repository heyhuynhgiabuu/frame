//! Platform-specific screen capture implementations

#[cfg(all(target_os = "macos", feature = "capture"))]
pub mod macos {
    use crate::capture::{
        AudioBuffer, CaptureArea, CaptureConfig, Frame, PixelFormat, ScreenCapture,
    };
    use crate::{FrameError, FrameResult};
    use async_trait::async_trait;
    use screencapturekit::cm::SCFrameStatus;
    use screencapturekit::prelude::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tokio::sync::mpsc;

    /// Frame received from ScreenCaptureKit
    struct CapturedFrame {
        data: Vec<u8>,
        width: u32,
        height: u32,
        timestamp: std::time::Duration,
        format: PixelFormat,
    }

    /// macOS Screen Capture using ScreenCaptureKit (macOS 12.3+)
    pub struct MacOSScreenCapture {
        config: Option<CaptureConfig>,
        is_recording: Arc<AtomicBool>,
        stream: Option<SCStream>,
        frame_receiver: Option<mpsc::Receiver<CapturedFrame>>,
        audio_receiver: Option<mpsc::Receiver<AudioBuffer>>,
    }

    impl MacOSScreenCapture {
        pub fn new() -> FrameResult<Self> {
            Ok(Self {
                config: None,
                is_recording: Arc::new(AtomicBool::new(false)),
                stream: None,
                frame_receiver: None,
                audio_receiver: None,
            })
        }

        /// Check and request screen recording permission
        pub fn check_permission() -> FrameResult<bool> {
            // SCShareableContent::get() will prompt for permission if needed
            match SCShareableContent::get() {
                Ok(_) => Ok(true),
                Err(e) => {
                    tracing::warn!("Screen recording permission check failed: {:?}", e);
                    Ok(false)
                }
            }
        }

        /// Get list of available displays
        pub fn get_displays() -> FrameResult<Vec<DisplayInfo>> {
            let content = SCShareableContent::get()
                .map_err(|e| FrameError::PermissionDenied(format!("Screen recording: {:?}", e)))?;

            let displays = content
                .displays()
                .iter()
                .map(|d| DisplayInfo {
                    id: d.display_id() as u64,
                    width: d.width() as u32,
                    height: d.height() as u32,
                    frame_rate: 60,
                })
                .collect();

            Ok(displays)
        }

        /// Get list of available windows
        pub fn get_windows() -> FrameResult<Vec<WindowInfo>> {
            let content = SCShareableContent::get()
                .map_err(|e| FrameError::PermissionDenied(format!("Screen recording: {:?}", e)))?;

            let windows = content
                .windows()
                .iter()
                .filter(|w| w.title().map(|t| !t.is_empty()).unwrap_or(false))
                .map(|w| {
                    let frame = w.frame();
                    let size = frame.size();
                    WindowInfo {
                        id: w.window_id() as u64,
                        title: w.title().unwrap_or_default().to_string(),
                        app_name: w
                            .owning_application()
                            .map(|a| a.application_name().to_string())
                            .unwrap_or_default(),
                        width: size.width as u32,
                        height: size.height as u32,
                    }
                })
                .collect();

            Ok(windows)
        }

        fn create_stream_config(
            &self,
            config: &CaptureConfig,
            width: u32,
            height: u32,
        ) -> SCStreamConfiguration {
            let mut stream_config = SCStreamConfiguration::new()
                .with_width(width)
                .with_height(height)
                .with_pixel_format(screencapturekit::stream::configuration::PixelFormat::BGRA)
                .with_shows_cursor(config.capture_cursor);

            // Set frame rate: CMTime(1, fps) = 1/fps seconds per frame
            stream_config.set_minimum_frame_interval(&CMTime::new(1, config.frame_rate as i32));

            // Audio capture (macOS 13.0+)
            if config.capture_audio {
                stream_config.set_captures_audio(true);
            }

            stream_config
        }
    }

    #[async_trait]
    impl ScreenCapture for MacOSScreenCapture {
        async fn start(&mut self, config: CaptureConfig) -> FrameResult<()> {
            if self.is_recording.load(Ordering::SeqCst) {
                return Err(FrameError::RecordingInProgress);
            }

            tracing::info!("Starting macOS screen capture with config: {:?}", config);

            // Get shareable content (prompts for permission)
            let content = SCShareableContent::get().map_err(|e| {
                FrameError::PermissionDenied(format!("Screen recording permission denied: {:?}", e))
            })?;

            // Determine what to capture
            let (filter, width, height) = match &config.capture_area {
                CaptureArea::FullScreen => {
                    let displays = content.displays();
                    let display = displays
                        .first()
                        .ok_or_else(|| FrameError::CaptureError("No displays found".into()))?;

                    let filter = SCContentFilter::create()
                        .with_display(display)
                        .with_excluding_windows(&[])
                        .build();
                    (filter, display.width() as u32, display.height() as u32)
                }
                CaptureArea::Window { window_id } => {
                    let windows = content.windows();
                    let window = windows
                        .iter()
                        .find(|w| w.window_id() as u64 == *window_id)
                        .ok_or_else(|| {
                            FrameError::CaptureError(format!("Window {} not found", window_id))
                        })?;

                    let filter = SCContentFilter::create().with_window(window).build();
                    let frame = window.frame();
                    let size = frame.size();
                    (filter, size.width as u32, size.height as u32)
                }
                CaptureArea::Region {
                    x,
                    y,
                    width,
                    height,
                } => {
                    let displays = content.displays();
                    let display = displays
                        .first()
                        .ok_or_else(|| FrameError::CaptureError("No displays found".into()))?;

                    let filter = SCContentFilter::create()
                        .with_display(display)
                        .with_excluding_windows(&[])
                        .build();
                    tracing::info!(
                        "Region capture: x={}, y={}, w={}, h={}",
                        x,
                        y,
                        width,
                        height
                    );
                    (filter, *width, *height)
                }
            };

            // Create stream configuration
            let stream_config = self.create_stream_config(&config, width, height);

            // Create channels for frames and audio
            let (frame_tx, frame_rx) = mpsc::channel::<CapturedFrame>(32);
            let (audio_tx, audio_rx) = mpsc::channel::<AudioBuffer>(64);

            let is_recording = self.is_recording.clone();
            let capture_audio = config.capture_audio;

            // Create the stream
            let mut stream = SCStream::new(&filter, &stream_config);

            // Add video output handler
            let frame_tx_clone = frame_tx.clone();
            let is_recording_clone = is_recording.clone();
            stream.add_output_handler(
                move |sample: CMSampleBuffer, _output_type: SCStreamOutputType| {
                    if !is_recording_clone.load(Ordering::SeqCst) {
                        return;
                    }

                    // Check frame status
                    let Some(status) = sample.frame_status() else {
                        return;
                    };
                    if status != SCFrameStatus::Complete {
                        return;
                    }

                    // Get the pixel buffer
                    let Some(pixel_buffer) = sample.image_buffer() else {
                        return;
                    };

                    let width = pixel_buffer.width() as u32;
                    let height = pixel_buffer.height() as u32;
                    let _bytes_per_row = pixel_buffer.bytes_per_row();
                    let data_size = pixel_buffer.data_size();

                    // Lock and copy pixel data
                    if let Ok(guard) = pixel_buffer.lock_read_only() {
                        let base_ptr = guard.base_address();

                        // Copy the raw pixel data
                        let data =
                            unsafe { std::slice::from_raw_parts(base_ptr, data_size).to_vec() };

                        // Explicitly drop the guard to unlock before we leave the scope
                        drop(guard);

                        // Get timestamp
                        let timestamp = std::time::Duration::from_secs_f64(
                            sample.presentation_timestamp().as_seconds().unwrap_or(0.0),
                        );

                        let frame = CapturedFrame {
                            data,
                            width,
                            height,
                            timestamp,
                            format: PixelFormat::Bgra,
                        };

                        let _ = frame_tx_clone.try_send(frame);
                    };
                },
                SCStreamOutputType::Screen,
            );

            // Add audio output handler if enabled
            if capture_audio {
                let audio_tx_clone = audio_tx.clone();
                let is_recording_clone = is_recording.clone();
                stream.add_output_handler(
                    move |sample: CMSampleBuffer, _output_type: SCStreamOutputType| {
                        if !is_recording_clone.load(Ordering::SeqCst) {
                            return;
                        }

                        // Get audio buffer list
                        let Some(audio_buffer_list) = sample.audio_buffer_list() else {
                            return;
                        };

                        // Get buffer count and collect samples
                        let mut samples: Vec<f32> = Vec::new();
                        let buffer_count = audio_buffer_list.num_buffers();

                        for i in 0..buffer_count {
                            if let Some(audio_buffer) = audio_buffer_list.get(i) {
                                let data: &[u8] = audio_buffer.data();
                                // Convert bytes to f32 samples (assuming 32-bit float format)
                                for chunk in data.chunks_exact(4) {
                                    let bytes: [u8; 4] = [chunk[0], chunk[1], chunk[2], chunk[3]];
                                    samples.push(f32::from_le_bytes(bytes));
                                }
                            }
                        }

                        if samples.is_empty() {
                            return;
                        }

                        let timestamp = std::time::Duration::from_secs_f64(
                            sample.presentation_timestamp().as_seconds().unwrap_or(0.0),
                        );

                        let buffer = AudioBuffer {
                            samples,
                            sample_rate: 48000,
                            channels: 2,
                            timestamp,
                        };

                        let _ = audio_tx_clone.try_send(buffer);
                    },
                    SCStreamOutputType::Audio,
                );
            }

            // Start the stream
            stream.start_capture().map_err(|e| {
                FrameError::CaptureError(format!("Failed to start capture: {:?}", e))
            })?;

            // Store state
            self.stream = Some(stream);
            self.frame_receiver = Some(frame_rx);
            self.audio_receiver = Some(audio_rx);
            self.config = Some(config);
            self.is_recording.store(true, Ordering::SeqCst);

            tracing::info!(
                "Screen capture started successfully ({}x{} @ {}fps)",
                width,
                height,
                self.config.as_ref().unwrap().frame_rate
            );

            Ok(())
        }

        async fn stop(&mut self) -> FrameResult<()> {
            if !self.is_recording.load(Ordering::SeqCst) {
                return Ok(());
            }

            tracing::info!("Stopping macOS screen capture");

            self.is_recording.store(false, Ordering::SeqCst);

            if let Some(stream) = self.stream.take() {
                stream.stop_capture().map_err(|e| {
                    FrameError::CaptureError(format!("Failed to stop capture: {:?}", e))
                })?;
            }

            self.frame_receiver = None;
            self.audio_receiver = None;

            tracing::info!("Screen capture stopped");
            Ok(())
        }

        async fn next_frame(&mut self) -> FrameResult<Option<Frame>> {
            if !self.is_recording.load(Ordering::SeqCst) {
                return Ok(None);
            }

            let receiver = self
                .frame_receiver
                .as_mut()
                .ok_or_else(|| FrameError::CaptureError("Capture not started".into()))?;

            match receiver.recv().await {
                Some(captured) => Ok(Some(Frame {
                    data: captured.data,
                    width: captured.width,
                    height: captured.height,
                    timestamp: captured.timestamp,
                    format: captured.format,
                })),
                None => Ok(None),
            }
        }

        async fn next_audio_buffer(&mut self) -> FrameResult<Option<AudioBuffer>> {
            if !self.is_recording.load(Ordering::SeqCst) {
                return Ok(None);
            }

            let receiver = self
                .audio_receiver
                .as_mut()
                .ok_or_else(|| FrameError::CaptureError("Capture not started".into()))?;

            match receiver.recv().await {
                Some(buffer) => Ok(Some(buffer)),
                None => Ok(None),
            }
        }
    }

    impl Drop for MacOSScreenCapture {
        fn drop(&mut self) {
            self.is_recording.store(false, Ordering::SeqCst);
            if let Some(stream) = self.stream.take() {
                let _ = stream.stop_capture();
            }
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
}

#[cfg(all(test, target_os = "macos", feature = "capture"))]
mod tests {
    use super::macos::*;

    #[test]
    fn test_create_capture() {
        let capture = MacOSScreenCapture::new();
        assert!(capture.is_ok());
    }
}
