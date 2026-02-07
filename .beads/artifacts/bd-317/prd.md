# Floating Overlay Panels for Recording UI

**Bead:** bd-317  
**Created:** 2026-02-06  
**Status:** Draft

## Bead Metadata

```yaml
depends_on: []
parallel: true
conflicts_with: []
blocks: []
estimated_hours: 6
```

---

## Problem Statement

### What problem are we solving?

Frame's recording mode currently embeds all controls inside the main app window. Users cannot see their screen while recording — the controls overlay nothing and the webcam preview only appears in the editor after recording. Screen Studio solves this with floating overlay panels that sit on top of the desktop: a frosted-glass toolbar at the bottom with recording controls, and a draggable webcam preview bubble. This lets users see exactly what they're recording and control it without switching windows.

### Why now?

The recording infrastructure (ScreenRecorder, WebcamCaptureEngine, RecordingCoordinator) is complete and building. Adding overlay panels now means users get a polished, professional recording experience from day one.

### Who is affected?

- **Primary users:** Anyone recording their screen — they need to see controls and webcam preview while working in other apps
- **Secondary users:** Developers extending Frame — a reusable `FloatingPanel` base class benefits future overlay features

---

## Scope

### In-Scope

- Reusable `FloatingPanel` NSPanel subclass (borderless, non-activating, always-on-top, transparent)
- `RecordingToolbar` floating panel with glass effect, capture mode selector, audio toggles, webcam toggle, record/stop button
- `WebcamPreviewPanel` floating panel with live camera feed, draggable, rounded corners
- Exclude both panels from screen capture via `SCContentFilter(exceptingWindows:)`
- Wire panels into existing `RecordingCoordinator` and `AppState`

### Out-of-Scope

- Countdown timer before recording starts (future)
- Area selection overlay with drag handles (future)
- Webcam preview shape options (circle vs rounded rect — future, default to rounded rect)
- Audio level meters in the toolbar (future)
- Panel position persistence across app launches (future)

---

## Proposed Solution

### Overview

When the user enters recording mode, two floating `NSPanel` windows appear on top of all applications. The **Recording Toolbar** is a pill-shaped frosted-glass bar at the bottom-center of the screen (above the Dock) with capture controls. The **Webcam Preview** is a rounded-rectangle panel in the bottom-right showing a live camera feed, draggable to any screen position. Both panels are excluded from the screen capture stream so they don't appear in the recording. When recording stops, both panels dismiss.

### User Flow

1. User opens Frame and is in **Recorder mode** — the main window shows settings, floating toolbar appears at bottom-center of screen, webcam preview appears at bottom-right (if webcam is enabled)
2. User clicks **Start Recording** on the floating toolbar — recording begins, toolbar switches to show stop button + duration timer
3. User works in other apps — the toolbar and webcam preview remain visible on top
4. User clicks **Stop** on the toolbar — recording stops, panels dismiss, Frame switches to editor mode with the new recording

---

## Requirements

### Functional Requirements

#### FloatingPanel Base Class

A reusable `NSPanel` subclass that other overlay features can inherit from.

**Scenarios:**

- **WHEN** a FloatingPanel is created **THEN** it uses `.nonactivatingPanel` style, does not steal focus from the current app
- **WHEN** the user is in a fullscreen app **THEN** the panel remains visible (`.fullScreenAuxiliary` collection behavior)
- **WHEN** the user switches Spaces/desktops **THEN** the panel appears on all spaces (`.canJoinAllSpaces`)
- **WHEN** the panel background is set **THEN** it is transparent (`backgroundColor = .clear`) for custom shapes

#### Recording Toolbar

A frosted-glass floating bar with recording controls.

**Scenarios:**

- **WHEN** recorder mode is active **THEN** the toolbar appears at bottom-center of screen, 20pt above the Dock
- **WHEN** idle (not recording) **THEN** toolbar shows: capture mode picker (Display/Window), audio toggles (System Audio, Mic), webcam toggle, Start Recording button
- **WHEN** recording is active **THEN** toolbar shows: pulsing red dot, elapsed duration, Stop button
- **WHEN** the user clicks Start Recording **THEN** recording begins via `RecordingCoordinator.startRecording()`
- **WHEN** the user clicks Stop **THEN** recording stops and panels dismiss

#### Webcam Preview Panel

A draggable live webcam preview overlay.

**Scenarios:**

- **WHEN** webcam is enabled in settings **THEN** the webcam preview panel appears at bottom-right of screen
- **WHEN** the user drags the webcam panel **THEN** it moves freely to any position on screen
- **WHEN** recording is active **THEN** the webcam preview shows the live camera feed from `WebcamCaptureEngine`
- **WHEN** webcam is disabled **THEN** the panel is hidden

#### Self-Exclusion from Capture

