import AppKit
import SwiftUI
import OSLog

private let logger = Logger(subsystem: "com.frame.app", category: "OverlayManager")

/// Manages the lifecycle of floating overlay panels (toolbar + webcam preview).
/// Owned by AppState — shows panels when in recorder mode, hides on editor mode.
@MainActor
final class OverlayManager {

    // MARK: - Panels

    private let toolbar = RecordingToolbarPanel()
    private let webcamPreview = WebcamPreviewPanel()
    private let selectionBackdrop = SelectionBackdropPanel()
    private let windowSelectionHighlight = WindowSelectionHighlightPanel()
    private let displayInfoCard = DisplayInfoCardPanel()

    private(set) var isShowing = false

    // MARK: - Show / Hide

    /// Shows all overlay panels on the specified screen.
    func showOverlays(appState: AppState, on screen: NSScreen? = nil) {
        guard !isShowing else { return }

        logger.info("Showing overlay panels")

        // Always show the toolbar
        toolbar.show(appState: appState, on: screen)

        // Only show webcam preview if webcam is running and preview is enabled
        if appState.webcamEngine.isRunning && !appState.recorderToolbarSettings.hideCameraPreview {
            webcamPreview.show(webcamEngine: appState.webcamEngine, appState: appState, on: screen)
        }

        isShowing = true
        updateDisplayCardVisibility(appState: appState)
    }

    /// Dismisses all overlay panels.
    func hideOverlays() {
        guard isShowing else { return }

        logger.info("Hiding overlay panels")

        toolbar.dismiss()
        webcamPreview.dismiss()
        selectionBackdrop.dismiss()
        windowSelectionHighlight.dismiss()
        displayInfoCard.dismiss()

        isShowing = false
    }

    /// Shows or hides the webcam preview based on current state.
    func updateWebcamVisibility(appState: AppState) {
        guard isShowing else { return }

        if appState.webcamEngine.isRunning && !appState.recorderToolbarSettings.hideCameraPreview {
            webcamPreview.show(webcamEngine: appState.webcamEngine, appState: appState)
        } else {
            webcamPreview.dismiss()
        }
    }

    /// Resizes the webcam preview panel for full-frame (16:9) or square (1:1) mode.
    func resizeWebcamPreview(fullFrame: Bool) {
        let newSize = fullFrame ? NSSize(width: 200, height: 112) : NSSize(width: 200, height: 200)
        guard let window = webcamPreview.nsWindow else { return }
        var frame = window.frame
        let heightDelta = newSize.height - frame.size.height
        frame.size = newSize
        // Adjust origin so the panel grows/shrinks from the bottom edge
        frame.origin.y -= heightDelta
        window.setFrame(frame, display: true, animate: true)
    }

    func updateDisplayCardVisibility(appState: AppState) {
        guard isShowing else {
            selectionBackdrop.dismiss()
            windowSelectionHighlight.dismiss()
            displayInfoCard.dismiss()
            return
        }

        let eligibleMode = appState.recorderToolbarSettings.captureMode == .display ||
            appState.recorderToolbarSettings.captureMode == .window ||
            appState.recorderToolbarSettings.captureMode == .area
        let shouldShow = !appState.isRecording &&
            appState.hasUserExplicitlySelectedCaptureModeForCard &&
            eligibleMode &&
            !appState.screenRecordingPermissionDenied
        if shouldShow {
            let shouldShowBackdrop = appState.recorderToolbarSettings.captureMode == .display ||
                appState.recorderToolbarSettings.captureMode == .window

            if shouldShowBackdrop {
                selectionBackdrop.show(on: appState.selectedCaptureScreen)
            } else {
                selectionBackdrop.dismiss()
            }

            if appState.recorderToolbarSettings.captureMode == .window,
               let windowFrame = appState.windowSelectionHighlightFrame {
                windowSelectionHighlight.show(
                    frame: windowFrame,
                    isPendingSelection: appState.isAwaitingWindowClickSelection
                )
            } else {
                windowSelectionHighlight.dismiss()
            }

            displayInfoCard.show(appState: appState, on: appState.selectedCaptureScreen)
        } else {
            selectionBackdrop.dismiss()
            windowSelectionHighlight.dismiss()
            displayInfoCard.dismiss()
        }
    }

