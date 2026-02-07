# Fix Recording Freeze When Clicking Record Now

**Bead:** bd-3cv  
**Created:** 2026-02-07  
**Status:** Approved

## Bead Metadata

```yaml
depends_on: []
parallel: true
conflicts_with: ["bd-317"] # Same recording/overlay files
blocks: []
estimated_hours: 2
```

---

## Problem Statement

### What problem are we solving?

When the user clicks "Record Now", the entire app freezes for 200ms–500ms+ (or indefinitely in some cases). The UI becomes unresponsive — no animations, no button feedback, nothing. Users perceive the app as crashed.

**Root cause:** Two blocking calls execute on the main thread (`@MainActor`):

1. `AVCaptureSession.startRunning()` at `WebcamCaptureEngine.swift:131` — synchronous call that blocks 200–500ms+ while the webcam hardware initializes
2. `SCStream.startCapture()` at `ScreenRecorder.swift:179` — awaited on `@MainActor`, blocking the main thread during ScreenCaptureKit setup

Both `WebcamCaptureEngine` and `ScreenRecorder` are marked `@MainActor`, forcing ALL their methods to run on the main thread. This violates Apple's documented best practice and the project's own AGENTS.md anti-pattern rules.

### Why now?

This is a **P0 critical bug**. The app is unusable for recording — its core function. Every user hits this on every recording attempt.

### Who is affected?

- **Primary users:** All users attempting to record (100% hit rate)
- **Secondary users:** Users with webcam enabled experience worse freeze (webcam + screen capture both block)

---

## Scope

### In-Scope

- Move `AVCaptureSession.startRunning()` off main thread to dedicated queue
- Move `AVCaptureSession.stopRunning()` off main thread to dedicated queue
- Move `SCStream.startCapture()` off main thread via `Task.detached`
- Ensure UI state updates still happen on `@MainActor` after background work completes
- Maintain existing recording pipeline correctness (timestamps, compositing, error handling)

### Out-of-Scope

- Refactoring OverlayManager panel creation (minor contributor, not root cause)
- Adding loading/progress indicators during recording startup
- Changing the webcam compositing pipeline
- Modifying RecordingCoordinator orchestration logic
- Adding new features or UI changes

---

## Proposed Solution

### Overview

Move the two blocking calls (`AVCaptureSession.startRunning()` and `SCStream.startCapture()`) off the main thread. `startRunning()` dispatches to the dedicated `outputQueue` already owned by `WebcamCaptureEngine`. `startCapture()` runs in a `Task.detached` context to avoid inheriting `@MainActor`. State updates (`isRunning`, `isRecording`) remain on `@MainActor` via explicit dispatch back.

### Fix Pattern

**WebcamCaptureEngine.start():**

```swift
// BEFORE (blocks main thread):
session.startRunning()
isRunning = true

// AFTER (runs on dedicated queue):
await withCheckedContinuation { continuation in
    outputQueue.async {
        session.startRunning()
        continuation.resume()
    }
}
isRunning = true  // Back on @MainActor
```

**WebcamCaptureEngine.stop():**

```swift
// BEFORE (blocks main thread):
captureSession?.stopRunning()

// AFTER (runs on dedicated queue):
if let session = captureSession {
    await withCheckedContinuation { continuation in
        outputQueue.async {
            session.stopRunning()
            continuation.resume()
        }
    }
}
```

**ScreenRecorder.startRecording():**

```swift
// BEFORE (blocks main actor):
try await captureStream.startCapture()

// AFTER (runs detached from main actor):
try await Task.detached(priority: .userInitiated) {
    try await captureStream.startCapture()
}.value
```

---

## Requirements

### Functional Requirements

#### FR-1: Non-blocking webcam start

The webcam capture session must start on a background queue, not the main thread.

**Scenarios:**

- **WHEN** user clicks "Record Now" with webcam enabled **THEN** UI remains responsive during webcam initialization (no freeze > 16ms on main thread)
- **WHEN** webcam hardware takes 500ms to initialize **THEN** app UI continues animating, button shows pressed state immediately
- **WHEN** webcam fails to start **THEN** error is propagated back to caller via async throw

#### FR-2: Non-blocking webcam stop

The webcam capture session must stop on a background queue, not the main thread.

**Scenarios:**

- **WHEN** user stops recording **THEN** UI remains responsive during webcam shutdown
- **WHEN** switching from recorder to editor mode **THEN** no freeze during webcam teardown

#### FR-3: Non-blocking screen capture start

`SCStream.startCapture()` must not block the main actor.

**Scenarios:**

- **WHEN** user clicks "Record Now" (even without webcam) **THEN** UI remains responsive during ScreenCaptureKit setup
- **WHEN** screen recording permission is denied **THEN** error is thrown and displayed without freezing
- **WHEN** `startCapture()` fails **THEN** cleanup runs and error propagates correctly

#### FR-4: Recording correctness preserved

All existing recording behavior must remain identical after the threading fix.

**Scenarios:**

- **WHEN** recording starts with webcam **THEN** webcam frames composite correctly into the video
- **WHEN** recording starts without webcam **THEN** screen-only recording works as before
- **WHEN** recording stops **THEN** AVAssetWriter finalizes correctly, video file is valid
- **WHEN** cursor/keystroke recorders start alongside **THEN** they still synchronize properly

### Non-Functional Requirements

- **Performance:** Main thread must never block for > 16ms (one frame at 60fps) during recording start/stop
- **Threading safety:** No data races — state transitions (`isRunning`, `isRecording`) must remain on `@MainActor`
- **Compatibility:** macOS 13.0+ (no API changes needed, all APIs already support background queues)

