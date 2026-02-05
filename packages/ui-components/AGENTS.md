# frame-ui

Reusable iced.rs widgets and styling for Frame desktop app.

## Structure

```
src/
├── lib.rs          # Public exports
├── theme.rs        # Dark theme default
└── components/
    ├── timeline.rs       # Canvas-based timeline with edit support
    ├── error_dialog.rs   # FrameError → modal with recovery
    ├── export_dialog.rs  # Export configuration UI
    ├── keyboard_badge.rs # Keyboard shortcut overlay widget
    ├── settings_panel.rs # Effects configuration UI
    ├── button.rs         # primary_button, secondary_button
    ├── input.rs          # input_field wrapper
    └── icons.rs          # Placeholder for icons
```

## Patterns

**Simple widgets**: Functional wrappers with theme applied

```rust
use frame_ui::button::primary_button;
primary_button("Record").on_press(Message::Start)
```

**Complex widgets**: Stateful structs with `view()` + `update()`

```rust
let mut timeline = Timeline::new(clips, duration);
timeline.set_width(container_width);
timeline.view().map(Message::Timeline)
```

**Settings Panel**: Effects configuration

```rust
use frame_ui::settings_panel::{SettingsPanel, SettingsMessage};

let mut panel = SettingsPanel::new();  // or with_config(config)
panel.update(SettingsMessage::ZoomEnabledChanged(true));
panel.view().map(Message::Settings)
// Access config: panel.config()
```

**Keyboard Badge**: Shortcut overlay

```rust
use frame_ui::keyboard_badge::{KeyboardBadge, BadgeConfig, BadgePosition};

let mut badge = KeyboardBadge::with_config(BadgeConfig::for_recording());
badge.set_content(Some("⌘S".to_string()));
badge.update_time(current_time);
badge.view() // or view_fixed(width, height)
```

## Timeline Editing (Phase 4)

Timeline widget supports non-destructive edit visualization:

```rust
use frame_ui::timeline::{Timeline, SelectionState, TrimBounds};

let mut timeline = Timeline::new(total_duration);

// Selection operations
timeline.set_in_point();           // Mark in point at playhead
timeline.set_out_point();          // Mark out point at playhead
timeline.split_at_playhead();      // Add split marker
timeline.cut_selection();          // Cut selected region (I→O)
timeline.apply_trim();             // Apply trim from selection

// Access selection state
let selection = timeline.selection();
selection.in_point;                // Option<Duration>
selection.out_point;               // Option<Duration>
selection.selected_range();        // Option<(Duration, Duration)>
selection.split_points;            // Vec<Duration>
selection.cut_regions;             // Vec<(Duration, Duration)>
selection.trim;                    // Option<TrimBounds>

// Clear state
timeline.selection_mut().clear_selection();  // Clear in/out points
timeline.clear_edit_state();                 // Clear all edits
```

### Visual Indicators

| Element        | Visual                                     |
| -------------- | ------------------------------------------ |
| Trim handles   | White draggable bars with grip lines       |
| Cut regions    | Dark red fill with X pattern overlay       |
| Selection      | Blue translucent highlight                 |
| In/Out markers | Blue vertical lines with directional arrow |
| Split points   | Yellow dashed lines with diamond marker    |

### Keyboard Shortcuts (Desktop)

| Key           | Action            |
| ------------- | ----------------- |
| `I`           | Set in point      |
| `O`           | Set out point     |
| `X`           | Cut selection     |
| `S`           | Split at playhead |
| `Cmd+Z`       | Undo              |
| `Cmd+Shift+Z` | Redo              |
| `Escape`      | Clear selection   |

## Key Components

| Component       | Purpose                                     |
| --------------- | ------------------------------------------- |
| `Timeline`      | Canvas widget with edit visualization       |
| `ErrorDialog`   | Maps `FrameError` → user-facing modal       |
| `ExportDialog`  | Export format/quality configuration         |
| `KeyboardBadge` | Shortcut display with fade animation        |
| `SettingsPanel` | Effects (zoom/keyboard/background) settings |

## Theme

- Dark mode default (`Theme::Dark`)
- Uses built-in `iced::theme` variants
- Custom colors hardcoded in Timeline (TODO: centralize)

## Gotchas

- `Timeline` requires manual `set_width()` call for accurate scaling
- Timeline drag-to-seek partially wired (playhead click incomplete)
- Trim handle dragging not yet implemented (visual only)
- Time formatting reimplemented locally (not shared util)
- `ErrorDialog` tightly coupled to `frame_core::FrameError`
