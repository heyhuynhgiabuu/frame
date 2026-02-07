# Frame

Open-core screen recorder for developers. 100% Swift (macOS native) desktop app.

## Tech Stack

- **Language:** Swift 5.9+ / SwiftUI / AppKit
- **Platform:** macOS 13.0+ (Ventura)
- **Build:** Xcode 15.0+
- **JS Tooling:** Bun 1.0+, Biome 1.5.3 (formatting only)

## Structure

```
apps/
└── desktop-swift/        # Swift/macOS native app (Xcode project)
    └── Frame/
        ├── App/          # App entry point, AppState
        ├── Recording/    # Screen, webcam, cursor, keystroke capture
        ├── Playback/     # Video playback engine
        ├── Export/       # Export config & engine
        ├── Overlay/      # Floating panels (toolbar, webcam preview)
        ├── Effects/      # Zoom engine, visual effects
        ├── Models/       # Data models (Project, RecordingConfig)
        ├── Utilities/    # Permissions, helpers
        └── Views/        # SwiftUI views (Editor, Recording, Export)
docs/                     # Documentation
```

## Commands

**Dev:** Open `apps/desktop-swift/Frame.xcodeproj` in Xcode, ⌘R
**Build:** `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`
**Lint (JS):** `bun run lint`
**Format (JS):** `bun run format`

## Key Modules

| Module                           | Purpose                                      |
| -------------------------------- | -------------------------------------------- |
| `Recording/ScreenRecorder`       | SCStream-based screen capture                |
| `Recording/WebcamCaptureEngine`  | AVCaptureSession webcam with frameBox        |
| `Recording/RecordingCoordinator` | Orchestrates screen + webcam recording       |
| `Recording/CursorRecorder`       | Mouse cursor position tracking               |
| `Recording/KeystrokeRecorder`    | Keyboard event recording                     |
| `Overlay/FloatingPanel`          | NSPanel base for floating overlays           |
| `Overlay/WebcamPreviewPanel`     | Live webcam preview (GPU-backed CIImageView) |
| `Overlay/RecordingToolbarPanel`  | Recording controls toolbar                   |
| `Overlay/OverlayManager`         | Manages floating panels lifecycle            |
| `Export/ExportEngine`            | AVAssetWriter-based export                   |
| `Playback/PlaybackEngine`        | AVPlayer-based playback                      |
| `Effects/ZoomEngine`             | Zoom/pan effects                             |
| `App/AppState`                   | @Observable singleton, app-wide state        |

## Frameworks Used

| Framework         | Purpose                       |
| ----------------- | ----------------------------- |
| ScreenCaptureKit  | Screen capture (macOS 13.0+)  |
| AVFoundation      | Webcam, audio, video playback |
| AVAssetWriter     | Hardware-accelerated encoding |
| CoreImage / Metal | GPU effects, webcam rendering |
| CoreVideo         | CVDisplayLink, pixel buffers  |
| SwiftUI           | UI framework                  |
| AppKit            | NSPanel, NSWindow, NSEvent    |
| Combine           | Reactive data flow            |

## Boundaries

✅ **Always:** Test in Xcode before commit, use Swift error handling (throws/Result)
⚠️ **Ask first:** New SPM dependencies, new entitlements, Info.plist changes
🚫 **Never:** Force unwrap (`!`) in production code, commit build artifacts, skip permission checks

## Swift Coding Patterns

See **[docs/SWIFT-BEST-PRACTICES.md](docs/SWIFT-BEST-PRACTICES.md)** for comprehensive reference with code examples.

### Critical Rules

| Rule                                                               | Why                                                          |
| ------------------------------------------------------------------ | ------------------------------------------------------------ |
| Separate DispatchQueue per SCStream output type                    | Shared queues cause priority inversion & frame drops         |
| Serial queue for all AVAssetWriter operations                      | `finishWriting` concurrent with `appendSampleBuffer` crashes |
| `alwaysDiscardsLateVideoFrames = true` on webcam                   | Prevents memory buildup during live preview                  |
| `beginConfiguration()`/`commitConfiguration()` on AVCaptureSession | Atomic configuration changes                                 |
| `@MainActor` on all `@Observable` state classes                    | Thread safety for UI state                                   |
| `sharingType = .none` on floating panels                           | Automatic exclusion from screen capture                      |
| Never force unwrap (`!`)                                           | Use `guard let` or `if let`                                  |
| Create `CIContext` once, reuse                                     | Expensive initialization; thread-safe to share               |
| Stop `CVDisplayLink` in `deinit`                                   | Prevents dangling callbacks                                  |
| Check `isReadyForMoreMediaData` before writing                     | Backpressure signal from AVAssetWriter                       |

### Threading Model

```
SCStream video    → dedicated queue (.userInteractive)
SCStream audio    → dedicated queue (.userInteractive)
SCStream mic      → dedicated queue (.userInteractive)
AVCaptureSession  → dedicated queue (.userInitiated)
AVAssetWriter     → serial queue (.userInitiated)
CVDisplayLink     → callback thread → dispatch to main
UI / @Observable  → @MainActor (main thread)
```

## Gotchas

- macOS only (ScreenCaptureKit requires 13.0+)
- Screen recording permission must be granted in System Settings
- Webcam preview uses CVDisplayLink + CIContext for GPU-backed rendering (not SwiftUI Image)
- FloatingPanel uses `sharingType = .none` to exclude from screen capture
- Actors guarantee data race safety but NOT atomicity across `await` suspension points
- `SCContentFilter` can be updated dynamically via `stream.updateContentFilter()` — no restart needed
- AVCaptureSession `startRunning()`/`stopRunning()` must be called on the session queue, not main thread
