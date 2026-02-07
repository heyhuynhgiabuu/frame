# Webcam Mirror + Toolbar Observation Fix Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use skill({ name: "executing-plans" }) to implement this plan task-by-task.

**Goal:** Fix webcam mirroring (horizontal flip) and toolbar icon observation updates in the SwiftUI app.

**Architecture:**

- AppState uses `@Observable` macro (new observation)
- RecordingCoordinator and WebcamCaptureEngine use `ObservableObject` + `@Published` (old observation)
- Views need explicit observation triggers to bridge between the two systems
- Webcam frames need X-axis flip for natural mirror effect, not just Y-axis flip

**Tech Stack:** Swift, SwiftUI, AVFoundation, CoreImage

---

## Root Cause Analysis

### Issue 1: Webcam Mirroring

**Location:** `apps/desktop-swift/Frame/Recording/WebcamCaptureEngine.swift:114-118`

Current code only flips Y-axis:

```swift
let ciImage = CIImage(cvPixelBuffer: pixelBuffer)
    .transformed(by: CGAffineTransform(scaleX: 1, y: -1))   // Flip Y only
    .transformed(by: CGAffineTransform(
        translationX: 0,
        y: CGFloat(CVPixelBufferGetHeight(pixelBuffer))
    ))
```

**Problem:** This shows the webcam as a "non-mirror" view (like looking at a photo). Users expect a mirror view (like looking in a mirror) where raising your right hand raises the hand on the right side of the screen.

### Issue 2: Toolbar Icons Not Updating

**Location:** `apps/desktop-swift/Frame/Overlay/RecordingToolbarPanel.swift:187-195`

Views read `appState.coordinator.config.captureMicrophone` and `appState.webcamEngine.isRunning` directly, but:

- AppState uses `@Observable` (tracks property access)
- Coordinator uses `ObservableObject` (tracks via `objectWillChange`)
- AppState bridges via `_observationBump`, but views don't read it

**Problem:** SwiftUI's `@Observable` system doesn't automatically observe nested `ObservableObject` properties. The bridge exists but isn't being utilized by views.

---

## Task 1: Fix Webcam Horizontal Mirroring

**Files:**

- Modify: `apps/desktop-swift/Frame/Recording/WebcamCaptureEngine.swift:106-123`

**Step 1: Understand the current transform**

Current transform chain (lines 113-118):

1. Scale Y by -1 (flip vertically to correct coordinate system)
2. Translate Y by height (move back into frame)

**Step 2: Add horizontal flip for mirror effect**

Change the transform to:

1. Scale X by -1 (flip horizontally for mirror)
2. Scale Y by -1 (flip vertically for coordinate system)
3. Translate X by width (move back into frame after X flip)
4. Translate Y by height (move back into frame after Y flip)

```swift
nonisolated func captureOutput(
    _ output: AVCaptureOutput,
    didOutput sampleBuffer: CMSampleBuffer,
    from connection: AVCaptureConnection
) {
    guard let pixelBuffer = CMSampleBufferGetImageBuffer(sampleBuffer) else { return }

    let width = CGFloat(CVPixelBufferGetWidth(pixelBuffer))
    let height = CGFloat(CVPixelBufferGetHeight(pixelBuffer))

    // Mirror horizontally (scaleX: -1) + Flip Y for SwiftUI (scaleY: -1)
    let ciImage = CIImage(cvPixelBuffer: pixelBuffer)
        .transformed(by: CGAffineTransform(scaleX: -1, y: -1))
        .transformed(by: CGAffineTransform(translationX: width, y: height))

    Task { @MainActor in
        self.latestFrame = ciImage
    }
}
```

**Step 3: Build and verify**

Run: `cd apps/desktop-swift && xcodebuild -project Frame.xcodeproj -scheme Frame build`
Expected: Clean build

**Step 4: Manual test**

1. Launch app
2. Enable webcam
3. Raise your right hand
4. Verify the hand on the right side of the screen raises (mirror effect)

**Step 5: Commit**

```bash
git add apps/desktop-swift/Frame/Recording/WebcamCaptureEngine.swift
git commit -m "fix(webcam): add horizontal mirroring for natural mirror effect

- Scale X by -1 to flip horizontally
- Translate by width to reposition after flip
- Users now see themselves as in a mirror (not like a photo)"
```

---

## Task 2: Fix Toolbar Observation - Webcam Toggle

**Files:**

