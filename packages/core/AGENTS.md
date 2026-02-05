# frame-core

Core library: capture, encoding, auto-save, error handling.

## Structure

```
src/
├── capture/        # Screen/audio capture (ScreenCaptureKit)
│   ├── mod.rs      # ScreenCapture trait
│   ├── platform.rs # MacOSScreenCapture impl
│   └── audio.rs    # Audio capture
├── error.rs        # FrameError + recovery actions
├── encoder.rs      # FFmpeg sidecar wrapper
├── auto_save.rs    # Background persistence
└── project.rs      # Project/Recording models
```

## Patterns

```rust
// Always use FrameResult<T> (not anyhow::Result)
pub fn do_work() -> FrameResult<()> {
    op().map_err(|e| FrameError::Io {
        source: e,
        context: ErrorContext::Project { name: "x".into() },
    })?;
    Ok(())
}

// Use .into_frame_error() extension trait
std::fs::read(path).into_frame_error(ErrorContext::File { path })?;
```

## Feature Flags

| Flag             | Purpose                                       |
| ---------------- | --------------------------------------------- |
| `capture`        | Screen/audio capture (macOS ScreenCaptureKit) |
| `encoding`       | FFmpeg-sidecar video encoding                 |
| `encoding-libav` | FFmpeg-next (libav) alternative               |
| `pro`            | Commercial features (placeholder)             |

## Error System

`FrameError` provides:

- `is_recoverable()` → Can retry?
- `recovery_action()` → UI hint (Retry, RequestPermissions, etc.)
- `severity()` → Warning, Error, Critical

## Anti-Patterns

🚫 Don't use `std::fs` in async context → use `tokio::fs`
🚫 Don't construct paths manually → project directory helpers exist
🚫 Don't ignore recovery actions → propagate to UI

## Gotchas

- Features disabled at runtime return `PlatformNotSupported`, not compile error
- `Encoder` manages temp files (`.video.mp4`, `.audio.wav`) - unclean exit leaves them
- First run may download FFmpeg via `auto_download()`
- Tests use `#[tokio::test]` for async
