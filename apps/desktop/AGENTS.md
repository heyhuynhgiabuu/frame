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

| Package      | Usage                                              |
| ------------ | -------------------------------------------------- |
| `frame-core` | RecordingService, CaptureConfig, Encoder, AutoSave |
| `frame-ui`   | Timeline, ErrorDialog, ExportDialog, buttons       |

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
```

## macOS-Specific

- Permission checks via `open -b com.apple.systempreferences`
- TCC (Transparency, Consent, Control) for screen capture
- Uses `#[cfg(target_os = "macos")]` guards

## Gotchas

- `AutoSaveService` moved into tokio::spawn - separate finalizer instance needed
- Timeline defaults to 30s placeholder duration
- Permission check assumes `create_capture() Ok` = granted (may be aggressive)
