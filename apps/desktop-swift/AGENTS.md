# Frame Desktop (Swift/macOS)

**Generated:** 2026-02-07 | **Commit:** 5502673 | **Branch:** feat/bd-317-floating-overlay-panels

## Overview

Native macOS screen recorder with WYSIWYG editor. Records screen + webcam + cursor + keystrokes, then composites effects (background, padding, shadows, zoom) for export. SwiftUI app with AppKit overlay panels.

## Structure

```
Frame/
├── App/              # Entry point, global state (AppState singleton)
├── Recording/        # SCStream capture, webcam, cursor, keystroke recording
├── Overlay/          # Floating NSPanel overlays (toolbar, webcam preview)
├── Playback/         # AVPlayer-based video playback
├── Export/           # AVAssetReader→Writer effects pipeline
├── Effects/          # Zoom engine
├── Models/           # Project, RecordingConfig, EffectsConfig
├── Views/
│   ├── ContentView   # Mode router (recorder ↔ editor)
│   ├── Recording/    # RecordingView, SourcePicker
│   ├── Editor/       # EditorView, PreviewCanvas, overlays, Inspector/*
│   ├── Export/       # ExportView
│   └── Shared/       # ToolbarItems
└── Utilities/        # PermissionsManager
```

## Where to Look

| Task                      | Location                         | Notes                                                 |
| ------------------------- | -------------------------------- | ----------------------------------------------------- |
| Add recording feature     | `Recording/RecordingCoordinator` | Orchestrates all recorders                            |
| Modify screen capture     | `Recording/ScreenRecorder`       | SCStream + AVAssetWriter                              |
| Change webcam behavior    | `Recording/WebcamCaptureEngine`  | AVCaptureSession, thread-safe `WebcamFrameBox`        |
| Add/edit floating panel   | `Overlay/FloatingPanel`          | Generic NSPanel base class                            |
| Change toolbar controls   | `Overlay/RecordingToolbarPanel`  | SwiftUI content inside FloatingPanel                  |
| Modify export effects     | `Export/ExportEngine`            | CIImage compositing pipeline                          |
| Add inspector tab         | `App/AppState.InspectorTab`      | Then add inspector view in `Views/Editor/Inspector`   |
| Edit preview rendering    | `Views/Editor/PreviewCanvas`     | WYSIWYG with all overlays                             |
| Change app lifecycle/mode | `App/AppState.handleModeChange`  | recorder ↔ editor transitions                         |
| Add keyboard shortcut     | `App/FrameApp`                   | CommandMenu declarations                              |
| Video playback            | `Playback/PlaybackEngine`        | AVPlayer wrapper                                      |
| Data models               | `Models/`                        | Project, RecordingConfig, EffectsConfig, ExportConfig |
| Permission checks         | `Utilities/PermissionsManager`   | Screen recording + camera permissions                 |

## Code Map

| Symbol                   | Type  | Location                 | Role                                           |
| ------------------------ | ----- | ------------------------ | ---------------------------------------------- |
| `AppState`               | class | App/AppState.swift       | @Observable singleton, bridges all engines     |
| `RecordingCoordinator`   | class | Recording/               | Orchestrates screen+cursor+keystroke recording |
| `ScreenRecorder`         | class | Recording/               | SCStream → AVAssetWriter pipeline              |
| `WebcamCaptureEngine`    | class | Recording/               | AVCaptureSession → WebcamFrameBox              |
| `WebcamFrameBox`         | class | Recording/               | NSLock-guarded thread-safe frame sharing       |
| `FloatingPanel<Content>` | class | Overlay/                 | Generic non-activating NSPanel base            |
| `OverlayManager`         | class | Overlay/                 | Toolbar + webcam panel lifecycle               |
| `ExportEngine`           | class | Export/                  | AVAssetReader→Writer with CIImage effects      |
| `PlaybackEngine`         | class | Playback/                | AVPlayer wrapper                               |
| `ZoomEngine`             | class | Effects/                 | Zoom/pan state                                 |
| `RecordingStreamOutput`  | class | Recording/ScreenRecorder | SCStreamOutput delegate, webcam compositing    |

## Conventions

- **ObservableObject→@Observable bridge**: AppState uses `_observationBump` counter + Combine subscriptions to forward changes from ObservableObject engines
- **WindowAccessor**: NSViewRepresentable captures NSWindow ref; the only reliable way to get the hosting window from SwiftUI
- **AppDelegate**: Returns false for `applicationShouldTerminateAfterLastWindowClosed` — required because main window hides in recorder mode
- **WebcamFrameBox**: Thread-safe CIImage sharing between capture thread and recording pipeline via NSLock
- **Sidecar files**: Cursor events saved as `.cursor.json`, keystrokes as `.keystrokes.json` alongside `.mov` video

## Anti-Patterns

- **Never** use `!` force unwrap in production code
- **Never** forget `sharingType = .none` on floating panels (they appear in recordings otherwise)
- **Never** use `@StateObject` for AppState — it's `@Observable`, use `@Environment`

See subdirectory AGENTS.md files for module-specific anti-patterns (Recording, Overlay, Export, Views).

## Threading Model

```
SCStream video/audio    → .global(qos: .userInitiated) — shared global queue
AVCaptureSession webcam → dedicated queue ("com.frame.webcam-output", .userInteractive)
AVAssetWriter appends   → inline on SCStream callback queue
CVDisplayLink           → callback thread → main thread for layer.contents
UI / @Observable        → @MainActor (main thread)
```

See `Recording/AGENTS.md` for detailed threading constraints.

## Commands

```bash
# Build
xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build

# Dev (open in Xcode, ⌘R)
open apps/desktop-swift/Frame.xcodeproj
```

## Notes

- macOS 13.0+ required (ScreenCaptureKit)
- Screen recording permission requires app restart after first grant
- Webcam compositing uses 0.15s grace window for frame staleness during recording
- Webcam preview: CVDisplayLink + CIContext GPU rendering bypasses SwiftUI render cycle
- Export supports MOV (ProRes 422) and MP4 (H.264); GIF defined in ExportConfig but not yet implemented in ExportEngine
- Recording saves to `~/Movies/Frame Recordings/`
