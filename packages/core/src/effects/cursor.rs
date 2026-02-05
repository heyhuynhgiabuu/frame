//! Cursor position tracking for zoom effects
//!
//! Tracks cursor position, velocity, and click events to drive automatic zoom.

use crate::effects::{CursorPosition, MouseEvent};
use std::time::Duration;

/// Velocity damping factor for smooth cursor following
const VELOCITY_DAMPING: f32 = 0.85;

/// Minimum velocity to consider cursor "moving" (pixels per second)
const MIN_VELOCITY_THRESHOLD: f32 = 10.0;

/// Track cursor position, velocity, and mouse events
#[derive(Debug)]
pub struct CursorTracker {
    /// Current cursor position
    position: CursorPosition,
    /// Previous position for velocity calculation
    prev_position: CursorPosition,
    /// Current velocity (pixels per second)
    velocity: (f32, f32),
    /// Time of last click (for zoom triggering)
    last_click_time: Option<Duration>,
    /// Position of last click
    last_click_position: Option<(f32, f32)>,
    /// Time of last movement
    last_move_time: Duration,
    /// Whether cursor is considered idle
    is_idle: bool,
    /// Idle timeout duration
    idle_timeout: Duration,
}

impl Default for CursorTracker {
    fn default() -> Self {
        Self::new(Duration::from_secs(2))
    }
}

impl CursorTracker {
    /// Create a new cursor tracker with specified idle timeout
    pub fn new(idle_timeout: Duration) -> Self {
        Self {
            position: CursorPosition::default(),
            prev_position: CursorPosition::default(),
            velocity: (0.0, 0.0),
            last_click_time: None,
            last_click_position: None,
            last_move_time: Duration::ZERO,
            is_idle: true,
            idle_timeout,
        }
    }

    /// Update cursor position
    pub fn update_position(&mut self, x: f32, y: f32, timestamp: Duration) {
        self.prev_position = self.position;
        self.position = CursorPosition::new(x, y, timestamp);

        // Calculate velocity
        let dt = timestamp
            .saturating_sub(self.prev_position.timestamp)
            .as_secs_f32();

        if dt > 0.0 {
            let dx = x - self.prev_position.x;
            let dy = y - self.prev_position.y;

            // Apply damping for smooth velocity
            self.velocity.0 =
                self.velocity.0 * VELOCITY_DAMPING + (dx / dt) * (1.0 - VELOCITY_DAMPING);
            self.velocity.1 =
                self.velocity.1 * VELOCITY_DAMPING + (dy / dt) * (1.0 - VELOCITY_DAMPING);
        }

        // Update movement time if velocity is above threshold
        if self.speed() > MIN_VELOCITY_THRESHOLD {
            self.last_move_time = timestamp;
            self.is_idle = false;
        }
    }

    /// Process a mouse event
    pub fn process_event(&mut self, event: MouseEvent, timestamp: Duration) {
        match event {
            MouseEvent::Click { x, y, button: _ } => {
                self.last_click_time = Some(timestamp);
                self.last_click_position = Some((x, y));
                self.update_position(x, y, timestamp);
            }
            MouseEvent::Move { x, y } => {
                self.update_position(x, y, timestamp);
            }
            MouseEvent::Scroll { x, y, delta_y: _ } => {
                // Scroll doesn't update position but marks activity
                self.last_move_time = timestamp;
                self.is_idle = false;
                // Optionally update position to scroll location
                self.update_position(x, y, timestamp);
            }
        }
    }

    /// Check and update idle state based on current time
    pub fn update_idle_state(&mut self, current_time: Duration) {
        if current_time.saturating_sub(self.last_move_time) > self.idle_timeout {
            self.is_idle = true;
            // Decay velocity when idle
            self.velocity.0 *= 0.5;
            self.velocity.1 *= 0.5;
        }
    }

    /// Get current cursor position
    pub fn position(&self) -> CursorPosition {
        self.position
    }

    /// Get current velocity (pixels per second)
    pub fn velocity(&self) -> (f32, f32) {
        self.velocity
    }

