# Frame API Documentation

## Overview

Frame is built with a modular architecture consisting of multiple Rust crates. This document describes the public API for each crate.

---

## frame-core

Core library for screen recording functionality.

### Modules

#### `capture`

Screen and audio capture abstractions.

```rust
use frame_core::capture::{ScreenCapture, CaptureConfig, CaptureArea, Frame, AudioBuffer};
```

**Traits:**

- `ScreenCapture` - Platform-agnostic screen capture interface
  - `start(config: CaptureConfig)` - Begin capture session
  - `stop()` - End capture session
  - `next_frame()` - Get next video frame
  - `next_audio_buffer()` - Get next audio buffer

**Structs:**

- `CaptureConfig` - Configuration for capture session
  - `capture_area: CaptureArea` - Full screen, window, or region
  - `capture_cursor: bool` - Whether to capture cursor
  - `capture_audio: bool` - Whether to capture audio
  - `frame_rate: u32` - Target frame rate (e.g., 60)

- `Frame` - Video frame data
  - `data: Vec<u8>` - Raw pixel data
  - `width: u32` - Frame width
  - `height: u32` - Frame height
  - `timestamp: Duration` - Frame timestamp
  - `format: PixelFormat` - Pixel format (RGBA, BGRA, etc.)

- `AudioBuffer` - Audio sample data
  - `samples: Vec<f32>` - Audio samples
  - `sample_rate: u32` - Sample rate (e.g., 48000)
  - `channels: u16` - Number of channels
  - `timestamp: Duration` - Buffer timestamp

#### `encoder`

Video encoding using ffmpeg.

```rust
use frame_core::encoder::Encoder;
```

**Structs:**

- `Encoder` - Video encoder
  - `new()` - Create new encoder
  - `encode_frame(frame: &Frame)` - Encode video frame
  - `encode_audio(buffer: &AudioBuffer)` - Encode audio buffer
  - `finalize(output_path: &Path)` - Write final video file

#### `project`

Project management and persistence.

```rust
use frame_core::project::{Project, ProjectSettings, Recording, Export};
```

**Structs:**

- `Project` - Recording project
  - `new(name: &str)` - Create new project
  - `save()` - Save project to disk
  - `load(project_id: &str)` - Load project from disk
  - `project_dir()` - Get project directory path
  - `id: String` - Unique project ID
  - `name: String` - Project name
  - `settings: ProjectSettings` - Project settings
  - `recordings: Vec<Recording>` - List of recordings
  - `exports: Vec<Export>` - List of exports

- `ProjectSettings` - Project configuration
  - `resolution: Resolution` - Video resolution
  - `frame_rate: u32` - Frame rate
  - `video_codec: VideoCodec` - Video codec (H.264, H.265, etc.)
  - `audio_codec: AudioCodec` - Audio codec (AAC, Opus, etc.)
  - `quality: Quality` - Quality preset

- `Recording` - Individual recording session
  - `id: String` - Recording ID
  - `started_at: DateTime<Utc>` - Start timestamp
  - `duration_ms: u64` - Duration in milliseconds
  - `file_path: PathBuf` - Path to raw recording file

- `Export` - Exported video file
  - `id: String` - Export ID
  - `file_path: PathBuf` - Path to exported file
  - `format: ExportFormat` - Export format (MP4, MOV, etc.)
  - `resolution: Resolution` - Export resolution

**Enums:**

- `Resolution` - Video resolutions
  - `Hd720` - 1280x720
  - `Hd1080` - 1920x1080
  - `QuadHd` - 2560x1440
  - `Uhd4k` - 3840x2160

- `VideoCodec` - Video codecs
  - `H264` - H.264/AVC
  - `H265` - H.265/HEVC
  - `ProRes` - Apple ProRes

- `AudioCodec` - Audio codecs
  - `Aac` - AAC
  - `Opus` - Opus

- `Quality` - Quality presets
  - `Low` - Low quality, small file
  - `Medium` - Balanced
  - `High` - High quality

#### `error`

Error types for Frame.

```rust
use frame_core::{FrameError, FrameResult};
```

**Types:**

