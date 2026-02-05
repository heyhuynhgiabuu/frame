//! Timeline component for reviewing recordings
//!
//! Provides a visual timeline with playhead, clips, and time markers.
//! Supports edit operations: selection, trim, cut, and split.

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
    #[allow(dead_code)] // Reserved for playhead drag implementation
    dragging_playhead: bool,
    /// Which trim handle is being dragged (if any)
    dragging_trim: Option<TrimHandleDrag>,
    /// Timeline width in pixels (set during layout)
    width: f32,
    /// Pixels per second scale
    pixels_per_second: f32,
    /// Selection state for edit operations
    selection: SelectionState,
}

/// Which trim handle is being dragged
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrimHandleDrag {
    /// Dragging the start (left) trim handle
    Start,
    /// Dragging the end (right) trim handle
    End,
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

/// Selection state for timeline editing
#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    /// In point (start of selection) - set with 'I' key
    pub in_point: Option<Duration>,
    /// Out point (end of selection) - set with 'O' key
    pub out_point: Option<Duration>,
    /// List of split points in the timeline
    pub split_points: Vec<Duration>,
    /// Cut regions (removed sections) - represented as (from, to) pairs
    pub cut_regions: Vec<(Duration, Duration)>,
    /// Trim boundaries for the recording
    pub trim: Option<TrimBounds>,
}

/// Trim boundaries for a recording
#[derive(Debug, Clone, Copy)]
pub struct TrimBounds {
    /// Content before this time is trimmed away
    pub start: Duration,
    /// Content after this time is trimmed away
    pub end: Duration,
}

impl SelectionState {
    /// Create a new selection state
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the currently selected range (if both in and out points are set)
    pub fn selected_range(&self) -> Option<(Duration, Duration)> {
        match (self.in_point, self.out_point) {
            (Some(i), Some(o)) if i < o => Some((i, o)),
            (Some(i), Some(o)) if o < i => Some((o, i)), // Swap if in reverse order
            _ => None,
        }
    }

    /// Set the in point at the given time
    pub fn set_in_point(&mut self, time: Duration) {
        self.in_point = Some(time);
    }

    /// Set the out point at the given time
    pub fn set_out_point(&mut self, time: Duration) {
        self.out_point = Some(time);
    }

    /// Clear the current selection (in/out points)
    pub fn clear_selection(&mut self) {
        self.in_point = None;
        self.out_point = None;
    }

    /// Add a split point at the given time
    pub fn add_split(&mut self, time: Duration) {
        if !self.split_points.contains(&time) {
            self.split_points.push(time);
            self.split_points.sort();
        }
    }

    /// Add a cut region
    pub fn add_cut(&mut self, from: Duration, to: Duration) {
        // Normalize order
        let (from, to) = if from <= to { (from, to) } else { (to, from) };
        self.cut_regions.push((from, to));
        // Sort by start time
        self.cut_regions.sort_by_key(|(f, _)| *f);
    }

    /// Set trim boundaries
    pub fn set_trim(&mut self, start: Duration, end: Duration) {
        self.trim = Some(TrimBounds { start, end });
    }

    /// Clear trim boundaries
    pub fn clear_trim(&mut self) {
        self.trim = None;
    }

    /// Check if a time point is within a cut region
    pub fn is_cut(&self, time: Duration) -> bool {
        self.cut_regions
            .iter()
            .any(|(from, to)| time >= *from && time <= *to)
    }

    /// Check if a time point is within the trim region (if set)
    pub fn is_trimmed(&self, time: Duration) -> bool {
        if let Some(trim) = self.trim {
            time < trim.start || time > trim.end
        } else {
            false
        }
    }

    /// Clear all selection state
    pub fn clear_all(&mut self) {
        *self = Self::default();
    }
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
    /// In point was set (for edit selection)
    InPointSet(Duration),
    /// Out point was set (for edit selection)
    OutPointSet(Duration),
    /// Selection was cleared
    SelectionCleared,
    /// Split point was added
    SplitAdded(Duration),
    /// Cut was performed on selected region
    CutPerformed(Duration, Duration),
    /// Trim was applied
    TrimApplied(Duration, Duration),
    /// Trim handle drag started
    TrimDragStarted(TrimHandleDrag),
    /// Trim handle was dragged to a new position
    TrimDragged(TrimHandleDrag, Duration),
    /// Trim handle drag ended
    TrimDragEnded,
}

