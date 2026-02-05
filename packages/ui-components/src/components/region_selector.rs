//! Region selector overlay for screen capture area selection
//!
//! Provides a full-screen transparent overlay where users can drag to select
//! a rectangular region. Supports resizing via 8 handles and keyboard shortcuts.

use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Program, Stroke, Text};
use iced::{keyboard, mouse, Color, Element, Length, Point, Rectangle, Renderer, Theme};

/// Represents a selected screen region
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RegionSelection {
    /// X coordinate of the top-left corner
    pub x: u32,
    /// Y coordinate of the top-left corner
    pub y: u32,
    /// Width of the selection
    pub width: u32,
    /// Height of the selection
    pub height: u32,
}

impl RegionSelection {
    /// Create a new region selection
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if the selection has non-zero dimensions
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// Get the right edge coordinate
    pub fn right(&self) -> u32 {
        self.x + self.width
    }

    /// Get the bottom edge coordinate
    pub fn bottom(&self) -> u32 {
        self.y + self.height
    }

    /// Get the center point
    pub fn center(&self) -> Point {
        Point::new(
            (self.x + self.width / 2) as f32,
            (self.y + self.height / 2) as f32,
        )
    }
}

/// Messages produced by the region selector
#[derive(Debug, Clone)]
pub enum RegionMessage {
    /// User started dragging to create/modify selection
    DragStarted(Point),
    /// User is actively dragging
    Dragging(Point),
    /// User released the drag
    DragEnded,
    /// Selection was confirmed (Enter key)
    Confirmed(RegionSelection),
    /// Selection was cancelled (Escape key)
    Cancelled,
    /// Selection is being resized via a handle
    Resizing(HandlePosition, Point),
    /// Resize operation completed
    ResizeEnded,
}

/// Position of resize handles (8 points around the selection)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlePosition {
    /// Top-left corner
    TopLeft,
    /// Top edge midpoint
    TopMiddle,
    /// Top-right corner
    TopRight,
    /// Right edge midpoint
    RightMiddle,
    /// Bottom-right corner
    BottomRight,
    /// Bottom edge midpoint
    BottomMiddle,
    /// Bottom-left corner
    BottomLeft,
    /// Left edge midpoint
    LeftMiddle,
}

impl HandlePosition {
    /// Get all handle positions
    pub fn all() -> [HandlePosition; 8] {
        [
            HandlePosition::TopLeft,
            HandlePosition::TopMiddle,
            HandlePosition::TopRight,
            HandlePosition::RightMiddle,
            HandlePosition::BottomRight,
            HandlePosition::BottomMiddle,
            HandlePosition::BottomLeft,
            HandlePosition::LeftMiddle,
        ]
    }

    /// Get cursor position for this handle given a region
    fn point(&self, region: &RegionSelection) -> Point {
        match self {
            HandlePosition::TopLeft => Point::new(region.x as f32, region.y as f32),
            HandlePosition::TopMiddle => {
                Point::new((region.x + region.width / 2) as f32, region.y as f32)
            }
            HandlePosition::TopRight => Point::new(region.right() as f32, region.y as f32),
            HandlePosition::RightMiddle => {
                Point::new(region.right() as f32, (region.y + region.height / 2) as f32)
            }
            HandlePosition::BottomRight => {
                Point::new(region.right() as f32, region.bottom() as f32)
            }
            HandlePosition::BottomMiddle => {
                Point::new((region.x + region.width / 2) as f32, region.bottom() as f32)
            }
            HandlePosition::BottomLeft => Point::new(region.x as f32, region.bottom() as f32),
            HandlePosition::LeftMiddle => {
                Point::new(region.x as f32, (region.y + region.height / 2) as f32)
            }
        }
    }
}

/// Current interaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InteractionState {
    /// No active interaction
    #[default]
    Idle,
    /// Creating a new selection by dragging
    CreatingSelection,
    /// Moving the entire selection
    MovingSelection,
    /// Resizing via a specific handle
    Resizing(HandlePosition),
}

