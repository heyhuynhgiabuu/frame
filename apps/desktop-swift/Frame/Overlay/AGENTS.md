# Overlay

Floating NSPanel system for recording-mode UI that sits above all windows without stealing focus.

## Where to Look

| Task                | File                    | Notes                                           |
| ------------------- | ----------------------- | ----------------------------------------------- |
| Base panel behavior | `FloatingPanel.swift`   | Generic `NSPanel` subclass — all panels inherit |
| Panel lifecycle     | `OverlayManager.swift`  | Shows/hides toolbar + webcam based on app mode  |
| Toolbar controls    | `RecordingToolbarPanel` | Source picker, audio/webcam toggles, start/stop |
| Webcam preview      | `WebcamPreviewPanel`    | 200×150 live feed, draggable, GPU-rendered      |

## Architecture

```
OverlayManager (@MainActor, owned by AppState)
├── RecordingToolbarPanel  → FloatingPanel<RecordingToolbarContent>
└── WebcamPreviewPanel     → FloatingPanel<WebcamPreviewContent>
                                └── CIImageView → WebcamLayerView (CVDisplayLink)
```

## FloatingPanel Configuration

| Property                      | Value                                               | Why                                     |
| ----------------------------- | --------------------------------------------------- | --------------------------------------- |
| `styleMask`                   | `.borderless, .nonactivatingPanel`                  | No title bar, doesn't steal focus       |
| `level`                       | `.floating`                                         | Always on top                           |
| `collectionBehavior`          | `canJoinAllSpaces, fullScreenAuxiliary, stationary` | Visible on all Spaces + fullscreen apps |
| `sharingType`                 | `.none`                                             | **Self-excludes from ScreenCaptureKit** |
| `canBecomeKey`                | `true`                                              | SwiftUI buttons receive clicks          |
| `canBecomeMain`               | `false`                                             | Never becomes the "main" window         |
| `isMovableByWindowBackground` | `true` (toolbar & webcam)                           | Panels are draggable                    |

## GPU-Backed Webcam Preview

`WebcamPreviewPanel` uses `CIImageView` (NSViewRepresentable) → `WebcamLayerView`:

1. **CVDisplayLink** fires at display refresh rate
2. Reads latest `WebcamFrameSnapshot` from `WebcamFrameBox`
3. Skips if timestamp unchanged (dedup)
4. `CIContext.createCGImage()` renders CIImage → CGImage (GPU)
5. Assigns to `layer.contents` on main thread with `CATransaction.setDisableActions(true)` (no implicit animation)
6. Display link started/stopped on `viewDidMoveToWindow` — **always stopped in deinit**

## Anti-Patterns

- **Never** forget `sharingType = .none` — panels will appear in screen recordings
- **Never** create CIContext per-frame in WebcamLayerView — create once, reuse (thread-safe)
- **Never** start CVDisplayLink without stopping in deinit — dangling callbacks crash
- **Never** use SwiftUI `Image(nsImage:)` for live webcam preview — main thread bottleneck during recording; use CIImageView instead

## Key Details

- Toolbar uses `NSVisualEffectView` with `.hudWindow` material for frosted-glass appearance
- `FirstMouseHostingView`: NSHostingView subclass with `acceptsFirstMouse = true` — buttons respond on first click without requiring panel activation
- `ToolbarPulseAnimation`: opacity 1.0→0.4 + scale 1.0→0.85, 0.8s repeat (recording indicator)
- `OverlayManager.overlayWindows` provides NSWindow refs for SCContentFilter exclusion