- Modify: `apps/desktop-swift/Frame/Overlay/RecordingToolbarPanel.swift:187-196`
- Modify: `apps/desktop-swift/Frame/App/AppState.swift:44-48, 178-184`

**Step 1: Add explicit webcam running state to AppState**

In `AppState.swift`, add a computed property that accesses the observable trigger:

```swift
// MARK: - Webcam

let webcamEngine = WebcamCaptureEngine()

/// Current webcam frame converted to NSImage for display
var webcamImage: NSImage?

/// Mirror of webcamEngine.isRunning for observation bridging
var isWebcamRunning: Bool {
    _ = _observationBump  // Trigger observation
    return webcamEngine.isRunning
}
```

**Step 2: Update toolbar to use the bridged property**

In `RecordingToolbarPanel.swift`, change the webcam toggle (lines 187-196):

```swift
@ViewBuilder
private var webcamToggle: some View {
    toolbarToggle(
        icon: appState.isWebcamRunning
            ? "video.fill" : "video.slash.fill",
        isOn: appState.isWebcamRunning,
        tooltip: "Webcam"
    ) {
        appState.toggleWebcam()
    }
}
```

**Step 3: Build and verify**

Run: `cd apps/desktop-swift && xcodebuild -project Frame.xcodeproj -scheme Frame build`
Expected: Clean build

**Step 4: Manual test**

1. Launch app
2. Click webcam toggle button
3. Verify icon changes from "video.slash.fill" to "video.fill"
4. Click again
5. Verify icon changes back

**Step 5: Commit**

```bash
git add apps/desktop-swift/Frame/App/AppState.swift
git add apps/desktop-swift/Frame/Overlay/RecordingToolbarPanel.swift
git commit -m "fix(observation): bridge webcam state for toolbar updates

- Add isWebcamRunning computed property to AppState
- Property accesses _observationBump to trigger SwiftUI updates
- Update toolbar to use bridged property instead of direct access"
```

---

## Task 3: Fix Toolbar Observation - Audio Toggles

**Files:**

- Modify: `apps/desktop-swift/Frame/Overlay/RecordingToolbarPanel.swift:163-184`
- Modify: `apps/desktop-swift/Frame/App/AppState.swift:28-34`

**Step 1: Add bridged audio state properties to AppState**

In `AppState.swift`, add after the coordinator property:

```swift
// MARK: - Recording

let coordinator = RecordingCoordinator()

/// Mirrors coordinator.isRecording for easy binding in views.
var isRecording: Bool { coordinator.isRecording }
var recordingDuration: TimeInterval { coordinator.recordingDuration }

// MARK: - Audio Settings (Bridged for Observation)

/// System audio capture state (bridged from coordinator.config)
var captureSystemAudio: Bool {
    get {
        _ = _observationBump
        return coordinator.config.captureSystemAudio
    }
    set {
        coordinator.config.captureSystemAudio = newValue
    }
}

/// Microphone capture state (bridged from coordinator.config)
var captureMicrophone: Bool {
    get {
        _ = _observationBump
        return coordinator.config.captureMicrophone
    }
    set {
        coordinator.config.captureMicrophone = newValue
    }
}
```

**Step 2: Update audio toggles to use bridged properties**

In `RecordingToolbarPanel.swift`, change the audioToggles view (lines 163-184):

```swift
@ViewBuilder
private var audioToggles: some View {
    // System audio
    toolbarToggle(
        icon: appState.captureSystemAudio
            ? "speaker.wave.2.fill" : "speaker.slash.fill",
        isOn: appState.captureSystemAudio,
        tooltip: "System Audio"
    ) {
        appState.captureSystemAudio.toggle()
    }

    // Microphone
    toolbarToggle(
        icon: appState.captureMicrophone
            ? "mic.fill" : "mic.slash.fill",
        isOn: appState.captureMicrophone,
        tooltip: "Microphone"
    ) {
        appState.captureMicrophone.toggle()
    }
}
```

**Step 3: Build and verify**

Run: `cd apps/desktop-swift && xcodebuild -project Frame.xcodeproj -scheme Frame build`
Expected: Clean build

**Step 4: Manual test**

1. Launch app
2. Click system audio toggle
3. Verify icon changes (speaker.slash.fill ↔ speaker.wave.2.fill)
4. Click microphone toggle
5. Verify icon changes (mic.slash.fill ↔ mic.fill)