/// Region selector widget state
#[derive(Debug, Clone)]
pub struct RegionSelector {
    /// Current selection (if any)
    selection: Option<RegionSelection>,
    /// Starting point of current drag operation
    drag_start: Option<Point>,
    /// Current interaction state
    state: InteractionState,
    /// Minimum selection size in pixels
    min_size: u32,
    /// Handle size for hit testing (radius)
    handle_radius: f32,
}

impl Default for RegionSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionSelector {
    /// Create a new region selector
    pub fn new() -> Self {
        Self {
            selection: None,
            drag_start: None,
            state: InteractionState::Idle,
            min_size: 10,
            handle_radius: 8.0,
        }
    }

    /// Get the current selection
    pub fn selection(&self) -> Option<RegionSelection> {
        self.selection
    }

    /// Set the current selection
    pub fn set_selection(&mut self, selection: Option<RegionSelection>) {
        self.selection = selection;
    }

    /// Clear the current selection
    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.state = InteractionState::Idle;
    }

    /// Update the selector state based on a message
    pub fn update(&mut self, message: RegionMessage) -> Option<RegionMessage> {
        match message {
            RegionMessage::DragStarted(point) => {
                // Check if clicking on an existing handle
                if let Some(ref selection) = self.selection {
                    if let Some(handle) = self.hit_test_handle(selection, point) {
                        self.state = InteractionState::Resizing(handle);
                        self.drag_start = Some(point);
                        return None;
                    }

                    // Check if clicking inside the selection (to move it)
                    if self.is_inside_selection(selection, point) {
                        self.state = InteractionState::MovingSelection;
                        self.drag_start = Some(point);
                        return None;
                    }
                }

                // Start creating a new selection
                self.state = InteractionState::CreatingSelection;
                self.drag_start = Some(point);
                self.selection = None;
                None
            }
            RegionMessage::Dragging(current) => {
                match self.state {
                    InteractionState::CreatingSelection => {
                        if let Some(start) = self.drag_start {
                            self.selection =
                                Some(self.create_selection_from_points(start, current));
                        }
                    }
                    InteractionState::MovingSelection => {
                        if let (Some(start), Some(ref mut selection)) =
                            (self.drag_start, self.selection.as_mut())
                        {
                            let dx = current.x - start.x;
                            let dy = current.y - start.y;

                            // Only update if we've moved enough
                            if dx.abs() > 1.0 || dy.abs() > 1.0 {
                                selection.x = (selection.x as f32 + dx) as u32;
                                selection.y = (selection.y as f32 + dy) as u32;
                                self.drag_start = Some(current);
                            }
                        }
                    }
                    InteractionState::Resizing(handle) => {
                        if let (Some(start), Some(selection)) =
                            (self.drag_start, self.selection.as_mut())
                        {
                            Self::resize_selection_static(
                                selection,
                                handle,
                                start,
                                current,
                                self.min_size,
                            );
                            self.drag_start = Some(current);
                        }
                    }
                    InteractionState::Idle => {}
                }
                None
            }
            RegionMessage::DragEnded | RegionMessage::ResizeEnded => {
                // Validate minimum size
                if let Some(ref selection) = self.selection {
                    if selection.width < self.min_size || selection.height < self.min_size {
                        self.selection = None;
                    }
                }

                self.state = InteractionState::Idle;
                self.drag_start = None;
                None
            }
            RegionMessage::Confirmed(_) => {
                // This should be handled by the parent - it receives selection and closes
                Some(message)
            }
            RegionMessage::Cancelled => {
                self.clear_selection();
                Some(message)
            }
            RegionMessage::Resizing(handle, point) => {
                self.state = InteractionState::Resizing(handle);
                self.drag_start = Some(point);
                None
            }
        }
    }

    /// Build the view
    pub fn view(&self) -> Element<'_, RegionMessage> {
        Canvas::new(RegionSelectorProgram { selector: self })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Test if a point is near a handle
    fn hit_test_handle(&self, selection: &RegionSelection, point: Point) -> Option<HandlePosition> {
        for handle in HandlePosition::all() {
            let handle_point = handle.point(selection);
            let distance =
                ((point.x - handle_point.x).powi(2) + (point.y - handle_point.y).powi(2)).sqrt();
            if distance <= self.handle_radius {
                return Some(handle);
            }
        }
        None
    }

    /// Test if a point is inside the selection
    fn is_inside_selection(&self, selection: &RegionSelection, point: Point) -> bool {
        point.x >= selection.x as f32
            && point.x <= (selection.x + selection.width) as f32
            && point.y >= selection.y as f32
            && point.y <= (selection.y + selection.height) as f32
    }

    /// Create a selection from two points (normalizes to top-left origin)
    fn create_selection_from_points(&self, start: Point, end: Point) -> RegionSelection {
        let x = start.x.min(end.x).max(0.0) as u32;
        let y = start.y.min(end.y).max(0.0) as u32;
        let width = (start.x.max(end.x) - start.x.min(end.x)).abs() as u32;
        let height = (start.y.max(end.y) - start.y.min(end.y)).abs() as u32;

        RegionSelection::new(x, y, width, height)
    }

    /// Resize selection based on handle movement (static version)
    fn resize_selection_static(
        selection: &mut RegionSelection,
        handle: HandlePosition,
        start: Point,
        current: Point,
        min_size: u32,
    ) {
        let dx = current.x - start.x;
        let dy = current.y - start.y;

        match handle {
            HandlePosition::TopLeft => {
                let new_x = (selection.x as f32 + dx) as u32;
                let new_y = (selection.y as f32 + dy) as u32;
                let new_width = (selection.width as f32 - dx) as u32;
                let new_height = (selection.height as f32 - dy) as u32;

                if new_width >= min_size && new_height >= min_size {
                    selection.x = new_x;
                    selection.y = new_y;
                    selection.width = new_width;
                    selection.height = new_height;
                }
            }
            HandlePosition::TopMiddle => {
                let new_y = (selection.y as f32 + dy) as u32;
                let new_height = (selection.height as f32 - dy) as u32;

                if new_height >= min_size {
                    selection.y = new_y;
                    selection.height = new_height;
                }
            }
            HandlePosition::TopRight => {
                let new_y = (selection.y as f32 + dy) as u32;
                let new_width = (selection.width as f32 + dx) as u32;
                let new_height = (selection.height as f32 - dy) as u32;

                if new_width >= min_size && new_height >= min_size {
                    selection.y = new_y;
                    selection.width = new_width;
                    selection.height = new_height;
                }
            }
            HandlePosition::RightMiddle => {
                let new_width = (selection.width as f32 + dx) as u32;

                if new_width >= min_size {
                    selection.width = new_width;
                }
            }
            HandlePosition::BottomRight => {
                let new_width = (selection.width as f32 + dx) as u32;
                let new_height = (selection.height as f32 + dy) as u32;

                if new_width >= min_size && new_height >= min_size {
                    selection.width = new_width;
                    selection.height = new_height;
                }
            }
            HandlePosition::BottomMiddle => {
                let new_height = (selection.height as f32 + dy) as u32;

                if new_height >= min_size {
                    selection.height = new_height;
                }
            }
            HandlePosition::BottomLeft => {
                let new_x = (selection.x as f32 + dx) as u32;
                let new_width = (selection.width as f32 - dx) as u32;
                let new_height = (selection.height as f32 + dy) as u32;

                if new_width >= min_size && new_height >= min_size {
                    selection.x = new_x;
                    selection.width = new_width;
                    selection.height = new_height;
                }
            }
            HandlePosition::LeftMiddle => {
                let new_x = (selection.x as f32 + dx) as u32;
                let new_width = (selection.width as f32 - dx) as u32;

                if new_width >= min_size {
                    selection.x = new_x;
                    selection.width = new_width;
                }
            }
        }
    }
}

