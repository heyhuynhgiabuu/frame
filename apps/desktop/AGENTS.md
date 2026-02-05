# frame-desktop

Main GUI application built with iced.rs (The Elm Architecture).

## Structure

```
src/
├── main.rs         # Entry point, tracing setup
├── app.rs          # FrameApp (Model), Message, update/view
├── ui/
│   ├── main.rs     # View routing by AppState
│   └── toolbar.rs  # Top toolbar
└── recording/
    └── mod.rs      # RecordingService (async capture orchestration)
```

## Architecture (TEA)

```
FrameApp (Model) ─→ view() ─→ Element tree
     ↑                              │
     └──── update(Message) ←────────┘
                 │
                 ↓
          Command::perform (async tasks)
```

## State Machine

```
CheckingPermissions → PermissionRequired (if denied)
        ↓
      Idle → Recording → Previewing → ExportConfiguring → Exporting
```

## Integration

| Package      | Usage                                                 |
| ------------ | ----------------------------------------------------- |
| `frame-core` | RecordingService, CaptureConfig, Encoder, EditHistory |
| `frame-ui`   | Timeline, ErrorDialog, ExportDialog, buttons          |

## Timeline Editing (Phase 4)

Edit operations are triggered via keyboard shortcuts in the `Previewing` state:

### Message Flow

```
KeyPressed(I) → SetInPoint → timeline.set_in_point()
KeyPressed(O) → SetOutPoint → timeline.set_out_point()
KeyPressed(X) → CutSelection → edit_history.push(Cut{...})
KeyPressed(S) → SplitAtPlayhead → edit_history.push(Split{...})
Cmd+Z → Undo → edit_history.undo()
Cmd+Shift+Z → Redo → edit_history.redo()
Escape → ClearSelection → timeline.selection_mut().clear_selection()
```

### Keyboard Shortcut Handler

Located in `app.rs::handle_edit_shortcut()`:

| Key           | Message         |
| ------------- | --------------- |
| `i`           | SetInPoint      |
| `o`           | SetOutPoint     |
| `x`           | CutSelection    |
| `s`           | SplitAtPlayhead |
| `Cmd+Z`       | Undo            |
| `Cmd+Shift+Z` | Redo            |
| `Escape`      | ClearSelection  |

### Edit State

`FrameApp` maintains `edit_history: EditHistory` which:

- Mirrors edits to project on save
- Syncs with timeline visualization state
- Persists in `.frame` project files

## Patterns

```rust
// Async operations via Command::perform
Command::perform(
    async { recording_service.start().await },
    Message::RecordingStarted
)

// State transitions in update()
AppState::Recording { .. } => match message {
    Message::StopRecording => { ... }
}

// Keyboard event subscription (Previewing state only)
iced::event::listen_with(|event, _| {
    if let Event::Keyboard(KeyPressed { key, modifiers, .. }) = event {
        handle_edit_shortcut(key, modifiers)
    } else {
        None
    }
})
```

## macOS-Specific

- Permission checks via `open -b com.apple.systempreferences`
- TCC (Transparency, Consent, Control) for screen capture
- Uses `#[cfg(target_os = "macos")]` guards

## Gotchas

- `AutoSaveService` moved into tokio::spawn - separate finalizer instance needed
- Timeline defaults to 30s placeholder duration
- Permission check assumes `create_capture() Ok` = granted (may be aggressive)
- Edit keyboard shortcuts only active in `Previewing` state
- Undo/redo clears timeline visualization (full sync pending)
