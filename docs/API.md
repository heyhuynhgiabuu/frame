# Frame API Documentation

## Overview

Frame is a 100% Swift native macOS screen recorder. This document describes the public API for each module.

---

## App

### AppState

`@MainActor @Observable` singleton managing app-wide state.

```swift
@MainActor
@Observable
class AppState {
    var currentMode: AppMode           // .recorder, .editor, .export
    var isRecording: Bool              // Recording in progress
    var currentProject: Project?       // Active project
    var recordingConfig: RecordingConfig  // Capture settings
    var overlayManager: OverlayManager?  // Floating panel lifecycle
}
```

**Modes:**

```
Recorder → Recording → Editor → Export → Recorder
```

---

## Recording

### ScreenRecorder

SCStream-based screen capture. Manages the ScreenCaptureKit stream lifecycle.

```swift
class ScreenRecorder {
    func startCapture(config: RecordingConfig, webcamFrameProvider: (() -> CIImage?)?) async throws
    func stopCapture() async throws
    func updateContentFilter(_ filter: SCContentFilter) async throws
}
```

**Threading:** Uses separate `DispatchQueue` per output type (video, audio, microphone) at `.userInteractive` priority.

### WebcamCaptureEngine

AVCaptureSession-based webcam capture at 480p.

```swift
class WebcamCaptureEngine {
    var frameBox: WebcamFrameBox       // Thread-safe frame container
    var latestFrame: CIImage?          // Most recent camera frame

    func startCapture() throws
    func stopCapture()
}
```

**Key:** `alwaysDiscardsLateVideoFrames = true` prevents memory buildup.

### RecordingCoordinator

Orchestrates screen + webcam + audio recording.

```swift
class RecordingCoordinator {
    func startRecording() async throws  // Starts all capture engines
    func stopRecording() async throws   // Stops and finalizes
}
```

### CursorRecorder

Tracks mouse cursor position during recording.

```swift
class CursorRecorder {
    func startTracking()
    func stopTracking()
    var cursorPositions: [CursorPosition]
}
```

### KeystrokeRecorder

Records keyboard events during recording.

```swift
class KeystrokeRecorder {
    func startRecording()
    func stopRecording()
    var keystrokes: [Keystroke]
}
```

---

## Overlay

### FloatingPanel

Reusable `NSPanel` subclass for floating overlays.

```swift
class FloatingPanel: NSPanel {
    init(contentRect: NSRect, content: NSView)
    // - .nonactivatingPanel: Won't steal focus
    // - .fullScreenAuxiliary: Visible over fullscreen apps
    // - .canJoinAllSpaces: Visible on all Spaces
    // - sharingType = .none: Invisible to screen capture
}
```

### RecordingToolbarPanel

Frosted-glass toolbar at bottom-center with recording controls.

```swift
class RecordingToolbarPanel: FloatingPanel {
    // Idle: source picker, audio toggles, webcam toggle, record button
    // Recording: red dot, elapsed time, stop button
}
```

### WebcamPreviewPanel

Draggable floating panel showing live webcam feed.

```swift
class WebcamPreviewPanel: FloatingPanel {
    // GPU-backed CIImageView rendering
    // Rounded corners, shadow, draggable
}
```

### OverlayManager

Manages floating panel lifecycle.

```swift
@MainActor
class OverlayManager {
    func showPanels()                    // Show toolbar + webcam preview
    func hidePanels()                    // Dismiss all panels
    func allPanelWindows() -> [NSWindow] // For SCContentFilter exclusion
}
```

---

## Playback

### PlaybackEngine

AVPlayer-based video playback.

```swift
class PlaybackEngine {
    func load(url: URL) async throws
    func play()
    func pause()
    func seek(to time: CMTime) async
    var currentTime: CMTime { get }
    var duration: CMTime { get }
}
```

---

## Export

### ExportEngine

AVAssetWriter-based hardware-accelerated export.

```swift
class ExportEngine {
    func export(project: Project, config: ExportConfig) async throws
    var progress: Double { get }       // 0.0 to 1.0
}
```

**Threading:** All AVAssetWriter operations on a single serial queue.

### ExportConfig

```swift
struct ExportConfig {
    var format: ExportFormat          // .mp4, .mov, .gif
    var quality: Quality             // .low, .medium, .high
    var resolution: Resolution       // .original, .hd720, .hd1080
    var includeWebcam: Bool
    var webcamConfig: WebcamOverlayConfig?
}
```

---

## Effects

### ZoomEngine

Zoom/pan effects applied during export.

```swift
class ZoomEngine {
    func addZoomEffect(at time: CMTime, duration: CMTime, rect: CGRect)
    func applyEffects(to frame: CIImage, at time: CMTime) -> CIImage
}
```

---

## Models

### Project

Recording project containing metadata and references.

```swift
struct Project: Codable, Identifiable {
    let id: UUID
    var name: String
    var createdAt: Date
    var videoURL: URL?
    var duration: TimeInterval?
    var cursorData: [CursorPosition]
    var keystrokeData: [Keystroke]
}
```

### RecordingConfig

Capture settings.

```swift
struct RecordingConfig {
    var captureType: CaptureType      // .display or .window
    var frameRate: Int                 // Default: 30
    var showsCursor: Bool
    var captureSystemAudio: Bool
    var captureMicrophone: Bool
    var enableWebcam: Bool
    var quality: Quality
}
```

---

## Frameworks Used

| Framework         | Purpose                       | Min Version |
| ----------------- | ----------------------------- | ----------- |
| ScreenCaptureKit  | Screen capture                | macOS 13.0+ |
| AVFoundation      | Webcam, audio, video playback | macOS 13.0+ |
| AVAssetWriter     | Hardware-accelerated encoding | macOS 13.0+ |
| CoreImage / Metal | GPU effects, webcam rendering | macOS 13.0+ |
| CoreVideo         | CVDisplayLink, pixel buffers  | macOS 13.0+ |
| SwiftUI           | UI framework                  | macOS 13.0+ |
| AppKit            | NSPanel, NSWindow, NSEvent    | macOS 13.0+ |
| Combine           | Reactive data flow            | macOS 13.0+ |

---

## Permissions

| Permission       | Required For             | Prompt                        |
| ---------------- | ------------------------ | ----------------------------- |
| Screen Recording | ScreenCaptureKit capture | System Settings manual toggle |
| Camera           | Webcam capture           | System alert on first use     |
| Microphone       | Microphone audio         | System alert on first use     |

---

## See Also

- [Swift Best Practices](SWIFT-BEST-PRACTICES.md) — Comprehensive coding patterns and performance guidelines
- [README](README.md) — Getting started guide
