# Frame Documentation

Comprehensive documentation for Frame — a 100% Swift screen recorder for developers.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Architecture Overview](#architecture-overview)
3. [Core Features](#core-features)
4. [Configuration](#configuration)
5. [Troubleshooting](#troubleshooting)

## Getting Started

### Prerequisites

- **macOS 13.0+** (Ventura) — ScreenCaptureKit required
- **Xcode 15.0+** — For building and running the app
- **Bun 1.0+** (optional) — For JS tooling (linting/formatting docs)

### Installation

```bash
# Clone the repository
git clone https://github.com/frame/frame.git
cd frame

# Open in Xcode
open apps/desktop-swift/Frame.xcodeproj

# Press ⌘R to build and run
```

Or build from command line:

```bash
xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build
```

### Quick Start Guide

1. **First Launch**: Grant screen recording and microphone permissions when prompted
2. **Start Recording**: Click the "Record" button or use Cmd+Shift+R
3. **Stop Recording**: Click "Stop" or use Cmd+Shift+S
4. **Preview**: Your recording automatically opens in the editor
5. **Export**: Click "Export" to save in your preferred format

## Architecture Overview

### Project Structure

```
apps/desktop-swift/
└── Frame/
    ├── App/
    │   ├── FrameApp.swift          # App entry point
    │   └── AppState.swift          # @Observable singleton, app-wide state
    ├── Recording/
    │   ├── ScreenRecorder.swift    # SCStream-based screen capture
    │   ├── WebcamCaptureEngine.swift # AVCaptureSession webcam
    │   ├── RecordingCoordinator.swift # Orchestrates all recording
    │   ├── CursorRecorder.swift    # Mouse tracking
    │   └── KeystrokeRecorder.swift # Keyboard events
    ├── Playback/
    │   ├── PlaybackEngine.swift    # AVPlayer-based playback
    │   └── VideoPlayerView.swift   # Video display
    ├── Export/
    │   ├── ExportEngine.swift      # AVAssetWriter encoding
    │   └── ExportConfig.swift      # Export settings
    ├── Overlay/
    │   ├── FloatingPanel.swift     # NSPanel base class
    │   ├── WebcamPreviewPanel.swift # GPU-backed webcam preview
    │   ├── RecordingToolbarPanel.swift # Recording controls
    │   └── OverlayManager.swift    # Panel lifecycle
    ├── Effects/
    │   └── ZoomEngine.swift        # Zoom/pan effects
    ├── Models/
    │   ├── Project.swift           # Recording project
    │   └── RecordingConfig.swift   # Capture settings
    ├── Utilities/
    │   └── Permissions.swift       # System permissions
    └── Views/                      # SwiftUI views
        ├── Editor/                 # Editor mode views
        ├── Recording/              # Recording mode views
        ├── Export/                  # Export views
        ├── Shared/                 # Shared components
        └── ContentView.swift       # Root view
```

### Technology Stack

| Component       | Framework         | Purpose                       |
| --------------- | ----------------- | ----------------------------- |
| UI              | SwiftUI + AppKit  | Native macOS interface        |
| Screen Capture  | ScreenCaptureKit  | macOS native screen recording |
| Webcam          | AVFoundation      | Camera capture                |
| Video Encoding  | AVAssetWriter     | Hardware-accelerated encoding |
| GPU Effects     | CoreImage + Metal | Visual effects, rendering     |
| Display Sync    | CoreVideo         | CVDisplayLink frame timing    |
| Reactive Data   | Combine           | Event streams, data binding   |
| Floating Panels | AppKit NSPanel    | Recording overlays            |

### Key Design Decisions

- **GPU-backed webcam preview**: Uses `CVDisplayLink` + `CIContext` to render `CIImage` directly to `CALayer`, bypassing the SwiftUI Image pipeline for smooth 30fps during recording
- **Floating panels**: `NSPanel` with `sharingType = .none` to exclude overlays from screen capture
- **@Observable AppState**: Single source of truth for app-wide state, with Combine bridges for @Published sources
- **WebcamFrameBox**: Thread-safe container for passing webcam frames between capture thread and compositing

## Core Features

### Screen Recording

Uses macOS ScreenCaptureKit for high-performance screen capture:

```swift
// ScreenRecorder manages SCStream
let recorder = ScreenRecorder()
try await recorder.startCapture(
    config: recordingConfig,
    webcamFrameProvider: { webcamEngine.frameBox.snapshot }
)
```

### Webcam Compositing

Screen + webcam frames are composited in real-time during recording:

```swift
// WebcamOverlayConfig controls pip appearance
let config = WebcamOverlayConfig(
    position: .bottomRight,
    size: 0.2,        // 20% of screen width
    shape: .circle,
    padding: 20
)
```

### Live Webcam Preview

During recording, a floating panel shows the live webcam feed using GPU-backed rendering:

```swift
// CIImageView renders directly via CVDisplayLink + CIContext
// No NSImage conversion needed — zero main thread impact
let preview = CIImageView(frameBox: webcamEngine.frameBox)
```

### Export

Hardware-accelerated encoding via AVAssetWriter:

```swift
let config = ExportConfig(
    format: .mp4,
    quality: .high,
    resolution: .original
)
try await exportEngine.export(project: project, config: config)
```

## Configuration

### Recording Configuration

```swift
struct RecordingConfig {
    var captureType: CaptureType   // .display or .window
    var frameRate: Int             // Default: 30
    var showsCursor: Bool          // Show cursor in recording
    var captureSystemAudio: Bool   // Capture system audio
    var captureMicrophone: Bool    // Capture microphone
    var quality: Quality           // .low, .medium, .high
}
```

### Permissions

Frame requires these macOS permissions:

- **Screen Recording** — System Settings → Privacy & Security → Screen Recording
- **Camera** — Granted via system prompt on first use
- **Microphone** — Granted via system prompt on first use

## Troubleshooting

### "Permission Denied" Error

1. Open System Settings → Privacy & Security → Screen Recording
2. Enable Frame
3. Restart Frame

### Webcam Not Showing

1. Check System Settings → Privacy & Security → Camera
2. Ensure no other app is using the camera
3. Try toggling webcam off/on in recording settings

### High CPU During Recording

1. Lower resolution or frame rate in settings
2. Close other GPU-intensive applications
3. The webcam preview uses GPU rendering and should have minimal CPU impact

### No Audio in Recording

1. Check microphone permissions
2. For system audio, ensure an audio loopback driver (like BlackHole) is installed
3. Check audio levels in Frame settings

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
