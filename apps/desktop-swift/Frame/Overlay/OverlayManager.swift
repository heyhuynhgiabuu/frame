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

    private(set) var isShowing = false

    // MARK: - Show / Hide

    /// Shows all overlay panels on the specified screen.
    func showOverlays(appState: AppState, on screen: NSScreen? = nil) {
        guard !isShowing else { return }

        logger.info("Showing overlay panels")

        // Always show the toolbar
        toolbar.show(appState: appState, on: screen)

        // Only show webcam preview if webcam is running
        if appState.webcamEngine.isRunning {
            webcamPreview.show(webcamEngine: appState.webcamEngine, on: screen)
        }

        isShowing = true
    }

    /// Dismisses all overlay panels.
    func hideOverlays() {
        guard isShowing else { return }

        logger.info("Hiding overlay panels")

        toolbar.dismiss()
        webcamPreview.dismiss()

        isShowing = false
    }

    /// Shows or hides the webcam preview based on current state.
    func updateWebcamVisibility(appState: AppState) {
        guard isShowing else { return }

        if appState.webcamEngine.isRunning {
            webcamPreview.show(webcamEngine: appState.webcamEngine)
        } else {
            webcamPreview.dismiss()
        }
    }

    // MARK: - Window References (for SCContentFilter exclusion)

    /// Returns NSWindow references for all visible overlay panels.
    /// Used by ScreenRecorder to exclude these from the capture.
    var overlayWindows: [NSWindow] {
        var windows: [NSWindow] = []
        if let w = toolbar.nsWindow { windows.append(w) }
        if let w = webcamPreview.nsWindow { windows.append(w) }
        return windows
    }
}