/// Canvas program for rendering the region selector
struct RegionSelectorProgram<'a> {
    selector: &'a RegionSelector,
}

impl<'a> RegionSelectorProgram<'a> {
    /// Draw the selection rectangle
    fn draw_selection(&self, frame: &mut Frame, selection: &RegionSelection) {
        let stroke_color = Color::from_rgb(0.3, 0.7, 1.0); // Bright blue
        let fill_color = Color::from_rgba(0.3, 0.7, 1.0, 0.15); // Translucent blue
        let stroke_width = 2.0;

        // Fill the selection area
        frame.fill_rectangle(
            Point::new(selection.x as f32, selection.y as f32),
            iced::Size::new(selection.width as f32, selection.height as f32),
            fill_color,
        );

        // Draw the border
        let rect_path = Path::rectangle(
            Point::new(selection.x as f32, selection.y as f32),
            iced::Size::new(selection.width as f32, selection.height as f32),
        );
        frame.stroke(
            &rect_path,
            Stroke::default()
                .with_color(stroke_color)
                .with_width(stroke_width),
        );

        // Draw resize handles
        for handle in HandlePosition::all() {
            self.draw_handle(frame, handle, selection, stroke_color);
        }

        // Draw dimension label
        self.draw_dimension_label(frame, selection);
    }

