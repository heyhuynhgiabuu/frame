//! Integrated effects pipeline
//!
//! Combines cursor tracking, zoom, keyboard display, and background compositing
//! into a single unified pipeline for video frame processing.

use crate::capture::Frame;
use crate::effects::background::BackgroundCompositor;
use crate::effects::cursor::CursorTracker;
use crate::effects::keyboard::KeyboardCapture;
use crate::effects::zoom::ZoomEffect;
use crate::effects::{
    EffectInput, EffectsConfig, EffectsPipeline, KeyboardBadge, KeyboardEvent, MouseEvent,
    ProcessedFrame,
};
use crate::FrameResult;
use std::time::Duration;

/// Integrated effects pipeline implementation
#[derive(Debug)]
pub struct IntegratedPipeline {
    config: EffectsConfig,
    cursor: CursorTracker,
    zoom: ZoomEffect,
    keyboard: KeyboardCapture,
    background: BackgroundCompositor,
    /// Current time (updated each frame)
    current_time: Duration,
    /// Frame dimensions (set from first frame)
    frame_size: Option<(u32, u32)>,
}

impl Default for IntegratedPipeline {
    fn default() -> Self {
        Self::new(EffectsConfig::default())
    }
}

impl IntegratedPipeline {
    /// Create a new integrated pipeline with the given config
    pub fn new(config: EffectsConfig) -> Self {
        let cursor = CursorTracker::new(Duration::from_millis(config.zoom.idle_timeout_ms as u64));
        let zoom = ZoomEffect::new(config.zoom.clone());
        let keyboard = KeyboardCapture::new();
        let background = BackgroundCompositor::new(config.background.clone());

        Self {
            config,
            cursor,
            zoom,
            keyboard,
            background,
            current_time: Duration::ZERO,
            frame_size: None,
        }
    }

    /// Process a mouse event
    pub fn process_mouse_event(&mut self, event: MouseEvent) {
        self.cursor.process_event(event, self.current_time);
    }

    /// Process a keyboard event
    pub fn process_keyboard_event(&mut self, event: KeyboardEvent) {
        self.keyboard.process_event(event);
    }

    /// Update the current time
    pub fn update_time(&mut self, time: Duration) {
        self.current_time = time;
        self.cursor.update_idle_state(time);
        self.zoom.update(&self.cursor, time);
    }

    /// Get the current keyboard combo for display
    pub fn current_keyboard_combo(&self) -> Option<String> {
        if self.config.keyboard.enabled {
            self.keyboard.current_combo(self.current_time)
        } else {
            None
        }
    }

    /// Get the current zoom state
    pub fn zoom_state(&self) -> crate::effects::zoom::ZoomState {
        self.zoom.state()
    }

    /// Check if zoom is currently active
    pub fn is_zoomed(&self) -> bool {
        self.zoom.is_active()
    }

    /// Apply zoom cropping to a frame
    fn apply_zoom(&self, frame: &Frame) -> FrameResult<Frame> {
        if !self.zoom.is_active() {
            return Ok(frame.clone());
        }

        let (src_x, src_y, src_w, src_h) = self.zoom.source_rect(frame.width, frame.height);

        // Create cropped frame (scaled back to original size)
        // For now, we just crop without scaling - scaling would need a proper
        // image scaling algorithm (bilinear, lanczos, etc.)
        let mut cropped_data = Vec::with_capacity((src_w * src_h * 4) as usize);

        for y in src_y..(src_y + src_h).min(frame.height) {
            let row_start = ((y * frame.width + src_x) * 4) as usize;
            let row_end = ((y * frame.width + src_x + src_w) * 4) as usize;
            if row_end <= frame.data.len() {
                cropped_data.extend_from_slice(&frame.data[row_start..row_end]);
            }
        }

        Ok(Frame {
            data: cropped_data,
            width: src_w,
            height: src_h,
            timestamp: frame.timestamp,
            format: frame.format,
        })
    }

    /// Scale a frame to target dimensions using nearest neighbor
    fn scale_frame(frame: &Frame, target_width: u32, target_height: u32) -> Frame {
        if frame.width == target_width && frame.height == target_height {
            return frame.clone();
        }

        let mut scaled_data = vec![0u8; (target_width * target_height * 4) as usize];

        let x_ratio = frame.width as f32 / target_width as f32;
        let y_ratio = frame.height as f32 / target_height as f32;

        for ty in 0..target_height {
            for tx in 0..target_width {
                let sx = (tx as f32 * x_ratio) as u32;
                let sy = (ty as f32 * y_ratio) as u32;

                let src_idx = ((sy * frame.width + sx) * 4) as usize;
                let dst_idx = ((ty * target_width + tx) * 4) as usize;

                if src_idx + 3 < frame.data.len() && dst_idx + 3 < scaled_data.len() {
                    scaled_data[dst_idx..dst_idx + 4]
                        .copy_from_slice(&frame.data[src_idx..src_idx + 4]);
                }
            }
        }

        Frame {
            data: scaled_data,
            width: target_width,
            height: target_height,
            timestamp: frame.timestamp,
            format: frame.format,
        }
    }

    /// Calculate keyboard badge opacity based on time since last keypress
    fn keyboard_opacity(&self) -> f32 {
        let fade_duration = Duration::from_millis(self.config.keyboard.fade_out_duration_ms as u64);

        // Get time of last key release
        if let Some(combo) = self.keyboard.current_combo(self.current_time) {
            if !combo.is_empty() {
                return 1.0; // Still showing combo
            }
        }

        // Fading out - check last event time
        let buffer = self.keyboard.buffer();
        if let Some(last_event) = buffer.events_since(Duration::ZERO).last() {
            let elapsed = self.current_time.saturating_sub(last_event.timestamp);
            if elapsed < fade_duration {
                return 1.0 - (elapsed.as_secs_f32() / fade_duration.as_secs_f32());
            }
        }

        0.0
    }
}

