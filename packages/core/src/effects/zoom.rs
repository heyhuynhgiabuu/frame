//! Zoom effect implementation
//!
//! Smooth zoom that follows cursor activity with Bézier easing.

use crate::effects::cursor::CursorTracker;
use crate::effects::ZoomConfig;
use std::time::Duration;

/// Current zoom state
#[derive(Debug, Clone, Copy)]
pub struct ZoomState {
    /// Current zoom level (1.0 = no zoom)
    pub level: f32,
    /// Center point of zoom (normalized 0-1)
    pub center: (f32, f32),
}

impl Default for ZoomState {
    fn default() -> Self {
        Self {
            level: 1.0,
            center: (0.5, 0.5),
        }
    }
}

/// Zoom effect processor
#[derive(Debug)]
pub struct ZoomEffect {
    config: ZoomConfig,
    /// Current zoom state
    current: ZoomState,
    /// Target zoom state we're transitioning to
    target: ZoomState,
    /// Time when current transition started
    transition_start: Option<Duration>,
    /// State at transition start (for interpolation)
    transition_from: ZoomState,
    /// Frame dimensions for coordinate conversion
    frame_size: (u32, u32),
}

impl ZoomEffect {
    pub fn new(config: ZoomConfig) -> Self {
        Self {
            config,
            current: ZoomState::default(),
            target: ZoomState::default(),
            transition_start: None,
            transition_from: ZoomState::default(),
            frame_size: (1920, 1080),
        }
    }

    /// Set frame dimensions
    pub fn set_frame_size(&mut self, width: u32, height: u32) {
        self.frame_size = (width, height);
    }

    /// Update zoom state based on cursor tracker
    pub fn update(&mut self, cursor: &CursorTracker, current_time: Duration) {
        if !self.config.enabled {
            self.current = ZoomState::default();
            self.target = ZoomState::default();
            return;
        }

        // Check for recent click - trigger zoom
        let click_window = Duration::from_millis(100);
        if let Some((click_x, click_y)) = cursor.recent_click(current_time, click_window) {
            self.trigger_zoom_to(click_x, click_y, current_time);
        } else if cursor.is_idle() {
            // Zoom out when idle
            self.trigger_zoom_out(current_time);
        } else {
            // Follow cursor smoothly while moving
            let pos = cursor.position();
            self.update_center_smooth(pos.x, pos.y);
        }

        // Update current state based on transition
        self.update_transition(current_time);
    }

    /// Trigger zoom to a specific point
    fn trigger_zoom_to(&mut self, x: f32, y: f32, current_time: Duration) {
        // Convert pixel coordinates to normalized (0-1)
        let center_x = (x / self.frame_size.0 as f32).clamp(0.0, 1.0);
        let center_y = (y / self.frame_size.1 as f32).clamp(0.0, 1.0);

        // Only start new transition if target is different
        if (self.target.level - self.config.max_zoom).abs() > 0.01
            || (self.target.center.0 - center_x).abs() > 0.01
            || (self.target.center.1 - center_y).abs() > 0.01
        {
            self.target = ZoomState {
                level: self.config.max_zoom,
                center: (center_x, center_y),
            };
            self.transition_from = self.current;
            self.transition_start = Some(current_time);
        }
    }

    /// Trigger zoom out to default state
    fn trigger_zoom_out(&mut self, current_time: Duration) {
        if (self.target.level - 1.0).abs() > 0.01 {
            self.target = ZoomState::default();
            self.transition_from = self.current;
            self.transition_start = Some(current_time);
        }
    }

    /// Smoothly update center while cursor is moving (without changing zoom level)
    fn update_center_smooth(&mut self, x: f32, y: f32) {
        if self.current.level > 1.01 {
            // Only follow when zoomed in
            let center_x = (x / self.frame_size.0 as f32).clamp(0.0, 1.0);
            let center_y = (y / self.frame_size.1 as f32).clamp(0.0, 1.0);

            // Smooth follow with simple lerp
            const FOLLOW_SPEED: f32 = 0.1;
            self.current.center.0 += (center_x - self.current.center.0) * FOLLOW_SPEED;
            self.current.center.1 += (center_y - self.current.center.1) * FOLLOW_SPEED;
            self.target.center = self.current.center;
        }
    }

    /// Update current state based on ongoing transition
    fn update_transition(&mut self, current_time: Duration) {
        if let Some(start_time) = self.transition_start {
            let elapsed = current_time.saturating_sub(start_time);
            let duration = Duration::from_millis(self.config.transition_duration_ms as u64);

            if elapsed >= duration {
                // Transition complete
                self.current = self.target;
                self.transition_start = None;
            } else {
                // Interpolate
                let t = elapsed.as_secs_f32() / duration.as_secs_f32();
                let eased_t = self.config.easing.apply(t);

                self.current.level = self.transition_from.level
                    + (self.target.level - self.transition_from.level) * eased_t;
                self.current.center.0 = self.transition_from.center.0
                    + (self.target.center.0 - self.transition_from.center.0) * eased_t;
                self.current.center.1 = self.transition_from.center.1
                    + (self.target.center.1 - self.transition_from.center.1) * eased_t;
            }
        }
    }