The overlay panels must not appear in the recorded video.

**Scenarios:**

- **WHEN** screen recording starts **THEN** both floating panels are excluded via `SCContentFilter(exceptingWindows:)`
- **WHEN** display capture mode is used **THEN** panels are excluded from the `excludingApplications` or `exceptingWindows` filter
- **WHEN** window capture mode is used **THEN** panels are naturally excluded (only the target window is captured)

### Non-Functional Requirements

- **Performance:** Webcam preview must render at ≥15fps without dropping screen capture frames
- **Compatibility:** macOS 14.0+ (existing deployment target)
- **UX:** Panels must not interfere with normal app usage — clicking through to apps below must work except on panel UI elements

---

## Success Criteria

- [ ] FloatingPanel base class exists and is reusable for future overlays
  - Verify: Panel appears on top of all windows, doesn't steal focus, visible on all Spaces
- [ ] Recording toolbar shows at bottom-center with glass effect
  - Verify: `xcodebuild build` succeeds, toolbar visually matches frosted glass style
- [ ] Webcam preview shows live camera feed and is draggable
  - Verify: Camera feed renders in the panel, panel can be dragged to any position
- [ ] Overlay panels do NOT appear in the screen recording
  - Verify: Record a video, review output — no toolbar or webcam panel visible in recording
- [ ] Full build succeeds without errors
  - Verify: `cd apps/desktop-swift && xcodegen generate && xcodebuild build -project Frame.xcodeproj -scheme Frame -configuration Debug -destination 'platform=macOS'`

---

## Technical Context

### Existing Patterns

- `VisualEffectBackground` in `RecordingView.swift` — already bridges `NSVisualEffectView` to SwiftUI via `NSViewRepresentable`
- `WebcamCaptureEngine` — captures at 480p via `AVCaptureSession`, outputs `CIImage` to `latestFrame` property
- `ScreenRecorder.startRecording()` — uses `SCContentFilter(display:excludingApplications:exceptingWindows:)` — already has the exclusion parameter available

### Key Files

- `Frame/Recording/ScreenRecorder.swift` — `SCContentFilter` construction, needs `exceptingWindows` populated
- `Frame/Recording/RecordingCoordinator.swift` — Orchestrates start/stop, needs to manage overlay lifecycle
- `Frame/Recording/WebcamCaptureEngine.swift` — Webcam capture, `latestFrame: CIImage?`
- `Frame/App/AppState.swift` — Global state, manages mode transitions
- `Frame/Views/Recording/RecordingView.swift` — Current in-window recording UI
- `Frame/Views/Recording/SourcePicker.swift` — Capture source selection controls

### Affected Files

```yaml
files:
  - Frame/Overlay/FloatingPanel.swift # NEW — base NSPanel subclass
  - Frame/Overlay/RecordingToolbarPanel.swift # NEW — toolbar panel + SwiftUI content
  - Frame/Overlay/WebcamPreviewPanel.swift # NEW — webcam overlay panel
  - Frame/Overlay/OverlayManager.swift # NEW — manages panel lifecycle
  - Frame/Recording/ScreenRecorder.swift # MOD — add exceptingWindows to SCContentFilter
  - Frame/Recording/RecordingCoordinator.swift # MOD — wire overlay start/stop
  - Frame/App/AppState.swift # MOD — manage overlay state
  - apps/desktop-swift/project.yml # MOD — add new source files
```

---

## Risks & Mitigations

| Risk                                                                 | Likelihood | Impact | Mitigation                                                               |
| -------------------------------------------------------------------- | ---------- | ------ | ------------------------------------------------------------------------ |
| NSPanel from ScreenCaptureKit cannot be excluded by window reference | Low        | High   | Use `sharingType = .none` as fallback; test both approaches              |
| Webcam preview drops frames when screen recording is active          | Medium     | Medium | Use separate dispatch queue for webcam; keep preview resolution at 480p  |
| Floating panel steals keyboard focus from other apps                 | Medium     | High   | Use `.nonactivatingPanel` style mask; test with typing in other apps     |
| Panel doesn't appear above fullscreen apps                           | Low        | Medium | Set `.fullScreenAuxiliary` collection behavior and elevated window level |

---

## Open Questions

| Question                                                                                          | Owner  | Due Date              | Status                                                   |
| ------------------------------------------------------------------------------------------------- | ------ | --------------------- | -------------------------------------------------------- |
| Should we use `window.sharingType = .none` or `SCContentFilter(exceptingWindows:)` for exclusion? | Dev    | During implementation | Open                                                     |
| Should the webcam preview default to circle or rounded rectangle?                                 | Design | Before ship           | Resolved — rounded rectangle per Screen Studio reference |

---

## Tasks

### Create FloatingPanel base class [infrastructure]