    /// Get cursor speed (magnitude of velocity)
    pub fn speed(&self) -> f32 {
        (self.velocity.0.powi(2) + self.velocity.1.powi(2)).sqrt()
    }

    /// Check if cursor is idle (hasn't moved for idle_timeout duration)
    pub fn is_idle(&self) -> bool {
        self.is_idle
    }

    /// Check if a click recently occurred
    pub fn recent_click(&self, current_time: Duration, within: Duration) -> Option<(f32, f32)> {
        if let (Some(click_time), Some(pos)) = (self.last_click_time, self.last_click_position) {
            if current_time.saturating_sub(click_time) <= within {
                return Some(pos);
            }
        }
        None
    }

    /// Interpolate position between frames for smooth rendering
    ///
    /// `alpha` is the interpolation factor (0.0 = previous frame, 1.0 = current frame)
    pub fn interpolated_position(&self, alpha: f32) -> (f32, f32) {
        let alpha = alpha.clamp(0.0, 1.0);
        let x = self.prev_position.x + (self.position.x - self.prev_position.x) * alpha;
        let y = self.prev_position.y + (self.position.y - self.prev_position.y) * alpha;
        (x, y)
    }

    /// Reset tracker state
    pub fn reset(&mut self) {
        self.position = CursorPosition::default();
        self.prev_position = CursorPosition::default();
        self.velocity = (0.0, 0.0);
        self.last_click_time = None;
        self.last_click_position = None;
        self.last_move_time = Duration::ZERO;
        self.is_idle = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::MouseButton;

    #[test]
    fn test_position_tracking() {
        let mut tracker = CursorTracker::default();

        tracker.update_position(100.0, 200.0, Duration::from_millis(0));
        assert_eq!(tracker.position().x, 100.0);
        assert_eq!(tracker.position().y, 200.0);

        tracker.update_position(150.0, 250.0, Duration::from_millis(16));
        assert_eq!(tracker.position().x, 150.0);
        assert_eq!(tracker.position().y, 250.0);
    }

    #[test]
    fn test_velocity_calculation() {
        let mut tracker = CursorTracker::default();

        // Initial position
        tracker.update_position(0.0, 0.0, Duration::from_millis(0));

        // Move 100 pixels in 100ms = 1000 pixels/sec
        tracker.update_position(100.0, 0.0, Duration::from_millis(100));

        // Velocity should be positive (with damping, won't be exactly 1000)
        assert!(tracker.velocity().0 > 0.0);
        assert!(tracker.speed() > MIN_VELOCITY_THRESHOLD);
    }

    #[test]
    fn test_idle_detection() {
        let mut tracker = CursorTracker::new(Duration::from_millis(100));

        // Move cursor
        tracker.update_position(100.0, 100.0, Duration::from_millis(0));
        tracker.update_position(150.0, 150.0, Duration::from_millis(16));
        assert!(!tracker.is_idle());

        // Wait past idle timeout
        tracker.update_idle_state(Duration::from_millis(200));
        assert!(tracker.is_idle());
    }

    #[test]
    fn test_click_tracking() {
        let mut tracker = CursorTracker::default();

        // Record a click
        tracker.process_event(
            MouseEvent::Click {
                x: 100.0,
                y: 200.0,
                button: MouseButton::Left,
            },
            Duration::from_millis(0),
        );

        // Check recent click within window
        assert_eq!(
            tracker.recent_click(Duration::from_millis(50), Duration::from_millis(100)),
            Some((100.0, 200.0))
        );

        // Check click is no longer recent
        assert_eq!(
            tracker.recent_click(Duration::from_millis(200), Duration::from_millis(100)),
            None
        );
    }

    #[test]
    fn test_interpolation() {
        let mut tracker = CursorTracker::default();

        tracker.update_position(0.0, 0.0, Duration::from_millis(0));
        tracker.update_position(100.0, 100.0, Duration::from_millis(16));

        // Halfway interpolation
        let (x, y) = tracker.interpolated_position(0.5);
        assert!((x - 50.0).abs() < 0.01);
        assert!((y - 50.0).abs() < 0.01);
    }
}
