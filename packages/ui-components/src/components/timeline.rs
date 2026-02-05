//! Timeline component for reviewing recordings
//!
//! Provides a visual timeline with playhead, clips, and time markers.

use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Program, Stroke, Text};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Theme};
use std::time::Duration;

/// Timeline state and configuration
#[derive(Debug, Clone)]
pub struct Timeline {
    /// Total duration of the recording
    total_duration: Duration,
    /// Current playhead position
    current_position: Duration,
    /// List of clips/segments in the timeline
    clips: Vec<Clip>,
    /// Whether the user is currently dragging the playhead
    #[allow(dead_code)] // Reserved for drag implementation
    dragging: bool,
    /// Timeline width in pixels (set during layout)
    width: f32,
    /// Pixels per second scale
    pixels_per_second: f32,
}

/// A clip/segment in the timeline
#[derive(Debug, Clone)]
pub struct Clip {
    /// Start time of the clip
    pub start: Duration,
    /// End time of the clip
    pub end: Duration,
    /// Clip color
    pub color: Color,
    /// Optional label
    pub label: Option<String>,
}

/// Messages that can be produced by the timeline
#[derive(Debug, Clone)]
pub enum TimelineMessage {
    /// Playhead was moved to a new position
    PositionChanged(Duration),
    /// User started dragging the playhead
    DragStarted,
    /// User stopped dragging the playhead
    DragEnded,
}

impl Timeline {
    /// Create a new timeline with the given duration
    pub fn new(total_duration: Duration) -> Self {
        Self {
            total_duration,
            current_position: Duration::ZERO,
            clips: Vec::new(),
            dragging: false,
            width: 800.0,            // Default width
            pixels_per_second: 10.0, // Default scale
        }
    }

    /// Set the current playhead position
    pub fn set_position(&mut self, position: Duration) {
        self.current_position = position.min(self.total_duration);
    }

    /// Get the current playhead position
    pub fn position(&self) -> Duration {
        self.current_position
    }

    /// Add a clip to the timeline
    pub fn add_clip(&mut self, clip: Clip) {
        self.clips.push(clip);
    }

    /// Clear all clips
    pub fn clear_clips(&mut self) {
        self.clips.clear();
    }

    /// Set the timeline width (called during layout)
    pub fn set_width(&mut self, width: f32) {
        self.width = width;
        // Recalculate scale based on width and duration
        let duration_secs = self.total_duration.as_secs_f32();
        if duration_secs > 0.0 {
            self.pixels_per_second = (width - 40.0) / duration_secs; // 40px padding
        }
    }

    /// Convert time to x position
    fn time_to_x(&self, time: Duration) -> f32 {
        20.0 + time.as_secs_f32() * self.pixels_per_second
    }

    /// Convert x position to time
    #[allow(dead_code)] // Reserved for drag-to-seek implementation
    fn x_to_time(&self, x: f32) -> Duration {
        let relative_x = (x - 20.0).max(0.0);
        let seconds = relative_x / self.pixels_per_second;
        Duration::from_secs_f32(seconds.min(self.total_duration.as_secs_f32()))
    }

    /// Build the timeline view
    pub fn view(&self) -> Element<'_, TimelineMessage> {
        Canvas::new(TimelineProgram { timeline: self })
            .width(Length::Fill)
            .height(Length::Fixed(120.0))
            .into()
    }
}

/// Canvas program for rendering the timeline
struct TimelineProgram<'a> {
    timeline: &'a Timeline,
}

impl<'a, Message> Program<Message> for TimelineProgram<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // Background
        frame.fill_rectangle(
            Point::new(0.0, 0.0),
            bounds.size(),
            Color::from_rgb(0.15, 0.15, 0.15),
        );

        // Draw timeline track background
        let track_y = 60.0;
        let track_height = 40.0;
        let start_x = 20.0;
        let end_x = bounds.width - 20.0;

        frame.fill_rectangle(
            Point::new(start_x, track_y),
            iced::Size::new(end_x - start_x, track_height),
            Color::from_rgb(0.2, 0.2, 0.2),
        );

        // Draw clips
        for clip in &self.timeline.clips {
            let clip_x = self.timeline.time_to_x(clip.start);
            let clip_width = self.timeline.time_to_x(clip.end) - clip_x;

            frame.fill_rectangle(
                Point::new(clip_x, track_y + 5.0),
                iced::Size::new(clip_width, track_height - 10.0),
                clip.color,
            );
        }

        // Draw time markers
        let marker_interval = self.calculate_marker_interval();
        let mut current_time = Duration::ZERO;
        while current_time <= self.timeline.total_duration {
            let x = self.timeline.time_to_x(current_time);

            // Marker line
            frame.stroke(
                &Path::line(
                    Point::new(x, track_y),
                    Point::new(x, track_y + track_height),
                ),
                Stroke::default()
                    .with_color(Color::from_rgb(0.4, 0.4, 0.4))
                    .with_width(1.0),
            );

            // Time label
            let label = format_time(current_time);
            let text = Text {
                content: label,
                position: Point::new(x, track_y - 10.0),
                color: Color::from_rgb(0.7, 0.7, 0.7),
                size: iced::Pixels(10.0),
                ..Text::default()
            };
            frame.fill_text(text);

            current_time += marker_interval;
        }

        // Draw playhead
        let playhead_x = self.timeline.time_to_x(self.timeline.current_position);
        frame.stroke(
            &Path::line(Point::new(playhead_x, 20.0), Point::new(playhead_x, 100.0)),
            Stroke::default()
                .with_color(Color::from_rgb(1.0, 0.3, 0.3))
                .with_width(2.0),
        );

        // Playhead handle (circle)
        let handle_radius = 6.0;
        let handle_center = Point::new(playhead_x, track_y + track_height / 2.0);
        let circle_path = Path::new(|p| {
            p.circle(handle_center, handle_radius);
        });
        frame.fill(&circle_path, Color::from_rgb(1.0, 0.3, 0.3));

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        match event {
            canvas::Event::Mouse(mouse_event) => match mouse_event {
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    if let Some(position) = cursor.position_in(bounds) {
                        // Check if clicking near playhead
                        let playhead_x = self.timeline.time_to_x(self.timeline.current_position);
                        if (position.x - playhead_x).abs() < 15.0 {
                            // We'll need to communicate this back to the Timeline somehow
                            // For now, just capture the event
                            return (canvas::event::Status::Captured, None);
                        }
                    }
                    (canvas::event::Status::Ignored, None)
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => {
                    (canvas::event::Status::Ignored, None)
                }
                mouse::Event::CursorMoved { .. } => (canvas::event::Status::Ignored, None),
                _ => (canvas::event::Status::Ignored, None),
            },
            _ => (canvas::event::Status::Ignored, None),
        }
    }
}

impl TimelineProgram<'_> {
    /// Calculate appropriate time interval for markers
    fn calculate_marker_interval(&self) -> Duration {
        let total_secs = self.timeline.total_duration.as_secs_f32();
        if total_secs <= 10.0 {
            Duration::from_secs(1)
        } else if total_secs <= 60.0 {
            Duration::from_secs(5)
        } else if total_secs <= 300.0 {
            Duration::from_secs(15)
        } else if total_secs <= 600.0 {
            Duration::from_secs(30)
        } else {
            Duration::from_secs(60)
        }
    }
}

/// Format duration as MM:SS or HH:MM:SS
fn format_time(duration: Duration) -> String {
    let secs = duration.as_secs();
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}
