# Display Selection & Capture Mode UI Fix — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use skill({ name: "executing-plans" }) to implement this plan task-by-task.

**Goal:** Fix manual display selection so it isn't overwritten unexpectedly, ensure capture mode buttons don't look pre-selected by default, and show the center card only after explicit user interaction.

**Architecture:** Use explicit user-intent flags (`hasUserManuallySelectedDisplayForSession`, `hasUserExplicitlySelectedCaptureModeForCard`) to gate all auto-selection logic. Route all display selection through `AppState.setSelectedDisplay()` to ensure flags are respected.

**Tech Stack:** Swift 5.9, SwiftUI, ScreenCaptureKit, macOS 13.0+

---

## Prerequisites

- Ensure you have a multi-monitor setup for testing (or use DisplayLink/virtual displays)
- Verify build compiles: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`

---

## Phase 1: Fix Display Selection State Management

### Task 1: Remove auto-selection from `AppState.setCaptureMode()`

**Files:**

- Modify: `apps/desktop-swift/Frame/App/AppState.swift:611-628`

**Problem:** `setCaptureMode()` auto-picks a "preferred display" when entering `.display` mode, overwriting any manual selection.

**Step 1: Read current implementation**

Verify the current code at lines 611-628:

```swift
func setCaptureMode(_ mode: RecorderToolbarSettings.CaptureMode) {
    recorderToolbarSettings.captureMode = mode

    if mode == .display,
       !hasUserManuallySelectedDisplayForSession,
       let preferredDisplay = preferredDisplayForCurrentScreen() {
        coordinator.config.selectedDisplay = preferredDisplay
    }

    hasUserExplicitlySelectedCaptureModeForCard = true
    overlayManager.updateDisplayCardVisibility(appState: self)
}
```

**Step 2: Remove the auto-selection logic**

Edit to remove lines 614-618 (the `if mode == .display` block):

```swift
func setCaptureMode(_ mode: RecorderToolbarSettings.CaptureMode) {
    recorderToolbarSettings.captureMode = mode
    hasUserExplicitlySelectedCaptureModeForCard = true
    overlayManager.updateDisplayCardVisibility(appState: self)
}
```

**Step 3: Build to verify**

Run: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`
Expected: Build succeeds with no errors

**Step 4: Commit**

```bash
git add apps/desktop-swift/Frame/App/AppState.swift
git commit -m "fix: remove auto-display-selection from setCaptureMode"
```

---

### Task 2: Remove auto-selection from `RecordingCoordinator.refreshSources()`

**Files:**

- Modify: `apps/desktop-swift/Frame/Recording/RecordingCoordinator.swift:44-51`

**Problem:** `refreshSources()` auto-selects the first display when `selectedDisplay` is nil, which happens on startup and can overwrite intended state.

**Step 1: Read current implementation**

Verify lines 44-51:

```swift
func refreshSources() async {
    await screenRecorder.refreshAvailableContent()

    // Auto-select primary display if none selected
    if config.selectedDisplay == nil {
        config.selectedDisplay = screenRecorder.availableDisplays.first
    }
}
```

**Step 2: Remove the auto-selection logic**

Edit to remove lines 47-50:

```swift
func refreshSources() async {
    await screenRecorder.refreshAvailableContent()
}
```

**Step 3: Build to verify**

Run: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`
Expected: Build succeeds with no errors

**Step 4: Commit**

```bash
git add apps/desktop-swift/Frame/Recording/RecordingCoordinator.swift
git commit -m "fix: remove auto-display-selection from refreshSources"
```

---

### Task 3: Make `SourcePicker` use AppState instead of direct config mutation

**Files:**

- Modify: `apps/desktop-swift/Frame/Views/Recording/SourcePicker.swift:1-154`

**Problem:** `SourcePicker` directly sets `config.selectedDisplay = display` (line 40), bypassing `AppState.setSelectedDisplay()` which sets the `hasUserManuallySelectedDisplayForSession` flag.

**Step 1: Read current implementation**

Review the display selection section (lines 34-48):

```swift
Section("Display") {
    ForEach(Array(displays.enumerated()), id: \.offset) { index, display in
        Button(action: {
            config.captureType = .display
            config.selectedDisplay = display  // BUG: Direct mutation
        }) {
            Label(
                displays.count > 1 ? "Display \(index + 1)" : "Full Screen",
                systemImage: "macwindow"
            )
        }
    }
}
```

**Step 2: Add AppState dependency to SourcePicker**

Replace the struct signature (line 5-9):

**From:**

```swift
struct SourcePicker: View {
    @Binding var config: RecordingConfig
    let displays: [SCDisplay]
    let windows: [SCWindow]
    var appState: AppState?  // Optional — for webcam toggle
```

**To:**

```swift
struct SourcePicker: View {
    @Binding var config: RecordingConfig
    let displays: [SCDisplay]
    let windows: [SCWindow]
    var appState: AppState?  // Now required for display selection