    func bringToFront() {
        toolbar.nsWindow?.orderFrontRegardless()
        webcamPreview.nsWindow?.orderFrontRegardless()
        selectionBackdrop.nsWindow?.orderFrontRegardless()
        windowSelectionHighlight.nsWindow?.orderFrontRegardless()
        displayInfoCard.nsWindow?.orderFrontRegardless()
    }

    // MARK: - Window References (for SCContentFilter exclusion)

    /// Returns NSWindow references for all visible overlay panels.
    /// Used by ScreenRecorder to exclude these from the capture.
    var overlayWindows: [NSWindow] {
        var windows: [NSWindow] = []
        if let w = toolbar.nsWindow { windows.append(w) }
        if let w = webcamPreview.nsWindow { windows.append(w) }
        if let w = selectionBackdrop.nsWindow { windows.append(w) }
        if let w = windowSelectionHighlight.nsWindow { windows.append(w) }
        if let w = displayInfoCard.nsWindow { windows.append(w) }
        return windows
    }
}

private final class SelectionBackdropPanel {
    private var panel: FloatingPanel<SelectionBackdropContent>?

    @MainActor
    func show(on screen: NSScreen? = nil) {
        guard let target = screen ?? NSScreen.main ?? NSScreen.screens.first else { return }

        if let panel {
            panel.setFrame(target.frame, display: true)
            panel.show()
            return
        }

        let panel = FloatingPanel(contentRect: target.frame) {
            SelectionBackdropContent()
        }
        panel.level = NSWindow.Level(rawValue: NSWindow.Level.floating.rawValue - 1)
        panel.hasShadow = false
        panel.ignoresMouseEvents = true
        panel.isMovableByWindowBackground = false
        panel.setFrame(target.frame, display: true)
        panel.show()
        self.panel = panel
    }

    func dismiss() {
        panel?.dismiss()
        panel = nil
    }

    var nsWindow: NSWindow? {
        panel
    }
}

private struct SelectionBackdropContent: View {
    var body: some View {
        Color.black.opacity(0.24)
            .ignoresSafeArea()
    }
}

private final class WindowSelectionHighlightPanel {
    private var panel: FloatingPanel<WindowSelectionHighlightContent>?
    private var previousFrame: CGRect?
    private var previousPendingSelectionState: Bool?
    private let animationDuration: TimeInterval = 0.14

    @MainActor
    func show(frame: CGRect, isPendingSelection: Bool) {
        let normalizedFrame = frame.integral
        let content = WindowSelectionHighlightContent(isPendingSelection: isPendingSelection)
        let shouldAnimateTransition = shouldAnimateTransition(
            to: normalizedFrame,
            pendingSelectionState: isPendingSelection
        )

        if let panel {
            panel.updateContent {
                content
            }

            if shouldAnimateTransition {
                NSAnimationContext.runAnimationGroup { context in
                    context.duration = animationDuration
                    context.allowsImplicitAnimation = true
                    panel.animator().setFrame(normalizedFrame, display: true)
                    panel.animator().alphaValue = isPendingSelection ? 1.0 : 0.95
                }
            } else {
                panel.setFrame(normalizedFrame, display: true)
                panel.alphaValue = isPendingSelection ? 1.0 : 0.95
            }

            panel.show()
            previousFrame = normalizedFrame
            previousPendingSelectionState = isPendingSelection
            return
        }

        let panel = FloatingPanel(contentRect: normalizedFrame) {
            content
        }
        panel.level = NSWindow.Level(rawValue: NSWindow.Level.floating.rawValue)
        panel.hasShadow = false
        panel.ignoresMouseEvents = true
        panel.isMovableByWindowBackground = false
        panel.alphaValue = isPendingSelection ? 1.0 : 0.95
        panel.setFrame(normalizedFrame, display: true)
        panel.show()
        self.panel = panel
        previousFrame = normalizedFrame
        previousPendingSelectionState = isPendingSelection
    }

    func dismiss() {
        panel?.dismiss()
        panel = nil
        previousFrame = nil
        previousPendingSelectionState = nil
    }

    var nsWindow: NSWindow? {
        panel
    }

