# frame-core

Core library: capture, encoding, auto-save, effects, error handling.

## Structure

```
src/
├── capture/        # Screen/audio capture (ScreenCaptureKit)
│   ├── mod.rs      # ScreenCapture trait
│   ├── platform.rs # MacOSScreenCapture impl
│   └── audio.rs    # Audio capture
├── effects/        # Video effects and compositing
│   ├── mod.rs      # Types, configs, EffectsPipeline trait
│   ├── pipeline.rs # IntegratedPipeline (combines all effects)
│   ├── cursor.rs   # CursorTracker (position, velocity, idle)
│   ├── zoom.rs     # ZoomEffect (click-to-zoom, smooth transitions)
│   ├── keyboard.rs # KeyboardCapture (event buffer, combo display)
│   └── background.rs # BackgroundCompositor (padding, gradients)
├── error.rs        # FrameError + recovery actions
├── encoder.rs      # FFmpeg sidecar wrapper
├── auto_save.rs    # Background persistence
└── project.rs      # Project/Recording models (.frame format)
```

## Effects System

```rust
use frame_core::effects::{IntegratedPipeline, EffectsConfig, EffectInput, MouseEvent};

// Create pipeline with default config
let mut pipeline = IntegratedPipeline::default();

// Or customize config
let config = EffectsConfig {
    zoom: ZoomConfig { enabled: true, max_zoom: 1.5, .. },
    keyboard: KeyboardConfig { enabled: true, .. },
    background: Background::default(),
};
let mut pipeline = IntegratedPipeline::new(config);

// Process input events
pipeline.process_input(EffectInput::Mouse(MouseEvent::Click { x, y, button }));

// Process frames
let result = pipeline.process_frame(frame)?;
// result.frame = processed frame
// result.keyboard_badges = list of KeyboardBadge { text, position, opacity }
```

## Project Format

Binary `.frame` format (v1):

```
MAGIC: b"FRAME" (5 bytes)
VERSION: u32 le (4 bytes)
JSON: Project struct
```

```rust
// Save/load projects
project.save_to_file("path.frame")?;
let loaded = Project::load_from_file("path.frame")?;
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