impl Timeline {
    /// Create a new timeline with the given duration
    pub fn new(total_duration: Duration) -> Self {
        Self {
            total_duration,
            current_position: Duration::ZERO,
            clips: Vec::new(),
            dragging_playhead: false,
            dragging_trim: None,
            width: 800.0,            // Default width
            pixels_per_second: 10.0, // Default scale
            selection: SelectionState::default(),
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

    /// Get the selection state (immutable)
    pub fn selection(&self) -> &SelectionState {
        &self.selection
    }

    /// Get the selection state (mutable)
    pub fn selection_mut(&mut self) -> &mut SelectionState {
        &mut self.selection
    }

    /// Set the in point at the current playhead position
    pub fn set_in_point(&mut self) {
        self.selection.set_in_point(self.current_position);
    }

    /// Set the out point at the current playhead position
    pub fn set_out_point(&mut self) {
        self.selection.set_out_point(self.current_position);
    }

    /// Split at the current playhead position
    pub fn split_at_playhead(&mut self) {
        self.selection.add_split(self.current_position);
    }

    /// Cut the selected region (requires in and out points)
    pub fn cut_selection(&mut self) -> Option<(Duration, Duration)> {
        if let Some((from, to)) = self.selection.selected_range() {
            self.selection.add_cut(from, to);
            self.selection.clear_selection();
            Some((from, to))
        } else {
            None
        }
    }

    /// Apply trim using the selected region or in/out points
    pub fn apply_trim(&mut self) -> Option<(Duration, Duration)> {
        if let Some((start, end)) = self.selection.selected_range() {
            self.selection.set_trim(start, end);
            self.selection.clear_selection();
            Some((start, end))
        } else if let (Some(in_point), None) = (self.selection.in_point, self.selection.out_point) {
            // Only in point set - trim from in point to end
            self.selection.set_trim(in_point, self.total_duration);
            Some((in_point, self.total_duration))
        } else if let (None, Some(out_point)) = (self.selection.in_point, self.selection.out_point)
        {
            // Only out point set - trim from start to out point
            self.selection.set_trim(Duration::ZERO, out_point);
            Some((Duration::ZERO, out_point))
        } else {
            None
        }
    }

    /// Clear all edit selections
    pub fn clear_edit_state(&mut self) {
        self.selection.clear_all();
    }

    /// Start dragging a trim handle
    #[allow(dead_code)] // Public API for external integration
    pub fn start_trim_drag(&mut self, handle: TrimHandleDrag) {
        self.dragging_trim = Some(handle);
        // Ensure trim bounds exist - if not, create default (full duration)
        if self.selection.trim.is_none() {
            self.selection.trim = Some(TrimBounds {
                start: Duration::ZERO,
                end: self.total_duration,
            });
        }
    }

    /// Update trim position while dragging
    #[allow(dead_code)] // Public API for external integration
    pub fn update_trim_drag(&mut self, time: Duration) {
        if let Some(handle) = self.dragging_trim {
            if let Some(ref mut trim) = self.selection.trim {
                // Enforce minimum trim duration of 500ms
                const MIN_TRIM_DURATION: Duration = Duration::from_millis(500);

                match handle {
                    TrimHandleDrag::Start => {
                        // Don't let start go past end - MIN_TRIM_DURATION
                        let max_start = trim.end.saturating_sub(MIN_TRIM_DURATION);
                        trim.start = time.min(max_start).min(self.total_duration);
                    }
                    TrimHandleDrag::End => {
                        // Don't let end go before start + MIN_TRIM_DURATION
                        let min_end = trim.start + MIN_TRIM_DURATION;
                        trim.end = time.max(min_end).min(self.total_duration);
                    }
                }
            }
        }
    }

    /// End trim handle drag
    #[allow(dead_code)] // Public API for external integration
    pub fn end_trim_drag(&mut self) {
        self.dragging_trim = None;
    }

    /// Check if currently dragging a trim handle
    #[allow(dead_code)] // Public API for external integration
    pub fn is_dragging_trim(&self) -> bool {
        self.dragging_trim.is_some()
    }

    /// Get which trim handle is being dragged
    #[allow(dead_code)] // Public API for external integration
    pub fn dragging_trim_handle(&self) -> Option<TrimHandleDrag> {
        self.dragging_trim
    }

    /// Check if a point is near a trim handle (for hit testing)
    #[allow(dead_code)] // Used by canvas interaction (to be fully wired)
    fn is_near_trim_handle(
        &self,
        x: f32,
        track_y: f32,
        track_height: f32,
    ) -> Option<TrimHandleDrag> {
        if let Some(trim) = &self.selection.trim {
            let handle_hit_width = 12.0;

            let start_x = self.time_to_x(trim.start);
            let end_x = self.time_to_x(trim.end);

            // Check start handle
            if (x - start_x).abs() < handle_hit_width {
                return Some(TrimHandleDrag::Start);
            }

            // Check end handle
            if (x - end_x).abs() < handle_hit_width {
                return Some(TrimHandleDrag::End);
            }
        }

        // Suppress unused variable warning
        let _ = (track_y, track_height);

        None
    }

    /// Convert time to x position
    fn time_to_x(&self, time: Duration) -> f32 {
        20.0 + time.as_secs_f32() * self.pixels_per_second
    }

    /// Convert x position to time
    #[allow(dead_code)] // Used by canvas interaction (to be fully wired)
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

        // Constants for layout
        let track_y = 60.0;
        let track_height = 40.0;
        let start_x = 20.0;
        let end_x = bounds.width - 20.0;

        // Colors
        let bg_color = Color::from_rgb(0.15, 0.15, 0.15);
        let track_color = Color::from_rgb(0.2, 0.2, 0.2);
        let trimmed_color = Color::from_rgba(0.1, 0.1, 0.1, 0.8);
        let cut_color = Color::from_rgba(0.3, 0.1, 0.1, 0.7);
        let selection_color = Color::from_rgba(0.3, 0.5, 0.8, 0.3);
        let in_out_marker_color = Color::from_rgb(0.3, 0.7, 1.0);
        let split_color = Color::from_rgb(1.0, 0.8, 0.2);
        let trim_handle_color = Color::from_rgb(0.9, 0.9, 0.9);
        let playhead_color = Color::from_rgb(1.0, 0.3, 0.3);
        let marker_color = Color::from_rgb(0.4, 0.4, 0.4);
        let label_color = Color::from_rgb(0.7, 0.7, 0.7);

        // Background
        frame.fill_rectangle(Point::new(0.0, 0.0), bounds.size(), bg_color);

        // Draw timeline track background
        frame.fill_rectangle(
            Point::new(start_x, track_y),
            iced::Size::new(end_x - start_x, track_height),
            track_color,
        );

        // Draw trimmed regions (grayed out)
        if let Some(trim) = &self.timeline.selection.trim {
            // Region before trim start (trimmed away)
            if trim.start > Duration::ZERO {
                let trim_x = self.timeline.time_to_x(trim.start);
                frame.fill_rectangle(
                    Point::new(start_x, track_y),
                    iced::Size::new(trim_x - start_x, track_height),
                    trimmed_color,
                );
            }

            // Region after trim end (trimmed away)
            if trim.end < self.timeline.total_duration {
                let trim_x = self.timeline.time_to_x(trim.end);
                frame.fill_rectangle(
                    Point::new(trim_x, track_y),
                    iced::Size::new(end_x - trim_x, track_height),
                    trimmed_color,
                );
            }

            // Draw trim handles
            self.draw_trim_handle(
                &mut frame,
                trim.start,
                track_y,
                track_height,
                trim_handle_color,
            );
            self.draw_trim_handle(
                &mut frame,
                trim.end,
                track_y,
                track_height,
                trim_handle_color,
            );
        }

        // Draw cut regions (with X pattern overlay)
        for (cut_from, cut_to) in &self.timeline.selection.cut_regions {
            let from_x = self.timeline.time_to_x(*cut_from);
            let to_x = self.timeline.time_to_x(*cut_to);
            let cut_width = to_x - from_x;

            // Fill with cut color
            frame.fill_rectangle(
                Point::new(from_x, track_y),
                iced::Size::new(cut_width, track_height),
                cut_color,
            );

            // Draw X pattern overlay
            self.draw_cut_pattern(&mut frame, from_x, to_x, track_y, track_height);
        }

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

        // Draw selection region (if in and out points are set)
        if let Some((in_point, out_point)) = self.timeline.selection.selected_range() {
            let in_x = self.timeline.time_to_x(in_point);
            let out_x = self.timeline.time_to_x(out_point);

            frame.fill_rectangle(
                Point::new(in_x, track_y),
                iced::Size::new(out_x - in_x, track_height),
                selection_color,
            );
        }

        // Draw in point marker
        if let Some(in_point) = self.timeline.selection.in_point {
            let x = self.timeline.time_to_x(in_point);
            self.draw_in_out_marker(
                &mut frame,
                x,
                track_y,
                track_height,
                in_out_marker_color,
                true,
            );
        }

        // Draw out point marker
        if let Some(out_point) = self.timeline.selection.out_point {
            let x = self.timeline.time_to_x(out_point);
            self.draw_in_out_marker(
                &mut frame,
                x,
                track_y,
                track_height,
                in_out_marker_color,
                false,
            );
        }

        // Draw split points
        for split_time in &self.timeline.selection.split_points {
            let x = self.timeline.time_to_x(*split_time);
            self.draw_split_marker(&mut frame, x, track_y, track_height, split_color);
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
                Stroke::default().with_color(marker_color).with_width(1.0),
            );

            // Time label
            let label = format_time(current_time);
            let text = Text {
                content: label,
                position: Point::new(x, track_y - 10.0),
                color: label_color,
                size: iced::Pixels(10.0),
                ..Text::default()
            };
            frame.fill_text(text);

            current_time += marker_interval;
        }

        // Draw playhead (on top of everything else)
        let playhead_x = self.timeline.time_to_x(self.timeline.current_position);
        frame.stroke(
            &Path::line(Point::new(playhead_x, 20.0), Point::new(playhead_x, 100.0)),
            Stroke::default().with_color(playhead_color).with_width(2.0),
        );

        // Playhead handle (circle)
        let handle_radius = 6.0;
        let handle_center = Point::new(playhead_x, track_y + track_height / 2.0);
        let circle_path = Path::new(|p| {
            p.circle(handle_center, handle_radius);
        });
        frame.fill(&circle_path, playhead_color);

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

    /// Draw a trim handle at the given time position
    fn draw_trim_handle(
        &self,
        frame: &mut Frame,
        time: Duration,
        track_y: f32,
        track_height: f32,
        color: Color,
    ) {
        let x = self.timeline.time_to_x(time);
        let handle_width = 8.0;
        let handle_height = track_height + 10.0;

        // Vertical line
        frame.stroke(
            &Path::line(
                Point::new(x, track_y - 5.0),
                Point::new(x, track_y + track_height + 5.0),
            ),
            Stroke::default().with_color(color).with_width(2.0),
        );

        // Handle rectangle (draggable area)
        frame.fill_rectangle(
            Point::new(x - handle_width / 2.0, track_y - 5.0),
            iced::Size::new(handle_width, handle_height),
            Color::from_rgba(color.r, color.g, color.b, 0.3),
        );

        // Handle grip lines (3 horizontal lines)
        for i in 0..3 {
            let y = track_y + track_height / 2.0 - 6.0 + (i as f32 * 6.0);
            frame.stroke(
                &Path::line(Point::new(x - 3.0, y), Point::new(x + 3.0, y)),
                Stroke::default().with_color(color).with_width(1.0),
            );
        }
    }

    /// Draw an in/out point marker
    fn draw_in_out_marker(
        &self,
        frame: &mut Frame,
        x: f32,
        track_y: f32,
        track_height: f32,
        color: Color,
        is_in_point: bool,
    ) {
        // Vertical line
        frame.stroke(
            &Path::line(
                Point::new(x, track_y - 5.0),
                Point::new(x, track_y + track_height + 5.0),
            ),
            Stroke::default().with_color(color).with_width(2.0),
        );

        // Arrow pointing direction (in = right, out = left)
        let arrow_size = 6.0;
        let arrow_y = track_y - 10.0;
        let arrow_path = Path::new(|p| {
            if is_in_point {
                // Arrow pointing right (in point)
                p.move_to(Point::new(x - arrow_size, arrow_y - arrow_size / 2.0));
                p.line_to(Point::new(x, arrow_y));
                p.line_to(Point::new(x - arrow_size, arrow_y + arrow_size / 2.0));
                p.close();
            } else {
                // Arrow pointing left (out point)
                p.move_to(Point::new(x + arrow_size, arrow_y - arrow_size / 2.0));
                p.line_to(Point::new(x, arrow_y));
                p.line_to(Point::new(x + arrow_size, arrow_y + arrow_size / 2.0));
                p.close();
            }
        });
        frame.fill(&arrow_path, color);
    }

    /// Draw a split point marker
    fn draw_split_marker(
        &self,
        frame: &mut Frame,
        x: f32,
        track_y: f32,
        track_height: f32,
        color: Color,
    ) {
        // Dashed vertical line effect (using multiple short segments)
        let dash_length = 4.0;
        let gap_length = 3.0;
        let mut y = track_y;
        while y < track_y + track_height {
            let end_y = (y + dash_length).min(track_y + track_height);
            frame.stroke(
                &Path::line(Point::new(x, y), Point::new(x, end_y)),
                Stroke::default().with_color(color).with_width(2.0),
            );
            y += dash_length + gap_length;
        }

        // Diamond marker at top
        let diamond_size = 4.0;
        let diamond_y = track_y - 8.0;
        let diamond_path = Path::new(|p| {
            p.move_to(Point::new(x, diamond_y - diamond_size));
            p.line_to(Point::new(x + diamond_size, diamond_y));
            p.line_to(Point::new(x, diamond_y + diamond_size));
            p.line_to(Point::new(x - diamond_size, diamond_y));
            p.close();
        });
        frame.fill(&diamond_path, color);
    }

    /// Draw an X pattern overlay for cut regions
    fn draw_cut_pattern(
        &self,
        frame: &mut Frame,
        from_x: f32,
        to_x: f32,
        track_y: f32,
        track_height: f32,
    ) {
        let pattern_color = Color::from_rgba(0.8, 0.2, 0.2, 0.5);
        let spacing = 15.0;
        let stroke = Stroke::default().with_color(pattern_color).with_width(1.0);

        // Draw diagonal lines (\ direction)
        let mut x = from_x;
        while x < to_x + track_height {
            let start_x = x.max(from_x);
            let end_x = (x + track_height).min(to_x);
            let start_y = track_y + (start_x - x).max(0.0);
            let end_y = track_y + track_height - (x + track_height - end_x).max(0.0);

            if start_x < to_x && end_y > track_y {
                frame.stroke(
                    &Path::line(Point::new(start_x, start_y), Point::new(end_x, end_y)),
                    stroke.clone(),
                );
            }
            x += spacing;
        }

        // Draw diagonal lines (/ direction)
        x = from_x;
        while x < to_x + track_height {
            let start_x = x.max(from_x);
            let end_x = (x + track_height).min(to_x);
            let start_y = track_y + track_height - (start_x - x).max(0.0);
            let end_y = track_y + (x + track_height - end_x).max(0.0);

            if start_x < to_x && start_y > track_y {
                frame.stroke(
                    &Path::line(Point::new(start_x, start_y), Point::new(end_x, end_y)),
                    stroke.clone(),
                );
            }
            x += spacing;
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
