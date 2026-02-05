# Frame Documentation

Comprehensive documentation for Frame screen recorder.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Architecture Overview](#architecture-overview)
3. [Core Features](#core-features)
4. [API Reference](#api-reference)
5. [Error Handling](#error-handling)
6. [Configuration](#configuration)
7. [Development](#development)
8. [Troubleshooting](#troubleshooting)

## Getting Started

### Prerequisites

- **macOS**: 12.3+ (Monterey or later)
- **Rust**: 1.75+
- **Bun**: 1.0+ (for web components)
- **Xcode**: Command Line Tools (for macOS development)

### Installation

```bash
# Clone the repository
git clone https://github.com/frame/frame.git
cd frame

# Install dependencies
bun install

# Build the project
cargo build --release

# Run the desktop app
cd apps/desktop && cargo run
```

### Quick Start Guide

1. **First Launch**: Grant screen recording and microphone permissions when prompted
2. **Start Recording**: Click the "Record" button or use Cmd+Shift+R
3. **Stop Recording**: Click "Stop" or use Cmd+Shift+S
4. **Preview**: Your recording will automatically open in the preview window
5. **Export**: Click "Export" to save in your preferred format

## Architecture Overview

### Project Structure

```
frame/
├── apps/
│   ├── desktop/          # Main iced.rs application
│   │   ├── src/
│   │   │   ├── app.rs           # Main application state
│   │   │   ├── recording/       # Recording service
│   │   │   └── ui/              # UI components
│   │   └── Cargo.toml
│   └── web/              # Web viewer/sharing (SolidJS)
├── packages/
│   ├── core/             # Shared Rust library
│   │   ├── src/
│   │   │   ├── capture/         # Screen/audio capture
│   │   │   ├── encoder.rs       # Video encoding
│   │   │   ├── error.rs         # Error handling
│   │   │   ├── project.rs       # Project management
│   │   │   └── auto_save.rs     # Auto-save functionality
│   │   └── Cargo.toml
│   ├── ui-components/    # Reusable iced.rs components
│   │   ├── src/
│   │   │   ├── button.rs
│   │   │   ├── error_dialog.rs
│   │   │   ├── export_dialog.rs
│   │   │   ├── timeline.rs
│   │   │   └── ...
│   │   └── Cargo.toml
│   └── renderer/         # GPU-accelerated rendering
└── Cargo.toml            # Workspace configuration
```

### Technology Stack

| Component        | Technology       | Purpose                       |
| ---------------- | ---------------- | ----------------------------- |
| Desktop App      | Rust + iced.rs   | Native UI with async support  |
| Screen Capture   | screencapturekit | macOS native screen recording |
| Audio Capture    | cpal             | Cross-platform audio input    |
| Video Encoding   | ffmpeg-sidecar   | H.264/H.265 encoding          |
| Audio Processing | rubato           | Sample rate conversion        |
| State Management | iced.rs reactive | UI state management           |
| Error Handling   | thiserror        | Structured error types        |

## Core Features

### Screen Recording

Frame uses macOS's native ScreenCaptureKit for high-performance screen capture:

```rust
use frame_core::capture::{CaptureConfig, create_capture};

let mut capture = create_capture()?;
capture.start(CaptureConfig {
    capture_area: CaptureArea::FullScreen,
    capture_audio: true,
    frame_rate: 30,
}).await?;

// Capture frames
while let Some(frame) = capture.next_frame().await? {
    // Process frame
}
```

### Audio Capture

Supports both microphone and system audio capture:

```rust
use frame_core::capture::microphone::MicrophoneCapture;

let mut mic = MicrophoneCapture::new(AudioConfig::default())?;
mic.start()?;

// Get audio buffers
while let Some(buffer) = mic.next_buffer().await? {
    // Process audio
}
```

### Video Encoding

Hardware-accelerated encoding via VideoToolbox:

```rust
use frame_core::encoder::{Encoder, EncoderConfig, VideoCodec};

let mut encoder = Encoder::new(EncoderConfig {
    codec: VideoCodec::H264,
    hardware_accelerated: true,
    ..Default::default()
})?;

encoder.init(&output_path)?;
encoder.encode_frame(&frame)?;
encoder.finalize()?;
```

### Auto-Save

Automatic project state persistence during recording:

```rust
use frame_core::auto_save::{AutoSaveConfig, AutoSaveService};

let mut service = AutoSaveService::with_config(AutoSaveConfig {
    interval: Duration::from_secs(10),
    enabled: true,
});

// Auto-saves every 10 seconds during recording
service.start_project("My Recording").await?;
```

### Project Management

Projects organize recordings, exports, and metadata:

```rust
use frame_core::{Project, Recording};

let mut project = Project::new("Tutorial Video");
project.recordings.push(Recording {
    id: "rec-001".to_string(),
    started_at: Utc::now(),
    duration_ms: 30000,
    file_path: PathBuf::from("/path/to/video.mp4"),
    has_video: true,
    has_audio: true,
    resolution: Resolution::Hd1080,
    frame_rate: 30,
});

project.save()?;
```

## API Reference

### Core Types

#### `FrameError`

Comprehensive error type with recovery actions:

```rust
pub enum FrameError {
    Io(String),
    CaptureError(String),
    EncodingError(String),
    AudioError(String),
    PermissionDenied(String),
    ResourceExhausted(String),
    // ... and more
}
```

Methods:

- `is_recoverable() -> bool` - Check if error can be retried
- `severity() -> ErrorSeverity` - Get severity level
- `recovery_action() -> Option<RecoveryAction>` - Get suggested recovery

#### `RecordingService`

High-level recording API:

```rust
impl RecordingService {
    pub async fn start_recording(&mut self, config: RecordingConfig) -> FrameResult<String>;
    pub async fn stop_recording(&mut self) -> FrameResult<(String, PathBuf)>;
    pub fn frame_count(&self) -> u64;
    pub fn is_recording(&self) -> bool;

    // Recovery
    pub fn check_for_incomplete_recordings() -> FrameResult<Vec<PathBuf>>;
    pub fn recover_incomplete_recording(path: &PathBuf) -> FrameResult<Option<PathBuf>>;
}
```

#### `ExportDialog`

UI component for export configuration:

```rust
pub struct ExportDialog {
    pub config: ExportConfig,
    pub is_open: bool,
}

impl ExportDialog {
    pub fn open(&mut self);
    pub fn close(&mut self);
    pub fn update(&mut self, message: ExportDialogMessage);
    pub fn view(&self) -> Element<ExportDialogMessage>;
}
```

### Configuration

#### Recording Configuration

```rust
pub struct RecordingConfig {
    pub capture_area: CaptureArea,      // FullScreen, Window, or Region
    pub capture_audio: bool,            // Record microphone
    pub frame_rate: u32,                // 15-60 fps
    pub output_path: PathBuf,           // Where to save
}
```

#### Export Configuration

```rust
pub struct ExportConfig {
    pub format: ExportFormat,           // MP4, GIF, WebM
    pub quality: ExportQuality,         // Low, Medium, High, Maximum
    pub resolution: ExportResolution,   // Original, 1080p, 720p, 480p
    pub fps: u32,                       // 15-60 fps
    pub filename: String,
}
```

## Error Handling

Frame provides comprehensive error handling with user-friendly recovery suggestions.

### Error Severity Levels

- **Info**: Informational messages (e.g., user cancelled)
- **Warning**: Non-fatal issues (e.g., audio device unavailable)
- **Error**: Operation failed (e.g., encoding failed)
- **Critical**: App may need restart

### Recovery Actions

When errors occur, Frame suggests appropriate actions:

1. **Retry** - For transient failures (network, encoding)
2. **RequestPermissions** - For permission issues
3. **OpenSettings** - For configuration issues
4. **FreeDiskSpace** - For disk space errors
5. **SaveAndRestart** - For critical errors
6. **Ignore** - For non-critical warnings

### Error Dialog

```rust
use frame_ui::error_dialog::ErrorDialog;

let mut dialog = ErrorDialog::new();
dialog.open(FrameError::PermissionDenied(
    "screen recording".to_string()
));

// In your view
if dialog.is_open() {
    content = content.push(dialog.view().map(Message::ErrorDialog));
}
```

## Configuration

### App Configuration

Configuration is stored in `~/.config/frame/config.json`:

```json
{
  "recording": {
    "default_frame_rate": 30,
    "default_quality": "high",
    "auto_save_interval": 10,
    "capture_audio": true
  },
  "export": {
    "default_format": "mp4",
    "default_resolution": "1080p"
  },
  "ui": {
    "theme": "dark",
    "show_tips": true
  }
}
```

### Environment Variables

- `FRAME_LOG_LEVEL` - Logging level (trace, debug, info, warn, error)
- `FRAME_DATA_DIR` - Override data directory path
- `FFMPEG_PATH` - Custom FFmpeg binary path

## Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run specific test
cargo test test_name -- --nocapture
```

### Testing

Run integration tests:

```bash
# All tests
cargo test

# Core library tests
cargo test -p frame-core

# Desktop app tests
cargo test -p frame-desktop

# UI component tests
cargo test -p frame-ui
```

### Code Organization

- Follow Rust naming conventions
- Use `thiserror` for error types
- Document public APIs with rustdoc
- Keep functions under 50 lines when possible
- Use feature flags for optional functionality

### Adding Features

1. Add feature flag to `Cargo.toml`
2. Implement behind `#[cfg(feature = "...")]`
3. Add tests for the feature
4. Update documentation

## Troubleshooting

### Common Issues

#### "Permission Denied" Error

**Cause**: Screen recording or microphone permission not granted

**Solution**:

1. Open System Preferences → Security & Privacy → Privacy
2. Enable "Screen Recording" for Frame
3. Enable "Microphone" for Frame
4. Restart Frame

#### "FFmpeg Not Found" Error

**Cause**: FFmpeg binary not available or not in PATH

**Solution**:

```bash
# Install FFmpeg
brew install ffmpeg

# Or set custom path
export FFMPEG_PATH=/usr/local/bin/ffmpeg
```

#### "Disk Full" Error

**Cause**: Insufficient disk space for recording

**Solution**:

1. Check available space: `df -h`
2. Clear temp files: `rm -rf ~/Library/Caches/frame/*`
3. Change output directory in settings

#### High CPU Usage

**Cause**: Recording at high resolution/frame rate without hardware acceleration

**Solution**:

1. Lower resolution (1080p → 720p)
2. Reduce frame rate (60fps → 30fps)
3. Ensure hardware acceleration is enabled
4. Close other applications

### Debug Mode

Enable detailed logging:

```bash
RUST_LOG=debug cargo run
```

Or in the app:

```bash
export FRAME_LOG_LEVEL=trace
./frame
```

### Reporting Issues

When reporting issues, please include:

1. Frame version (`frame --version`)
2. macOS version
3. Steps to reproduce
4. Log output with `RUST_LOG=debug`
5. Hardware specs (for performance issues)

### Recovery Mode

If Frame crashes during recording, check for incomplete projects:

```bash
# List incomplete recordings
ls ~/Library/Application\ Support/frame/projects/

# Recovery is automatic on next launch
```

## Performance Tips

1. **Use Hardware Acceleration**: Enable in preferences for best performance
2. **Lower Resolution**: Record at 1080p instead of 4K
3. **Reduce Frame Rate**: 30fps is sufficient for most content
4. **Close Apps**: Free up CPU/RAM before recording
5. **External Storage**: Record to SSD for best write performance

## License

- **Core**: MIT/Apache-2.0
- **Pro Features**: Commercial license required

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.