**Step 5: Commit**

```bash
git add apps/desktop-swift/Frame/App/AppState.swift
git add apps/desktop-swift/Frame/Overlay/RecordingToolbarPanel.swift
git commit -m "fix(observation): bridge audio toggle states for toolbar updates

- Add captureSystemAudio and captureMicrophone to AppState
- Properties access _observationBump for SwiftUI observation
- Update toolbar to use bridged properties"
```

---

## Task 4: Fix Webcam Preview Panel Mirroring

**Files:**

- Modify: `apps/desktop-swift/Frame/Overlay/WebcamPreviewPanel.swift:60-87`

**Step 1: Apply horizontal flip to preview content**

The preview panel should show the mirrored view consistently. Add a scaleEffect modifier:

```swift
struct WebcamPreviewContent: View {
    var appState: AppState

    var body: some View {
        ZStack {
            if let webcamImage = appState.webcamImage {
                Image(nsImage: webcamImage)
                    .resizable()
                    .aspectRatio(contentMode: .fill)
                    .frame(width: 200, height: 150)
                    .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
                    .scaleEffect(x: -1, y: 1)  // Mirror horizontally for consistency
            } else {
                // Placeholder when no camera feed
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(.black.opacity(0.6))
                    .frame(width: 200, height: 150)
                    .overlay {
                        VStack(spacing: 8) {
                            Image(systemName: "video.slash.fill")
                                .font(.system(size: 24))
                                .foregroundStyle(.white.opacity(0.5))
                            Text("No Camera")
                                .font(.caption)
                                .foregroundStyle(.white.opacity(0.5))
                        }
                    }
            }
        }
        .shadow(color: .black.opacity(0.4), radius: 8, x: 0, y: 4)
    }
}
```

Wait - this would double-flip since the source is already flipped. Actually, since the CIImage in WebcamCaptureEngine is already flipped horizontally, we should NOT flip it again here. The preview should show the same mirrored view.

**Correction:** No change needed to WebcamPreviewPanel - the source CIImage is already mirrored.

**Step 2: Verify in WebcamOverlayView**

Check `WebcamOverlayView.swift` - it displays the same CIImage (converted to NSImage), so it's already mirrored correctly.

**Step 3: Skip commit (no changes needed)**

The preview panel will automatically show the mirrored view since it uses `appState.webcamImage` which comes from the already-mirrored CIImage.

---

## Task 5: Verify All Observation Bridges Work

**Files:**

- Test: Manual verification

**Step 1: Full manual test of all toolbar toggles**

Run the app and verify each toggle updates immediately:

| Toggle       | Off Icon           | On Icon             | Test Action     |
| ------------ | ------------------ | ------------------- | --------------- |
| System Audio | speaker.slash.fill | speaker.wave.2.fill | Click to toggle |
| Microphone   | mic.slash.fill     | mic.fill            | Click to toggle |
| Webcam       | video.slash.fill   | video.fill          | Click to toggle |

**Step 2: Verify webcam mirror behavior**

1. Enable webcam
2. Raise right hand
3. Verify hand appears on right side of preview (mirror effect)
4. Verify it feels natural (like looking in a mirror, not at a photo)

**Step 3: Run SwiftUI preview if available**

If using Xcode previews, verify they update correctly.

**Step 4: Final commit (if all tests pass)**

```bash
git log --oneline -5  # Review commits
git status  # Verify clean working tree
```

---

## Verification Checklist

- [ ] Webcam shows mirrored view (raise right hand → appears on right)
- [ ] Webcam toolbar icon updates when toggled
- [ ] System audio toolbar icon updates when toggled
- [ ] Microphone toolbar icon updates when toggled
- [ ] All icons show correct state on app launch
- [ ] No build warnings or errors
- [ ] No console warnings about observation

---

## Rollback Plan

If issues occur, revert individual commits:

```bash
# Revert just the observation changes
git revert <commit-hash-of-task-2>
git revert <commit-hash-of-task-3>

# Or revert everything
git reset --hard HEAD~3
```

---

## References

- SwiftUI Observation: https://developer.apple.com/documentation/swiftui/managing-model-data-in-your-app
- ObservableObject vs @Observable: https://developer.apple.com/documentation/swiftui/observable
- CIImage Transforms: https://developer.apple.com/documentation/coreimage/ciimage
- AVCaptureSession: https://developer.apple.com/documentation/avfoundation/avcapturesession