    private func shouldAnimateTransition(to frame: CGRect, pendingSelectionState: Bool) -> Bool {
        guard let previousFrame else {
            return false
        }

        if previousPendingSelectionState != pendingSelectionState {
            return true
        }

        let horizontalShift = abs(previousFrame.midX - frame.midX)
        let verticalShift = abs(previousFrame.midY - frame.midY)
        let widthDelta = abs(previousFrame.width - frame.width)
        let heightDelta = abs(previousFrame.height - frame.height)

        return horizontalShift > 1 || verticalShift > 1 || widthDelta > 1 || heightDelta > 1
    }
}

private struct WindowSelectionHighlightContent: View {
    let isPendingSelection: Bool

    var body: some View {
        let tintColor = isPendingSelection ? Color.accentColor : Color.white

        RoundedRectangle(cornerRadius: 11, style: .continuous)
            .fill(tintColor.opacity(isPendingSelection ? 0.26 : 0.13))
            .overlay(
                RoundedRectangle(cornerRadius: 11, style: .continuous)
                    .stroke(tintColor.opacity(isPendingSelection ? 0.92 : 0.58), lineWidth: 2)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 11, style: .continuous)
                    .stroke(tintColor.opacity(isPendingSelection ? 0.44 : 0.20), lineWidth: 6)
                    .blur(radius: 9)
                    .padding(-2)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 11, style: .continuous)
                    .stroke(Color.white.opacity(isPendingSelection ? 0.24 : 0.12), lineWidth: 1)
                    .padding(2)
            )
            .shadow(color: tintColor.opacity(isPendingSelection ? 0.30 : 0.14), radius: 16)
            .animation(.easeInOut(duration: 0.14), value: isPendingSelection)
            .ignoresSafeArea()
    }
}

private final class DisplayInfoCardPanel {
    private var panel: FloatingPanel<DisplayInfoCardContent>?

    @MainActor
    private func panelSize(for appState: AppState) -> NSSize {
        switch appState.recorderToolbarSettings.captureMode {
        case .window:
            return NSSize(width: 360, height: 220)
        default:
            return NSSize(width: 320, height: 136)
        }
    }

    @MainActor
    func show(appState: AppState, on screen: NSScreen? = nil) {
        let content = DisplayInfoCardContent(appState: appState)
        let size = panelSize(for: appState)

        if let panel {
            panel.updateContent {
                content
            }
            panel.setContentSize(size)
            if let target = screen ?? NSScreen.main ?? NSScreen.screens.first {
                panel.positionAtCenter(of: target)
            }
            panel.show()
            return
        }

        let panel = FloatingPanel(
            contentRect: NSRect(x: 0, y: 0, width: size.width, height: size.height)
        ) {
            content
        }
        panel.isMovableByWindowBackground = true
        panel.hasShadow = false
        if let target = screen ?? NSScreen.main ?? NSScreen.screens.first {
            panel.positionAtCenter(of: target)
        }
        panel.show()
        self.panel = panel
    }

    func dismiss() {
        panel?.dismiss()
        panel = nil
    }

    var nsWindow: NSWindow? {
        panel
    }
}

private struct DisplayInfoCardContent: View {
    var appState: AppState

    var body: some View {
        @Bindable var appState = appState

        Group {
            if appState.recorderToolbarSettings.captureMode == .window {
                windowCardContent(appState: appState)
            } else {
                defaultCardContent(appState: appState)
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .strokeBorder(.white.opacity(0.15), lineWidth: 1)
        )
    }

    @ViewBuilder
    private func defaultCardContent(appState: AppState) -> some View {
        VStack(spacing: 6) {
            Text(appState.selectedCaptureTitleForCard)
                .font(.system(size: 22, weight: .semibold))
                .foregroundStyle(.white)

            Text("\(appState.selectedCaptureResolutionForCard) · \(appState.selectedDisplayFPSForToolbar)")
                .font(.system(size: 14, weight: .medium))
                .foregroundStyle(.white.opacity(0.75))

            if let hint = appState.displaySelectionHintForCard {
                Text(hint)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(.white.opacity(0.85))
            }

            RecordingStartSplitButton(appState: appState)
        }
    }

    @ViewBuilder
    private func windowCardContent(appState: AppState) -> some View {
        VStack(spacing: 8) {
            Group {
                if let icon = appState.selectedWindowAppIconForCard {
                    Image(nsImage: icon)
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                } else {
                    Image(systemName: "macwindow")
                        .font(.system(size: 24, weight: .medium))
                        .foregroundStyle(.white.opacity(0.85))
                }
            }
            .frame(width: 48, height: 48)
            .padding(6)
            .background(.white.opacity(0.08), in: RoundedRectangle(cornerRadius: 12, style: .continuous))

            Text(appState.selectedWindowAppNameForCard)
                .font(.system(size: 40, weight: .semibold))
                .foregroundStyle(.white)
                .lineLimit(1)
                .minimumScaleFactor(0.65)

            HStack(spacing: 8) {
                Text(appState.selectedCaptureResolutionForCard)
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(.white.opacity(0.78))

                WindowResizeMenuButton(appState: appState)
            }

            if let hint = appState.displaySelectionHintForCard {
                Text(hint)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(.white.opacity(0.85))
            }

            RecordingStartSplitButton(appState: appState)
        }
    }
}

private struct WindowResizeMenuButton: View {
    var appState: AppState