---

## Success Criteria

- [ ] App does NOT freeze when clicking "Record Now" with webcam enabled
  - Verify: Run app, enable webcam, click Record Now — UI stays responsive
- [ ] App does NOT freeze when clicking "Record Now" without webcam
  - Verify: Run app, disable webcam, click Record Now — UI stays responsive
- [ ] Recording produces valid video file with webcam overlay
  - Verify: Record 5s with webcam, play back the .mov file, webcam bubble visible
- [ ] Recording produces valid video file without webcam
  - Verify: Record 5s without webcam, play back the .mov file
- [ ] App does NOT freeze when stopping recording
  - Verify: Click stop — UI transitions to editor immediately
- [ ] Project builds with zero warnings
  - Verify: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build 2>&1 | grep -E "warning:|error:"`

---

## Technical Context

### Existing Patterns

- `WebcamCaptureEngine.outputQueue` (`DispatchQueue`, `.userInteractive`) — already exists for delegate callbacks, reuse for `startRunning()`
- `withCheckedContinuation` — standard Swift pattern for bridging DispatchQueue → async/await
- `Task.detached` — standard pattern for escaping `@MainActor` isolation

### Key Files

- `apps/desktop-swift/Frame/Recording/WebcamCaptureEngine.swift` — Contains blocking `session.startRunning()` on line 131 and `session.stopRunning()` on line 139
- `apps/desktop-swift/Frame/Recording/ScreenRecorder.swift` — Contains blocking `captureStream.startCapture()` on line 179
- `apps/desktop-swift/Frame/Recording/RecordingCoordinator.swift` — Orchestrator, calls both engines
- `apps/desktop-swift/Frame/App/AppState.swift` — Entry point for recording, calls coordinator

### Affected Files

Files this bead will modify (for conflict detection):

```yaml
files:
  - apps/desktop-swift/Frame/Recording/WebcamCaptureEngine.swift # Move startRunning/stopRunning off main thread
  - apps/desktop-swift/Frame/Recording/ScreenRecorder.swift # Move startCapture off main actor
```

---

## Risks & Mitigations

| Risk                                           | Likelihood | Impact | Mitigation                                                                      |
| ---------------------------------------------- | ---------- | ------ | ------------------------------------------------------------------------------- |
| Race condition on `isRunning` state            | Low        | Medium | State updates remain on @MainActor after background work completes              |
| Webcam `startRunning()` failure not propagated | Low        | Medium | Use `withCheckedContinuation` to bridge back; errors thrown before continuation |
| `Task.detached` loses error context            | Low        | Low    | `.value` rethrows; wrap in do/catch with existing `cleanupAfterFailedStart()`   |
| Breaking existing recording tests              | Low        | Medium | Run full build verification after changes                                       |

---

## Open Questions

None — root cause is clear, fix pattern is standard Swift concurrency.

---

## Tasks

### Move AVCaptureSession.startRunning off main thread [bug-fix]

`WebcamCaptureEngine.start()` dispatches `session.startRunning()` to the dedicated `outputQueue` using `withCheckedContinuation`, then updates `isRunning` on `@MainActor`.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: ["Move AVCaptureSession.stopRunning off main thread"]
files:
  - apps/desktop-swift/Frame/Recording/WebcamCaptureEngine.swift
```

**Verification:**

- Build succeeds: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`
- Run app with webcam enabled, click Record Now — no freeze
- Recording with webcam produces valid video

### Move AVCaptureSession.stopRunning off main thread [bug-fix]

`WebcamCaptureEngine.stop()` dispatches `session.stopRunning()` to the dedicated `outputQueue` using `withCheckedContinuation`, then updates state on `@MainActor`.

**Metadata:**

```yaml
depends_on: ["Move AVCaptureSession.startRunning off main thread"]
parallel: false
conflicts_with: ["Move AVCaptureSession.startRunning off main thread"]
files:
  - apps/desktop-swift/Frame/Recording/WebcamCaptureEngine.swift
```

**Verification:**

- Build succeeds
- Click stop recording — no freeze, transitions to editor smoothly

### Move SCStream.startCapture off main actor [bug-fix]

`ScreenRecorder.startRecording()` wraps `captureStream.startCapture()` in `Task.detached(priority: .userInitiated)` to avoid blocking the main actor, then updates state on `@MainActor`.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - apps/desktop-swift/Frame/Recording/ScreenRecorder.swift
```

**Verification:**

- Build succeeds
- Run app without webcam, click Record Now — no freeze
- Screen recording produces valid .mov file
- Error handling still works (deny permission → error alert, not freeze)

---

## Dependency Legend

| Field            | Purpose                                           | Example              |
| ---------------- | ------------------------------------------------- | -------------------- |
| `depends_on`     | Must complete before this task starts             | `["Setup database"]` |
| `parallel`       | Can run concurrently with other parallel tasks    | `true` / `false`     |
| `conflicts_with` | Cannot run in parallel (same files)               | `["Update config"]`  |
| `files`          | Files this task modifies (for conflict detection) | `["src/path.swift"]` |

---

## Notes

- The AGENTS.md for the Recording module already documents this anti-pattern: "Never call `startRunning()`/`stopRunning()` on main thread — use the session's dedicated queue"
- `WebcamCaptureEngine` already owns a dedicated `outputQueue` — perfect target for `startRunning()`
- `stop()` is currently synchronous (`func stop()`) — must become `func stop() async` to await the background work
- Changing `stop()` to async will require updating call sites in `AppState` (minor)
