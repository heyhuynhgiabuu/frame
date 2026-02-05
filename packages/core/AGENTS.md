# frame-core

Core library: capture, encoding, auto-save, effects, timeline editing, error handling.

## Structure

```
src/
├── capture/              # Screen/audio capture (ScreenCaptureKit)
│   ├── mod.rs            # ScreenCapture trait
│   ├── platform.rs       # MacOSScreenCapture impl
│   ├── audio.rs          # Audio capture
│   └── webcam.rs         # Webcam capture (nokhwa)
├── effects/              # Video effects and compositing
│   ├── mod.rs            # Types, configs, EffectsPipeline trait
│   ├── pipeline.rs       # IntegratedPipeline (combines all effects)
│   ├── cursor.rs         # CursorTracker (position, velocity, idle)
│   ├── zoom.rs           # ZoomEffect (click-to-zoom, smooth transitions)
│   ├── keyboard.rs       # KeyboardCapture (event buffer, combo display)
│   ├── background.rs     # BackgroundCompositor (padding, gradients)
│   ├── aspect_ratio.rs   # Aspect ratio calculations
│   ├── shadow.rs         # Shadow effect
│   ├── inset.rs          # Inset/depth effect
│   └── webcam_overlay.rs # Webcam overlay compositing
├── encoder/              # Video encoding
│   ├── mod.rs            # FFmpeg sidecar wrapper + EditFilter
│   └── gif.rs            # GIF encoding (gifski)
├── error.rs              # FrameError + recovery actions
├── auto_save.rs          # Background persistence
├── export_preset.rs      # Export preset system
└── project.rs            # Project/Recording models (.frame format) + EditHistory
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

## Timeline Editing (Phase 4)

Non-destructive editing with undo/redo support.

### Edit Operations

```rust
use frame_core::{EditOperation, EditHistory};

// Create edit history
let mut history = EditHistory::new();

// Push edit operations
history.push(EditOperation::Trim {
    start: Duration::from_secs(5),
    end: Duration::from_secs(30),
});

history.push(EditOperation::Cut {
    from: Duration::from_secs(10),
    to: Duration::from_secs(15),
});

history.push(EditOperation::Split {
    at: Duration::from_secs(20),
});

// Undo/redo
history.undo(); // Returns Some(&EditOperation)
history.redo();
history.can_undo(); // bool
history.can_redo(); // bool

// Get effective duration after edits
let effective = history.effective_duration(original_duration);

// Validation (prevents empty videos)
history.push_trim(original_duration, start, end)?; // Returns Result
history.push_cut(original_duration, from, to)?;
```

### Encoder Edit Support

```rust
use frame_core::encoder::EditFilter;

// Filter frames during export
let filter = EditFilter::new(&edit_history, original_duration);

for frame in frames {
    if let Some(adjusted_time) = filter.filter_timestamp(frame.timestamp) {
        // Frame is included, use adjusted_time as new timestamp
        encoder.encode_frame_at(frame, adjusted_time)?;
    }
    // Frame excluded if None (trimmed or cut)
}
```

## Phase 5 Features

Webcam capture, visual effects, and export presets.

### Webcam Capture

```rust
use frame_core::capture::webcam::WebcamCapture;

// Initialize webcam capture
let webcam = WebcamCapture::new()?;

// Start webcam capture stream
webcam.start()?;

// Get current frame (composited with screen)
let frame = webcam.capture_frame()?;

// Configure webcam overlay position
let overlay = WebcamOverlay {
    position: OverlayPosition::BottomRight,
    size: WebcamSize::Medium,
    border_radius: 8.0,
};
```

### Visual Effects

```rust
use frame_core::effects::{ShadowEffect, InsetEffect, AspectRatio};

// Shadow effect
let shadow = ShadowEffect::new(ShadowConfig {
    blur_radius: 12.0,
    offset: (4.0, 4.0),
    color: Color::new(0.0, 0.0, 0.0, 0.3),
});

// Inset/depth effect
let inset = InsetEffect::new(InsetConfig {
    padding: 20.0,
    corner_radius: 12.0,
    background: BackgroundStyle::Gradient { /* ... */ },
});

// Aspect ratio control
let aspect = AspectRatio::new(16, 9);
let constrained = aspect.apply(frame);
```

### Export Presets

```rust
use frame_core::export_preset::{ExportPreset, ExportFormat, Quality};

// Predefined presets
let gif_preset = ExportPreset::gif(GifConfig {
    fps: 30,
    max_colors: 128,
    dither: true,
});

let mp4_preset = ExportPreset::mp4(VideoConfig {
    resolution: (1920, 1080),
    fps: 60,
    quality: Quality::High,
    codec: VideoCodec::H264,
});

// Custom preset
let custom = ExportPreset::builder()
    .format(ExportFormat::WebM)
    .resolution(2560, 1440)
    .fps(60)
    .build()?;
```

## Project Format

Binary `.frame` format (v2 - includes edit history):

```
MAGIC: b"FRAME" (5 bytes)
VERSION: u32 le (4 bytes)
JSON: Project struct (includes edit_history)
```

```rust
// Save/load projects (edits persist automatically)
project.save_to_file("path.frame")?;
let loaded = Project::load_from_file("path.frame")?;

// Access edit history
let history = &project.edit_history;
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
🚫 Don't skip validation → use `push_trim()` / `push_cut()` instead of raw `push()`

## Gotchas

- Features disabled at runtime return `PlatformNotSupported`, not compile error
- `Encoder` manages temp files (`.video.mp4`, `.audio.wav`) - unclean exit leaves them
- First run may download FFmpeg via `auto_download()`
- Tests use `#[tokio::test]` for async
- Edit history has MAX_UNDO_HISTORY (50) limit to prevent unbounded memory