    /// Get current zoom state
    pub fn state(&self) -> ZoomState {
        self.current
    }

    /// Check if zoom is active (level > 1)
    pub fn is_active(&self) -> bool {
        self.current.level > 1.01
    }

    /// Get the visible rectangle in normalized coordinates (0-1)
    /// Returns (x, y, width, height)
    pub fn visible_rect(&self) -> (f32, f32, f32, f32) {
        let visible_size = 1.0 / self.current.level;
        let half_size = visible_size / 2.0;

        // Center the visible area around the zoom center, but clamp to bounds
        let x = (self.current.center.0 - half_size).clamp(0.0, 1.0 - visible_size);
        let y = (self.current.center.1 - half_size).clamp(0.0, 1.0 - visible_size);

        (x, y, visible_size, visible_size)
    }

    /// Get the source rectangle in pixel coordinates for a given frame size
    pub fn source_rect(&self, width: u32, height: u32) -> (u32, u32, u32, u32) {
        let (norm_x, norm_y, norm_w, norm_h) = self.visible_rect();

        let x = (norm_x * width as f32) as u32;
        let y = (norm_y * height as f32) as u32;
        let w = (norm_w * width as f32) as u32;
        let h = (norm_h * height as f32) as u32;

        (x, y, w.max(1), h.max(1))
    }

    /// Update configuration
    pub fn set_config(&mut self, config: ZoomConfig) {
        self.config = config;
    }

    /// Get configuration
    pub fn config(&self) -> &ZoomConfig {
        &self.config
    }

    /// Reset to default state
    pub fn reset(&mut self) {
        self.current = ZoomState::default();
        self.target = ZoomState::default();
        self.transition_start = None;
        self.transition_from = ZoomState::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EasingFunction;

    fn default_config() -> ZoomConfig {
        ZoomConfig {
            enabled: true,
            max_zoom: 1.5,
            transition_duration_ms: 300,
            idle_timeout_ms: 2000,
            easing: EasingFunction::EaseInOutCubic,
        }
    }

    #[test]
    fn test_initial_state() {
        let zoom = ZoomEffect::new(default_config());
        assert_eq!(zoom.state().level, 1.0);
        assert!(!zoom.is_active());
    }

    #[test]
    fn test_zoom_in_on_click() {
        let mut zoom = ZoomEffect::new(default_config());
        zoom.set_frame_size(1920, 1080);

        let mut cursor = CursorTracker::default();
        cursor.process_event(
            crate::effects::MouseEvent::Click {
                x: 960.0,
                y: 540.0,
                button: crate::effects::MouseButton::Left,
            },
            Duration::from_millis(0),
        );

        zoom.update(&cursor, Duration::from_millis(0));

        // After transition completes
        zoom.update(&cursor, Duration::from_millis(350));

        assert!(zoom.state().level > 1.0);
        assert!(zoom.is_active());
    }

    #[test]
    fn test_zoom_out_on_idle() {
        let mut zoom = ZoomEffect::new(default_config());
        zoom.set_frame_size(1920, 1080);

        // Start zoomed in
        zoom.current.level = 1.5;
        zoom.target.level = 1.5;

        let mut cursor = CursorTracker::new(Duration::from_millis(100));
        cursor.update_position(500.0, 500.0, Duration::from_millis(0));
        cursor.update_idle_state(Duration::from_millis(200)); // Trigger idle

        zoom.update(&cursor, Duration::from_millis(200));

        // After transition completes
        zoom.update(&cursor, Duration::from_millis(600));

        assert!((zoom.state().level - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_visible_rect() {
        let mut zoom = ZoomEffect::new(default_config());
        zoom.current.level = 2.0;
        zoom.current.center = (0.5, 0.5);

        let (x, y, w, h) = zoom.visible_rect();

        // At 2x zoom, visible area is 50% of frame
        assert!((w - 0.5).abs() < 0.01);
        assert!((h - 0.5).abs() < 0.01);
        // Centered
        assert!((x - 0.25).abs() < 0.01);
        assert!((y - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_disabled_zoom() {
        let mut config = default_config();
        config.enabled = false;

        let mut zoom = ZoomEffect::new(config);
        zoom.set_frame_size(1920, 1080);

        let mut cursor = CursorTracker::default();
        cursor.process_event(
            crate::effects::MouseEvent::Click {
                x: 960.0,
                y: 540.0,
                button: crate::effects::MouseButton::Left,
            },
            Duration::from_millis(0),
        );

        zoom.update(&cursor, Duration::from_millis(0));
        zoom.update(&cursor, Duration::from_millis(500));

        // Should stay at 1.0 when disabled
        assert_eq!(zoom.state().level, 1.0);
    }
}