    /// Draw a single resize handle
    fn draw_handle(
        &self,
        frame: &mut Frame,
        handle: HandlePosition,
        selection: &RegionSelection,
        color: Color,
    ) {
        let point = handle.point(selection);
        let size = 10.0; // Handle visual size
        let half_size = size / 2.0;

        // Handle background (white with border)
        let handle_rect = Path::rectangle(
            Point::new(point.x - half_size, point.y - half_size),
            iced::Size::new(size, size),
        );
        frame.fill(&handle_rect, Color::WHITE);
        frame.stroke(
            &handle_rect,
            Stroke::default().with_color(color).with_width(2.0),
        );
    }

    /// Draw dimension label showing width x height
    fn draw_dimension_label(&self, frame: &mut Frame, selection: &RegionSelection) {
        let label = format!("{} x {}", selection.width, selection.height);

        // Position label above the selection, centered
        let label_x = (selection.x + selection.width / 2) as f32;
        let label_y = selection.y as f32 - 25.0;

        // Draw label background
        let bg_width = 80.0;
        let bg_height = 20.0;
        let bg_rect = Path::rectangle(
            Point::new(label_x - bg_width / 2.0, label_y - bg_height / 2.0),
            iced::Size::new(bg_width, bg_height),
        );
        frame.fill(&bg_rect, Color::from_rgba(0.0, 0.0, 0.0, 0.7));

        // Draw label text
        let text = Text {
            content: label,
            position: Point::new(label_x, label_y + 4.0), // Slight vertical adjustment for centering
            color: Color::WHITE,
            size: iced::Pixels(12.0),
            ..Text::default()
        };
        frame.fill_text(text);
    }

    /// Draw instructions/help text
    fn draw_instructions(&self, frame: &mut Frame, bounds: Rectangle) {
        let instruction_text = if self.selector.selection.is_some() {
            "Drag to move • Drag handles to resize • Enter to confirm • Escape to cancel"
        } else {
            "Click and drag to select a region • Escape to cancel"
        };

        let text = Text {
            content: instruction_text.to_string(),
            position: Point::new(bounds.width / 2.0, bounds.height - 40.0),
            color: Color::from_rgb(0.8, 0.8, 0.8),
            size: iced::Pixels(14.0),
            ..Text::default()
        };
        frame.fill_text(text);
    }
}

impl<'a> Program<RegionMessage> for RegionSelectorProgram<'a> {
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

        // Semi-transparent overlay for the entire screen
        let overlay_color = Color::from_rgba(0.0, 0.0, 0.0, 0.3);
        frame.fill_rectangle(Point::new(0.0, 0.0), bounds.size(), overlay_color);

        // Draw the current selection if it exists
        if let Some(ref selection) = self.selector.selection {
            // Clear the overlay inside the selection (make it transparent)
            // We do this by drawing the selection fill color
            frame.fill_rectangle(
                Point::new(selection.x as f32, selection.y as f32),
                iced::Size::new(selection.width as f32, selection.height as f32),
                Color::TRANSPARENT,
            );

            // Draw the selection rectangle with handles
            self.draw_selection(&mut frame, selection);
        }