impl EffectsPipeline for IntegratedPipeline {
    fn config(&self) -> &EffectsConfig {
        &self.config
    }

    fn set_config(&mut self, config: EffectsConfig) {
        // Update sub-components
        self.cursor = CursorTracker::new(Duration::from_millis(config.zoom.idle_timeout_ms as u64));
        self.zoom.set_config(config.zoom.clone());
        self.keyboard.set_enabled(config.keyboard.enabled);
        self.background.set_config(config.background.clone());
        self.config = config;
    }

    fn process_input(&mut self, input: EffectInput) {
        match input {
            EffectInput::Cursor(pos) => {
                self.cursor.update_position(pos.x, pos.y, pos.timestamp);
            }
            EffectInput::Mouse(event) => self.process_mouse_event(event),
            EffectInput::Keyboard(event) => self.process_keyboard_event(event),
        }
    }

    fn process_frame(&mut self, frame: Frame) -> FrameResult<ProcessedFrame> {
        // Update frame size on first frame
        if self.frame_size.is_none() {
            self.frame_size = Some((frame.width, frame.height));
            self.zoom.set_frame_size(frame.width, frame.height);
        }

        let original_size = (frame.width, frame.height);

        // Step 1: Apply zoom (crop to zoomed region)
        let zoomed_frame = self.apply_zoom(&frame)?;

        // Step 2: Scale back to original size if zoomed
        let scaled_frame = if self.zoom.is_active() {
            Self::scale_frame(&zoomed_frame, original_size.0, original_size.1)
        } else {
            zoomed_frame
        };

        // Step 3: Apply background compositing (padding, corner radius)
        let composited_frame = self.background.composite(&scaled_frame)?;

        // Step 4: Generate keyboard badges if needed
        let keyboard_badges = if let Some(combo) = self.current_keyboard_combo() {
            vec![KeyboardBadge {
                text: combo,
                position: self.config.keyboard.position,
                opacity: self.keyboard_opacity(),
            }]
        } else {
            vec![]
        };

        Ok(ProcessedFrame {
            frame: composited_frame,
            keyboard_badges,
        })
    }

    fn reset(&mut self) {
        self.cursor.reset();
        self.zoom.reset();
        self.keyboard.reset();
        self.background.reset();
        self.current_time = Duration::ZERO;
        self.frame_size = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::PixelFormat;
    use crate::effects::{MouseButton, ZoomConfig};

    fn test_frame(width: u32, height: u32) -> Frame {
        Frame {
            data: vec![128u8; (width * height * 4) as usize],
            width,
            height,
            timestamp: Duration::from_secs(0),
            format: PixelFormat::Rgba,
        }
    }

    #[test]
    fn test_pipeline_creation() {
        let pipeline = IntegratedPipeline::default();
        assert!(!pipeline.is_zoomed());
        assert!(pipeline.current_keyboard_combo().is_none());
    }

    #[test]
    fn test_pipeline_process_frame() {
        let mut pipeline = IntegratedPipeline::default();
        let frame = test_frame(1920, 1080);

        let result = pipeline.process_frame(frame).unwrap();

        // With default config (no padding), size should match
        // But background might add padding, so check it's reasonable
        assert!(result.frame.width >= 1920);
        assert!(result.frame.height >= 1080);
        assert!(result.keyboard_badges.is_empty()); // No keys pressed
    }

    #[test]
    fn test_pipeline_zoom_on_click() {
        let mut config = EffectsConfig::default();
        config.zoom = ZoomConfig {
            enabled: true,
            max_zoom: 1.5,
            transition_duration_ms: 100,
            idle_timeout_ms: 2000,
            easing: crate::effects::EasingFunction::Linear,
        };

        let mut pipeline = IntegratedPipeline::new(config);
        let frame = test_frame(1920, 1080);

        // Process first frame to set size
        pipeline.process_frame(frame.clone()).unwrap();

        // Click to trigger zoom
        pipeline.process_input(EffectInput::Mouse(MouseEvent::Click {
            x: 960.0,
            y: 540.0,
            button: MouseButton::Left,
        }));
        pipeline.update_time(Duration::from_millis(0));

        // Wait for transition
        pipeline.update_time(Duration::from_millis(150));

        assert!(pipeline.is_zoomed());
    }

    #[test]
    fn test_pipeline_keyboard_combo() {
        let mut config = EffectsConfig::default();
        config.keyboard.enabled = true;

        let mut pipeline = IntegratedPipeline::new(config);

        // Press Cmd+S
        use crate::effects::{Key, Modifiers};
        pipeline.process_input(EffectInput::Keyboard(KeyboardEvent {
            key: Key::Character('s'),
            modifiers: Modifiers {
                command: true,
                ..Default::default()
            },
            timestamp: Duration::from_millis(0),
            pressed: true,
        }));

        pipeline.update_time(Duration::from_millis(100));

        let combo = pipeline.current_keyboard_combo();
        assert!(combo.is_some());
        assert!(combo.unwrap().contains("S"));
    }

    #[test]
    fn test_pipeline_reset() {
        let mut pipeline = IntegratedPipeline::default();
        let frame = test_frame(100, 100);

        pipeline.process_frame(frame).unwrap();
        pipeline.update_time(Duration::from_secs(10));

        pipeline.reset();

        assert!(!pipeline.is_zoomed());
        assert_eq!(pipeline.current_time, Duration::ZERO);
    }
}
