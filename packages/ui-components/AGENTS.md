# frame-ui

Reusable iced.rs widgets and styling for Frame desktop app.

## Structure

```
src/
├── lib.rs          # Public exports
├── theme.rs        # Dark theme default
└── components/
    ├── timeline.rs       # Canvas-based recording timeline
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

## Key Components

| Component       | Purpose                                     |
| --------------- | ------------------------------------------- |
| `Timeline`      | Canvas widget for recording navigation      |
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
- Timeline interactions partially wired (playhead click incomplete)
- Time formatting reimplemented locally (not shared util)
- `ErrorDialog` tightly coupled to `frame_core::FrameError`