        // Draw instructions at the bottom
        self.draw_instructions(&mut frame, bounds);

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<RegionMessage>) {
        match event {
            canvas::Event::Mouse(mouse_event) => match mouse_event {
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    if let Some(position) = cursor.position_in(bounds) {
                        return (
                            canvas::event::Status::Captured,
                            Some(RegionMessage::DragStarted(position)),
                        );
                    }
                    (canvas::event::Status::Ignored, None)
                }
                mouse::Event::ButtonReleased(mouse::Button::Left) => (
                    canvas::event::Status::Captured,
                    Some(RegionMessage::DragEnded),
                ),
                mouse::Event::CursorMoved { .. } => {
                    if let Some(position) = cursor.position_in(bounds) {
                        return (
                            canvas::event::Status::Captured,
                            Some(RegionMessage::Dragging(position)),
                        );
                    }
                    (canvas::event::Status::Ignored, None)
                }
                _ => (canvas::event::Status::Ignored, None),
            },
            canvas::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => match key {
                keyboard::Key::Named(keyboard::key::Named::Enter) => {
                    if let Some(ref selection) = self.selector.selection {
                        if selection.is_valid() {
                            return (
                                canvas::event::Status::Captured,
                                Some(RegionMessage::Confirmed(*selection)),
                            );
                        }
                    }
                    (canvas::event::Status::Ignored, None)
                }
                keyboard::Key::Named(keyboard::key::Named::Escape) => (
                    canvas::event::Status::Captured,
                    Some(RegionMessage::Cancelled),
                ),
                _ => (canvas::event::Status::Ignored, None),
            },
            _ => (canvas::event::Status::Ignored, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_selection_creation() {
        let region = RegionSelection::new(100, 200, 800, 600);
        assert_eq!(region.x, 100);
        assert_eq!(region.y, 200);
        assert_eq!(region.width, 800);
        assert_eq!(region.height, 600);
        assert!(region.is_valid());
    }

    #[test]
    fn test_region_selection_invalid() {
        let region = RegionSelection::new(0, 0, 0, 100);
        assert!(!region.is_valid());

        let region = RegionSelection::new(0, 0, 100, 0);
        assert!(!region.is_valid());
    }

    #[test]
    fn test_handle_positions() {
        let region = RegionSelection::new(100, 100, 200, 200);

        assert_eq!(
            HandlePosition::TopLeft.point(&region),
            Point::new(100.0, 100.0)
        );
        assert_eq!(
            HandlePosition::TopMiddle.point(&region),
            Point::new(200.0, 100.0)
        );
        assert_eq!(
            HandlePosition::TopRight.point(&region),
            Point::new(300.0, 100.0)
        );
        assert_eq!(
            HandlePosition::RightMiddle.point(&region),
            Point::new(300.0, 200.0)
        );
        assert_eq!(
            HandlePosition::BottomRight.point(&region),
            Point::new(300.0, 300.0)
        );
        assert_eq!(
            HandlePosition::BottomMiddle.point(&region),
            Point::new(200.0, 300.0)
        );
        assert_eq!(
            HandlePosition::BottomLeft.point(&region),
            Point::new(100.0, 300.0)
        );
        assert_eq!(
            HandlePosition::LeftMiddle.point(&region),
            Point::new(100.0, 200.0)
        );
    }

    #[test]
    fn test_selector_update_drag_started() {
        let mut selector = RegionSelector::new();
        let result = selector.update(RegionMessage::DragStarted(Point::new(100.0, 100.0)));

        assert!(result.is_none());
        assert!(matches!(
            selector.state,
            InteractionState::CreatingSelection
        ));
    }

    #[test]
    fn test_selector_update_confirmed() {
        let mut selector = RegionSelector::new();
        selector.selection = Some(RegionSelection::new(100, 100, 200, 200));

        let result = selector.update(RegionMessage::Confirmed(RegionSelection::new(
            100, 100, 200, 200,
        )));

        assert!(result.is_some());
        assert!(matches!(result.unwrap(), RegionMessage::Confirmed(_)));
    }

    #[test]
    fn test_selector_update_cancelled() {
        let mut selector = RegionSelector::new();
        selector.selection = Some(RegionSelection::new(100, 100, 200, 200));

        let result = selector.update(RegionMessage::Cancelled);

        assert!(result.is_some());
        assert!(selector.selection.is_none());
    }
}