- `FrameError` - Error enum with variants:
  - `Io(std::io::Error)` - IO errors
  - `Serialization(serde_json::Error)` - JSON errors
  - `Capture(String)` - Capture errors
  - `Encoding(String)` - Encoding errors
  - `Project(String)` - Project errors
  - `Audio(String)` - Audio errors
  - `PlatformNotSupported(String)` - Platform errors
  - `ProFeature(String)` - Pro feature errors

- `FrameResult<T>` - Result type alias

---

## frame-ui

Reusable UI components for iced.rs.

### Modules

#### `components`

UI components.

```rust
use frame_ui::components::{primary_button, secondary_button, text_input};
```

**Functions:**

- `primary_button(label: &str) -> Button<Message>` - Primary action button
- `secondary_button(label: &str) -> Button<Message>` - Secondary action button
- `text_input(placeholder: &str, value: &str) -> TextInput<Message>` - Text input field

#### `theme`

Theme and styling.

```rust
use frame_ui::theme::default_theme;
```

**Functions:**

- `default_theme() -> Theme` - Returns default dark theme

---

## frame-renderer

GPU-accelerated rendering (future feature).

```rust
use frame_renderer::Renderer;
```

**Structs:**

- `Renderer` - GPU renderer
  - `new() -> Renderer` - Create new renderer

---

## frame-desktop

Main desktop application. Not a library, but documents the app structure.

### Architecture

The desktop app uses **The Elm Architecture** pattern:

1. **Model** - `FrameApp` struct holds application state
2. **Messages** - `Message` enum represents all possible actions
3. **Update** - `update()` function handles state transitions
4. **View** - `view()` function renders UI based on state

### State Machine

```
Idle → Recording → Previewing → Exporting → Idle
```

**States:**

- `Idle` - Ready to record
- `Recording { start_time }` - Currently recording
- `Previewing { project_id }` - Reviewing recording
- `Exporting { project_id, progress }` - Exporting video

### Messages

- `StartRecording` - Begin recording
- `StopRecording` - End recording
- `PauseRecording` - Pause (if supported)
- `ResumeRecording` - Resume (if supported)
- `RecordingStarted` - Recording started successfully
- `RecordingStopped(project_id)` - Recording stopped
- `ExportProject(project_id)` - Start export
- `ExportProgress(f32)` - Export progress update
- `ExportComplete` - Export finished
- `ThemeChanged(Theme)` - Change UI theme
- `SettingsOpened` - Open settings

---

## Examples

### Creating a New Project

```rust
use frame_core::project::Project;

let project = Project::new("My Recording");
project.save().expect("Failed to save project");
```

### Starting a Recording

```rust
use frame_core::capture::{create_capture, CaptureConfig, CaptureArea};

let mut capture = create_capture()?;
let config = CaptureConfig {
    capture_area: CaptureArea::FullScreen,
    capture_cursor: true,
    capture_audio: true,
    frame_rate: 60,
};
capture.start(config).await?;
```

### Encoding Video

```rust
use frame_core::encoder::Encoder;
use std::path::Path;

let mut encoder = Encoder::new()?;
encoder.encode_frame(&frame)?;
encoder.encode_audio(&audio_buffer)?;
encoder.finalize(Path::new("output.mp4"))?;
```

---

## Feature Flags

### frame-core

- `default` - Enables `capture`
- `capture` - Screen/audio capture (requires platform-specific deps)
- `encoding` - Video encoding with ffmpeg
- `pro` - Pro tier features

### frame-desktop

- `default` - Basic features
- `pro` - Pro tier features (cloud sync, advanced export)

---

## Platform Support

| Platform    | Screen Capture   | Audio Capture         | Status       |
| ----------- | ---------------- | --------------------- | ------------ |
| macOS 12.3+ | ScreenCaptureKit | CoreAudio + BlackHole | ✅ Supported |
| Linux       | TBD              | TBD                   | 🚧 Planned   |
| Windows     | TBD              | TBD                   | 🚧 Planned   |

---

## Version Compatibility

- **Rust**: 1.75+
- **macOS**: 12.3+ (Monterey)
- **Bun**: 1.0+ (for tooling)

---

## See Also

- [Setup Guide](SETUP.md) - Installation and configuration
- [Contributing Guide](CONTRIBUTING.md) - How to contribute
- [Architecture](ARCHITECTURE.md) - System architecture (coming soon)