    /// Helper to set display through AppState to ensure flags are updated
    private func selectDisplay(_ display: SCDisplay) {
        appState?.setSelectedDisplay(display)
    }
```

**Step 3: Update display button action**

Edit lines 37-46 to use the helper:

**From:**

```swift
Button(action: {
    config.captureType = .display
    config.selectedDisplay = display
}) {
    Label(
        displays.count > 1 ? "Display \(index + 1)" : "Full Screen",
        systemImage: "macwindow"
    )
}
```

**To:**

```swift
Button(action: {
    config.captureType = .display
            selectDisplay(display)
}) {
    Label(
        displays.count > 1 ? "Display \(index + 1)" : "Full Screen",
        systemImage: "macwindow"
    )
}
```

**Step 4: Update display name to use AppState**

The display name currently uses generic "Display N" labels. Update to use `appState.displayName(for:)`:

**From:**

```swift
Label(
    displays.count > 1 ? "Display \(index + 1)" : "Full Screen",
    systemImage: "macwindow"
)
```

**To:**

```swift
Label(
    appState?.displayName(for: display) ?? "Display \(index + 1)",
    systemImage: "macwindow"
)
```

**Step 5: Build to verify**

Run: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`
Expected: Build succeeds with no errors

**Step 6: Commit**

```bash
git add apps/desktop-swift/Frame/Views/Recording/SourcePicker.swift
git commit -m "fix: route SourcePicker display selection through AppState"
```

---

### Task 4: Fix flag reset logic in `hideRecorderOverlays()`

**Files:**

- Modify: `apps/desktop-swift/Frame/App/AppState.swift:589-595`

**Problem:** `hideRecorderOverlays()` resets both `hasUserExplicitlySelectedCaptureModeForCard` and `hasUserManuallySelectedDisplayForSession` to `false`, losing the user's manual-selection intent.

**Step 1: Read current implementation**

Verify lines 589-595:

```swift
func hideRecorderOverlays() {
    overlayManager.hideOverlays()
    hasUserExplicitlySelectedCaptureModeForCard = false
    hasUserManuallySelectedDisplayForSession = false
    hideMainWindow()
    menuBarManager?.refresh()
}
```

**Step 2: Decide on the correct behavior**

The intent of `hideRecorderOverlays()` is to dismiss the UI when the user clicks the X button. The question is: should this reset their capture mode/display selection intent?

Based on user requirements, the answer is **NO** — hiding overlays should not reset the user's explicit selections. The selections should persist until:

1. The app quits and restarts (fresh session)
2. The user explicitly changes the selection again

**Step 3: Remove the flag resets**

Edit to remove lines 591-592:

```swift
func hideRecorderOverlays() {
    overlayManager.hideOverlays()
    // Note: We intentionally do NOT reset hasUserExplicitlySelectedCaptureModeForCard
    // or hasUserManuallySelectedDisplayForSession here. Hiding overlays is a UI action,
    // not a user intent to clear their selections. The flags persist for the session.
    hideMainWindow()
    menuBarManager?.refresh()
}
```

**Step 4: Build to verify**

Run: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`
Expected: Build succeeds with no errors

**Step 5: Commit**

```bash
git add apps/desktop-swift/Frame/App/AppState.swift
git commit -m "fix: preserve manual selection flags when hiding overlays"
```

---

## Phase 2: UI Behavior Fixes

### Task 5: Ensure center card is hidden on startup

**Files:**

- Verify: `apps/desktop-swift/Frame/Overlay/OverlayManager.swift:64-79`

**Problem:** The center card visibility depends on `hasUserExplicitlySelectedCaptureModeForCard` which starts as `false` (line 385 in AppState). Let's verify the logic is correct.

**Step 1: Review current card visibility logic**

In `OverlayManager.swift` lines 64-79:

```swift
func updateDisplayCardVisibility(appState: AppState) {
    guard isShowing else {
        displayInfoCard.dismiss()
        return
    }

    let eligibleMode = appState.recorderToolbarSettings.captureMode == .display ||
        appState.recorderToolbarSettings.captureMode == .window ||
        appState.recorderToolbarSettings.captureMode == .area
    let shouldShow = !appState.isRecording && appState.hasUserExplicitlySelectedCaptureModeForCard && eligibleMode
    if shouldShow {
        displayInfoCard.show(appState: appState, on: appState.selectedCaptureScreen)
    } else {
        displayInfoCard.dismiss()
    }
}
```

This logic is correct — the card only shows when `hasUserExplicitlySelectedCaptureModeForCard` is `true`. Since it defaults to `false`, the card is hidden on startup.

**Step 2: Verify the flag default value**

In `AppState.swift` line 385:

```swift
var hasUserExplicitlySelectedCaptureModeForCard = false
```

This is correct. No changes needed for this task — the existing code already satisfies the requirement.

**Step 3: Document verification**

This task is verification-only. The card is already hidden on startup because:

1. `hasUserExplicitlySelectedCaptureModeForCard` defaults to `false`
2. `updateDisplayCardVisibility()` requires it to be `true` to show the card

---

### Task 6: Ensure capture mode buttons don't look pre-selected

**Files:**

- Verify: `apps/desktop-swift/Frame/Overlay/RecordingToolbarPanel.swift:195-233`

**Problem:** The capture mode buttons use `isSelected` styling based on `hasUserExplicitlySelectedCaptureModeForCard`.

**Step 1: Review current button logic**

Lines 195-233:

```swift
@ViewBuilder
private var captureSourceButtons: some View {
    HStack(spacing: 2) {
        sourceButton(
            icon: "display",
            label: "Display",
            isSelected: appState.hasUserExplicitlySelectedCaptureModeForCard &&
                appState.recorderToolbarSettings.captureMode == RecorderToolbarSettings.CaptureMode.display
        ) {
            appState.setCaptureMode(RecorderToolbarSettings.CaptureMode.display)
        }
        // ... other buttons
    }
}
```

This is correct — buttons only show as selected when `hasUserExplicitlySelectedCaptureModeForCard` is `true`. Since it defaults to `false`, no buttons appear selected on startup.

**Step 2: Verify this still works after our changes**

Run a quick build check:

```bash
xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build
```

Expected: Build succeeds

**Step 3: Document verification**

This task is verification-only. The buttons already don't look pre-selected because:

1. `hasUserExplicitlySelectedCaptureModeForCard` defaults to `false`
2. `isSelected` for each button requires it to be `true`

---

### Task 7: Ensure card appears after explicit capture mode click

**Files:**

- Verify: `apps/desktop-swift/Frame/App/AppState.swift:611-628`

**Step 1: Review current flow**

When a capture mode button is clicked:

1. `setCaptureMode()` is called
2. It sets `hasUserExplicitlySelectedCaptureModeForCard = true` (line 620)
3. It calls `overlayManager.updateDisplayCardVisibility(appState: self)` (line 621)

This flow is correct and should still work after our changes.

**Step 2: Verify the display card appears for eligible modes**

In `OverlayManager.swift` lines 70-72:

```swift
let eligibleMode = appState.recorderToolbarSettings.captureMode == .display ||
    appState.recorderToolbarSettings.captureMode == .window ||
    appState.recorderToolbarSettings.captureMode == .area
```

The card only appears for `.display`, `.window`, and `.area` modes — not for `.device`. This is intentional.

**Step 3: Document verification**

This task is verification-only. The card appears correctly after clicking a capture mode button because:

1. Clicking a button calls `setCaptureMode()`
2. `setCaptureMode()` sets `hasUserExplicitlySelectedCaptureModeForCard = true`
3. `updateDisplayCardVisibility()` is triggered and shows the card for eligible modes

---

## Phase 3: Display Name Improvements

### Task 8: Verify and improve display name mapping

**Files:**

- Review: `apps/desktop-swift/Frame/App/AppState.swift:696-712`

**Current implementation:**

```swift
func displayName(for display: SCDisplay) -> String {
    if let matchedScreen = screenForDisplay(display) {
        let name = matchedScreen.localizedName.trimmingCharacters(in: .whitespacesAndNewlines)
        if !name.isEmpty {
            return name
        }
    }

    if availableDisplays.count > 1,
       let index = availableDisplays.firstIndex(where: {
           $0.displayID == display.displayID
       }) {
        return "Display \(index + 1)"
    }

    return "Built-in Display"
}
```

**Problem:** The current implementation already tries to use `NSScreen.localizedName`, but there are potential issues:

1. `screenForDisplay()` uses `NSScreenNumber` matching which may fail in some configurations
2. Fallback to "Display N" is generic
3. "Built-in Display" fallback may be inaccurate

**Step 1: Check if we can get better names from ScreenCaptureKit**

Unfortunately, `SCDisplay` doesn't have a `localizedName` property — only `displayID` and frame. We must rely on `NSScreen` matching.

**Step 2: Improve the screen matching logic**

The current `screenForDisplay()` (lines 787-805) tries two methods:

1. Match by `NSScreenNumber` (device ID)
2. Match by frame coordinates

This is reasonable but could be more robust. Let's review:

```swift
private func screenForDisplay(_ selectedDisplay: SCDisplay) -> NSScreen? {
    // Method 1: Match by NSScreenNumber (displayID)
    if let matchedByID = NSScreen.screens.first(where: {
        guard let screenNumber = $0.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber else {
            return false
        }
        return screenNumber.uint32Value == selectedDisplay.displayID
    }) {
        return matchedByID
    }

    // Method 2: Match by frame coordinates (fallback)
    let targetFrame = selectedDisplay.frame
    return NSScreen.screens.first(where: { screen in
        let frame = screen.frame
        return abs(frame.minX - targetFrame.minX) < 2 &&
            abs(frame.minY - targetFrame.minY) < 2 &&
            abs(frame.width - targetFrame.width) < 2 &&
            abs(frame.height - targetFrame.height) < 2
    })
}
```

This is already fairly robust. However, let's add a caching mechanism to improve reliability and add some debug logging.

**Step 3: Add display name caching for consistency**

Since display names shouldn't change during a session, we can cache them to avoid repeated lookups.

Add a cache property to AppState (after line 386):

```swift
// Display name cache to avoid repeated NSScreen lookups
private var displayNameCache: [UInt32: String] = [:]
```

Update `displayName(for:)` to use the cache:

```swift
func displayName(for display: SCDisplay) -> String {
    // Check cache first
    if let cached = displayNameCache[display.displayID] {
        return cached
    }

    let name: String

    if let matchedScreen = screenForDisplay(display) {
        let screenName = matchedScreen.localizedName.trimmingCharacters(in: .whitespacesAndNewlines)
        if !screenName.isEmpty {
            name = screenName
        } else {
            name = fallbackDisplayName(for: display)
        }
    } else {
        name = fallbackDisplayName(for: display)
    }

    // Cache the result
    displayNameCache[display.displayID] = name
    return name
}

private func fallbackDisplayName(for display: SCDisplay) -> String {
    if availableDisplays.count > 1,
       let index = availableDisplays.firstIndex(where: { $0.displayID == display.displayID }) {
        return "Display \(index + 1)"
    }
    return "Built-in Display"
}
```

**Step 4: Clear cache when displays change**

In `refreshSources()`, clear the cache:

```swift
func refreshSources() async {
    await coordinator.refreshSources()
    // Clear display name cache when displays may have changed
    displayNameCache.removeAll()
}
```

Wait — we can't easily do this from `AppState.refreshSources()` since it's a simple wrapper. Instead, we should clear the cache whenever `availableDisplays` changes.

Actually, a simpler approach: don't cache, just ensure the lookup is fast enough. The current implementation is fine without caching for now.

**Step 5: Simplified approach — just improve the fallback naming**

Instead of complex caching, let's improve the fallback to be more descriptive:

```swift
func displayName(for display: SCDisplay) -> String {
    if let matchedScreen = screenForDisplay(display) {
        let name = matchedScreen.localizedName.trimmingCharacters(in: .whitespacesAndNewlines)
        if !name.isEmpty {
            return name
        }
    }

    // Fallback: Use index-based naming with total count
    if let index = availableDisplays.firstIndex(where: { $0.displayID == display.displayID }) {
        if availableDisplays.count > 1 {
            return "Display \(index + 1) of \(availableDisplays.count)"
        }
    }

    return "Display"
}
```

**Step 6: Build to verify**

Run: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`
Expected: Build succeeds with no errors

**Step 7: Commit**

```bash
git add apps/desktop-swift/Frame/App/AppState.swift
git commit -m "refactor: improve display name fallback logic"
```

---

## Phase 4: Testing & Verification

### Task 9: Manual testing checklist

**Setup:**

- macOS with 2+ monitors connected (or 1 external + built-in)
- Frame app built and running from Xcode

**Test Cases:**

#### Test 1: Startup behavior

1. Quit Frame completely
2. Launch Frame
3. **Expected:**
   - Center card is NOT visible
   - Capture mode buttons (Display/Window/Area/Device) are NOT highlighted/selected
   - Toolbar shows at bottom of screen

#### Test 2: Display selection flow

1. Click "Display" button in toolbar
2. **Expected:**
   - "Display" button becomes highlighted/selected
   - Display selector dropdown appears next to the buttons
   - Center card appears showing the selected display name and resolution
3. Click the display selector dropdown
4. **Expected:**
   - Shows list of available displays with real monitor names (e.g., "S27B80P", "Built-in Display")
   - Currently selected display has a checkmark

#### Test 3: Manual display persistence

1. Select Display A from the dropdown
2. Click "Window" button (switch modes)
3. Click "Display" button again
4. **Expected:**
   - Display A is still selected (not reset to Display B or auto-selected)
   - The dropdown shows Display A as selected

#### Test 4: Multi-monitor selection

1. With 2+ monitors connected, click "Display" button
2. Select "Display 2" (or external monitor name) from dropdown
3. **Expected:**
   - Center card updates to show Display 2's name and resolution
   - Recording (when started) captures Display 2

#### Test 5: Hiding overlays preserves selection

1. Select a specific display from the dropdown
2. Click the X button to hide overlays
3. Reopen Frame from menu bar or dock
4. **Expected:**
   - Your display selection is preserved (the same display is still selected)

#### Test 6: Window mode card visibility

1. Click "Window" button
2. **Expected:**
   - Center card appears (window mode is eligible for card)
   - Card shows "Window" or the selected window title

#### Test 7: Device mode card visibility

1. Click "Device" button
2. **Expected:**
   - Center card does NOT appear (device mode is not eligible)

### Task 10: Edge case testing

#### Edge Case 1: No displays available

1. Simulate or wait for a state where no displays are detected
2. **Expected:** Display dropdown shows "No displays available"

#### Edge Case 2: Single display

1. Disconnect all external monitors (use only built-in)
2. Click "Display" button
3. **Expected:**
   - Display selector shows the single display
   - Name is either the real monitor name or "Display"

#### Edge Case 3: Rapid mode switching

1. Rapidly click between Display → Window → Area → Display
2. **Expected:**
   - No crashes
   - Card visibility updates correctly
   - Display selection persists through the switching

---

## Summary of Changes

### Files Modified

1. **`apps/desktop-swift/Frame/App/AppState.swift`**
   - Removed auto-display-selection from `setCaptureMode()`
   - Removed flag reset from `hideRecorderOverlays()`
   - Improved `displayName(for:)` fallback logic

2. **`apps/desktop-swift/Frame/Recording/RecordingCoordinator.swift`**
   - Removed auto-display-selection from `refreshSources()`

3. **`apps/desktop-swift/Frame/Views/Recording/SourcePicker.swift`**
   - Added `selectDisplay()` helper that routes through AppState
   - Updated display button to use the helper
   - Updated display name to use `appState.displayName(for:)`

### Behavior Changes

| Before                                           | After                                      |
| ------------------------------------------------ | ------------------------------------------ |
| Display auto-selected when entering Display mode | User must explicitly select a display      |
| First display auto-selected on startup           | No display selected until user chooses one |
| Flags reset when hiding overlays                 | Flags persist for the session              |
| SourcePicker directly mutated config             | SourcePicker routes through AppState       |
| Generic "Display 1", "Display 2" labels          | Real OS monitor names when available       |

### Verification Checklist

- [ ] Build succeeds: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`
- [ ] Card hidden on startup
- [ ] Buttons not pre-selected on startup
- [ ] Card appears after clicking Display/Window/Area
- [ ] Display dropdown shows real monitor names
- [ ] Manual display selection persists across mode switches
- [ ] Manual display selection persists after hiding overlays
- [ ] Multi-monitor selection works correctly
- [ ] No regressions in recording functionality