    var body: some View {
        @Bindable var appState = appState

        Menu {
            ForEach(appState.allWindowPresetGroups) { group in
                Menu(group.title) {
                    ForEach(group.presets) { preset in
                        Button {
                            appState.selectWindowOutputSize(preset)
                        } label: {
                            sizeLabel(preset.label, isSelected: appState.isSelectedWindowOutputSize(preset))
                        }
                    }
                }
            }

            Divider()

            if appState.hasSavedWindowOutputSizes {
                ForEach(appState.savedWindowOutputSizes) { preset in
                    Button {
                        appState.selectWindowOutputSize(preset)
                    } label: {
                        sizeLabel(preset.label, isSelected: appState.isSelectedWindowOutputSize(preset))
                    }
                }
            } else {
                Text("No saved sizes")
            }

            Button("Save current size") {
                appState.saveCurrentWindowOutputSizePreset()
            }
            .disabled(!appState.canSaveCurrentWindowSizePreset)

            Button("Custom...") {
                appState.openCustomWindowSizeSheet()
            }
        } label: {
            HStack(spacing: 4) {
                Text("Resize")
                    .font(.system(size: 11, weight: .semibold))
                Image(systemName: "chevron.down")
                    .font(.system(size: 9, weight: .semibold))
            }
            .foregroundStyle(.white.opacity(0.92))
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(.white.opacity(0.14), in: RoundedRectangle(cornerRadius: 6, style: .continuous))
        }
        .menuStyle(.borderlessButton)
        .buttonStyle(.plain)
        .sheet(
            isPresented: Binding(
                get: { appState.showWindowCustomSizeSheet },
                set: { appState.showWindowCustomSizeSheet = $0 }
            )
        ) {
            WindowCustomSizeSheet(appState: appState)
        }
    }

    @ViewBuilder
    private func sizeLabel(_ title: String, isSelected: Bool) -> some View {
        if isSelected {
            Label(title, systemImage: "checkmark")
        } else {
            Text(title)
        }
    }
}

private struct WindowCustomSizeSheet: View {
    var appState: AppState

    var body: some View {
        @Bindable var appState = appState

        VStack(alignment: .leading, spacing: 14) {
            Text("Custom size")
                .font(.system(size: 16, weight: .semibold))

            HStack(spacing: 12) {
                Text("Horizontal")
                    .font(.system(size: 13, weight: .medium))
                    .frame(width: 86, alignment: .leading)

                TextField("Width", value: $appState.customWindowOutputWidth, format: .number)
                    .textFieldStyle(.roundedBorder)
            }

            HStack(spacing: 12) {
                Text("Vertical")
                    .font(.system(size: 13, weight: .medium))
                    .frame(width: 86, alignment: .leading)

                TextField("Height", value: $appState.customWindowOutputHeight, format: .number)
                    .textFieldStyle(.roundedBorder)
            }

            HStack(spacing: 10) {
                Spacer()

                Button("Cancel") {
                    appState.showWindowCustomSizeSheet = false
                }

                Button("Apply") {
                    appState.applyCustomWindowSize()
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(16)
        .frame(width: 300)
    }
}