A reusable `NSPanel` subclass exists at `Frame/Overlay/FloatingPanel.swift` with non-activating, borderless, always-on-top, transparent, multi-space behavior configured.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - Frame/Overlay/FloatingPanel.swift
```

**Verification:**

- File exists and compiles
- Panel can be instantiated with SwiftUI content via `NSHostingView`
- Panel has `.nonactivatingPanel`, `.fullScreenAuxiliary`, `.canJoinAllSpaces` configured

### Build RecordingToolbar overlay [ui]

A floating frosted-glass toolbar at bottom-center shows recording controls (idle: source picker, audio toggles, webcam toggle, record button; recording: red dot, timer, stop button).

**Metadata:**

```yaml
depends_on: ["Create FloatingPanel base class"]
parallel: false
conflicts_with: ["Build WebcamPreviewPanel overlay"]
files:
  - Frame/Overlay/RecordingToolbarPanel.swift
  - Frame/Views/Recording/SourcePicker.swift
```

**Verification:**

- Toolbar renders with `NSVisualEffectView` glass background
- Shows correct controls for idle vs recording states
- Positioned at bottom-center of screen, above Dock

### Build WebcamPreviewPanel overlay [ui]

A draggable floating panel at bottom-right shows live webcam feed from `WebcamCaptureEngine.latestFrame` in a rounded rectangle with shadow.

**Metadata:**

```yaml
depends_on: ["Create FloatingPanel base class"]
parallel: false
conflicts_with: ["Build RecordingToolbar overlay"]
files:
  - Frame/Overlay/WebcamPreviewPanel.swift
  - Frame/Recording/WebcamCaptureEngine.swift
```

**Verification:**

- Panel shows live camera feed
- Panel is draggable to any position
- Panel has rounded corners and shadow
- Panel hides when webcam is disabled

### Create OverlayManager and wire into AppState [wiring]

An `OverlayManager` class manages the lifecycle of toolbar and webcam panels, showing/hiding them based on recording mode. AppState owns the OverlayManager and triggers show/hide on mode transitions.

**Metadata:**

```yaml
depends_on: ["Build RecordingToolbar overlay", "Build WebcamPreviewPanel overlay"]
parallel: false
conflicts_with: []
files:
  - Frame/Overlay/OverlayManager.swift
  - Frame/App/AppState.swift
  - Frame/Recording/RecordingCoordinator.swift
  - apps/desktop-swift/project.yml
```

**Verification:**

- Panels appear when entering recorder mode
- Panels dismiss when switching to editor mode
- Start/Stop recording buttons on toolbar trigger `RecordingCoordinator`
- All new files added to `project.yml`

### Exclude overlay panels from screen capture [capture]

`ScreenRecorder` excludes both floating panels from the `SCContentFilter` so they don't appear in recordings. Falls back to `sharingType = .none` if needed.

**Metadata:**

```yaml
depends_on: ["Create OverlayManager and wire into AppState"]
parallel: false
conflicts_with: []
files:
  - Frame/Recording/ScreenRecorder.swift
  - Frame/Overlay/OverlayManager.swift
```

**Verification:**

- Record a screen capture with panels visible
- Review the output video — no toolbar or webcam panel visible
- Both `sharingType = .none` and `SCContentFilter` exclusion paths work

### Build verification [verification]

Full project builds and all panels work end-to-end.

**Metadata:**

```yaml
depends_on: ["Exclude overlay panels from screen capture"]
parallel: false
conflicts_with: []
files: []
```

**Verification:**

- `xcodegen generate && xcodebuild build` succeeds
- App launches, recorder mode shows floating panels
- Recording produces clean video without overlay artifacts

---

## Dependency Legend

| Field            | Purpose                                           | Example                                    |
| ---------------- | ------------------------------------------------- | ------------------------------------------ |
| `depends_on`     | Must complete before this task starts             | `["Setup database", "Create schema"]`      |
| `parallel`       | Can run concurrently with other parallel tasks    | `true` / `false`                           |
| `conflicts_with` | Cannot run in parallel (same files)               | `["Update config"]`                        |
| `files`          | Files this task modifies (for conflict detection) | `["src/db/schema.ts", "src/db/client.ts"]` |

---

## Notes

- Research from Screen Studio analysis confirms: uses separate `NSPanel` windows, not a single full-screen overlay
- `NSPanel` with `.nonactivatingPanel` is the standard macOS pattern for floating toolbars (used by Sketch, Figma, etc.)
- `SCContentFilter(exceptingWindows:)` is the modern ScreenCaptureKit approach; `sharingType = .none` is the legacy fallback
- Webcam feed reuses existing `WebcamCaptureEngine` — no new capture code needed, just a new display surface
- The existing `SourcePicker.swift` controls can be extracted/reused in the floating toolbar
