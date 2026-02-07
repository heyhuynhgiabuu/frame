import SwiftUI
import ScreenCaptureKit
import AVFoundation
import Combine
import OSLog

private let logger = Logger(subsystem: "com.frame.app", category: "AppState")

/// The global application state, shared across all views.
@Observable
@MainActor
final class AppState {

    // MARK: - Mode

    enum Mode: String, CaseIterable {
        case recorder
        case editor
    }

    var mode: Mode = .recorder {
        didSet {
            handleModeChange(from: oldValue, to: mode)
        }
    }

    // MARK: - Recording

    let coordinator = RecordingCoordinator()

    /// Mirrors coordinator.isRecording for easy binding in views.
    /// Must read _observationBump so @Observable triggers re-render when coordinator changes.
    var isRecording: Bool {
        _ = _observationBump
        return coordinator.isRecording
    }

    /// Whether screen recording permission is denied.
    /// Bridged from ScreenRecorder so SwiftUI views can observe it reliably
    /// (nested ObservableObject changes don't propagate through @Observable automatically).
    var screenRecordingPermissionDenied: Bool {
        _ = _observationBump
        return coordinator.screenRecorder.permissionDenied
    }

    /// True while startRecordingAsync() is in-flight — prevents double-clicks
    /// and lets the toolbar show a "starting…" state.
    var isStartingRecording = false
    var recordingDuration: TimeInterval {
        _ = _observationBump
        return coordinator.recordingDuration
    }

    /// Bridged config accessors for SwiftUI Observation.
    /// Reading _observationBump ensures views re-render when coordinator publishes changes.
    var captureSystemAudio: Bool {
        get {
            _ = _observationBump
            return coordinator.config.captureSystemAudio
        }
        set {
            coordinator.config.captureSystemAudio = newValue
        }
    }

    var captureMicrophone: Bool {
        get {
            _ = _observationBump
            return coordinator.config.captureMicrophone
        }
        set {
            coordinator.config.captureMicrophone = newValue
        }
    }

    var isWebcamRunning: Bool {
        _ = _observationBump
        return webcamEngine.isRunning
    }

    // MARK: - Playback

    let playbackEngine = PlaybackEngine()

    // MARK: - Export

    let exportEngine = ExportEngine()

    // MARK: - Webcam

    let webcamEngine = WebcamCaptureEngine()

    /// Current webcam frame converted to NSImage for display
    var webcamImage: NSImage?

    /// Webcam playback player for editor mode (plays the recorded webcam .mov)
    var webcamPlayer: AVPlayer?

    /// Current frame from webcam playback video (for PreviewCanvas overlay)
    var webcamPlaybackImage: NSImage?

    /// Serialises webcam start/stop so rapid toggles or mode changes
    /// cannot race (e.g. stop still in-flight when start is requested).
    private var webcamLifecycleTask: Task<Void, Never>?

    // MARK: - Overlay Panels

    let overlayManager = OverlayManager()

    // MARK: - Zoom

    let zoomEngine = ZoomEngine()

    // MARK: - Keyboard

    let keystrokeRecorder = KeystrokeRecorder()

    /// Recorded keystroke events for the current project
    var keystrokeEvents: [KeystrokeEvent] = []

    // MARK: - Project

    var currentProject: Project? {
        didSet {
            // Auto-load video into player when project changes
            if let url = currentProject?.recordingURL {
                playbackEngine.loadVideo(url: url)
            }

            // Load webcam recording for synced playback if available
            if let webcamURL = currentProject?.webcamRecordingURL {
                webcamPlayer = AVPlayer(url: webcamURL)
            } else {
                webcamPlayer = nil
                webcamPlaybackImage = nil
            }

            // Auto-load cursor events if available
            if let videoURL = currentProject?.recordingURL {
                let cursorFile = videoURL.deletingPathExtension().appendingPathExtension("cursor.json")
                if FileManager.default.fileExists(atPath: cursorFile.path) {
                    cursorEvents = CursorRecorder.loadEvents(from: cursorFile)
                    logger.info("Loaded \(self.cursorEvents.count) cursor events")
                } else {
                    cursorEvents = []
                }

                // Auto-load keystroke events if available
                let keystrokeFile = videoURL.deletingPathExtension().appendingPathExtension("keystrokes.json")
                if FileManager.default.fileExists(atPath: keystrokeFile.path) {
                    keystrokeEvents = KeystrokeRecorder.loadEvents(from: keystrokeFile)
                    logger.info("Loaded \(self.keystrokeEvents.count) keystroke events")
                } else {
                    keystrokeEvents = []
                }
            } else {
                cursorEvents = []
                keystrokeEvents = []
            }
        }
    }

    /// Recorded cursor events for the current project
    var cursorEvents: [CursorEvent] = []

    // MARK: - Main Window

    /// Reference to the main NSWindow, managed by FrameApp.
    /// Hidden in recorder mode; shown in editor mode.
    var mainWindowController: NSWindow?

    // MARK: - UI State

    var showExportSheet = false
    var showSettings = false
    var selectedInspectorTab: InspectorTab = .background
    var recordingError: RecordingAppError?
    var showErrorAlert = false

    enum InspectorTab: String, CaseIterable, Identifiable {
        case background = "Background"
        case cursor = "Cursor"
        case keyboard = "Keyboard"
        case camera = "Camera"
        case zoom = "Zoom"
        case audio = "Audio"

        var id: String { rawValue }

        var icon: String {
            switch self {
            case .background: return "photo.on.rectangle"
            case .cursor: return "cursorarrow.motionlines"
            case .keyboard: return "keyboard"
            case .camera: return "camera.fill"
            case .zoom: return "plus.magnifyingglass"
            case .audio: return "speaker.wave.2.fill"
            }
        }
    }

    // MARK: - Error Type

    struct RecordingAppError: Identifiable {
        let id = UUID()
        let title: String
        let message: String
        let showOpenSettings: Bool
        let showRestartHint: Bool

        init(title: String, message: String, showOpenSettings: Bool = false, showRestartHint: Bool = false) {
            self.title = title
            self.message = message
            self.showOpenSettings = showOpenSettings
            self.showRestartHint = showRestartHint
        }
    }

    // MARK: - Observation Bridge

    /// Combine subscriptions to bridge ObservableObject → @Observable
    private var cancellables = Set<AnyCancellable>()

    init() {
        // Bridge coordinator's @Published changes to trigger @Observable updates.
        // When coordinator publishes changes, we manually signal observation.
        coordinator.objectWillChange
            .receive(on: DispatchQueue.main)
            .sink { [weak self] (_: Void) in
                // Touch a property to trigger observation tracking
                self?._observationBump += 1
            }
            .store(in: &cancellables)

        // Bridge screenRecorder's @Published changes (nested ObservableObject).
        // Without this, changes to permissionDenied don't propagate to SwiftUI views.
        coordinator.screenRecorder.objectWillChange
            .receive(on: DispatchQueue.main)
            .sink { [weak self] (_: Void) in
                self?._observationBump += 1
            }
            .store(in: &cancellables)

        // Bridge export engine's @Published changes too
        exportEngine.objectWillChange
            .receive(on: DispatchQueue.main)
            .sink { [weak self] (_: Void) in
                self?._observationBump += 1
            }
            .store(in: &cancellables)

        // Bridge webcam engine changes
        webcamEngine.objectWillChange
            .receive(on: DispatchQueue.main)
            .sink { [weak self] (_: Void) in
                self?._observationBump += 1
            }
            .store(in: &cancellables)

        // Note: During recording, webcam preview rendering is handled directly
        // by CIImageView using CVDisplayLink + CIContext, bypassing the main thread.
        // For editor mode, we still need NSImage conversion for SwiftUI preview views.
        webcamEngine.$latestFrame
            .compactMap { $0 }
            .throttle(for: .milliseconds(33), scheduler: DispatchQueue.main, latest: true)  // ~30fps
            .sink { [weak self] ciImage in
                guard let self else { return }
                // Only convert to NSImage in editor mode — during recording,
                // CIImageView reads from frameBox directly (GPU-backed, no main thread)
                if self.mode == .editor {
                    self.webcamImage = WebcamOverlayView.convertToNSImage(ciImage)
                }
            }
            .store(in: &cancellables)

        // Bridge zoom engine changes
        zoomEngine.objectWillChange
            .receive(on: DispatchQueue.main)
            .sink { [weak self] (_: Void) in
                self?._observationBump += 1
            }
            .store(in: &cancellables)

        // Sync webcam player with main playback (play/pause/seek)
        playbackEngine.$isPlaying
            .receive(on: DispatchQueue.main)
            .sink { [weak self] (playing: Bool) in
                guard let self, let webcamPlayer = self.webcamPlayer else { return }
                if playing {
                    webcamPlayer.play()
                } else {
                    webcamPlayer.pause()
                }
            }
            .store(in: &cancellables)

        // Sync seek position: when main player time changes significantly, seek webcam too
        playbackEngine.$currentTime
            .receive(on: DispatchQueue.main)
            .throttle(for: .milliseconds(100), scheduler: DispatchQueue.main, latest: true)
            .sink { [weak self] (time: TimeInterval) in
                guard let self, let webcamPlayer = self.webcamPlayer else { return }
                let webcamTime = webcamPlayer.currentTime().seconds
                // Only seek if drifted more than 0.15s (avoid constant seeking during playback)
                if abs(webcamTime - time) > 0.15 {
                    let cmTime = CMTime(seconds: time, preferredTimescale: 600)
                    webcamPlayer.seek(to: cmTime, toleranceBefore: .zero, toleranceAfter: .zero)
                }
            }
            .store(in: &cancellables)
    }

    /// Private counter to force @Observable to re-evaluate computed properties.
    private var _observationBump: Int = 0

    // MARK: - Source Management

    func refreshSources() async {
        await coordinator.refreshSources()
    }

    // MARK: - Webcam Actions

    /// Toggle webcam on/off, and update the overlay webcam preview panel accordingly.
    func toggleWebcam() {
        let shouldStop = webcamEngine.isRunning
        let previous = webcamLifecycleTask
        webcamLifecycleTask = Task { [weak self] in
            // Wait for any in-flight webcam operation to finish first
            await previous?.value
            guard let self else { return }
            if shouldStop {
                await self.webcamEngine.stop()
                self.webcamImage = nil
                logger.info("Webcam stopped via toggle")
            } else {
                await self.webcamEngine.start()
                logger.info("Webcam started via toggle, isRunning=\(self.webcamEngine.isRunning)")
            }
            self.overlayManager.updateWebcamVisibility(appState: self)
        }
    }

    var availableDisplays: [SCDisplay] { coordinator.availableDisplays }
    var availableWindows: [SCWindow] { coordinator.availableWindows }

    // MARK: - Recording Actions

    func startRecording() {
        Task {
            await startRecordingAsync()
        }
    }

    func startRecordingAsync() async {
        guard !isStartingRecording else { return }
        isStartingRecording = true
        defer { isStartingRecording = false }

        // Preflight: refresh permission state and block if denied
        await refreshSources()
        let denied = screenRecordingPermissionDenied
        let rawDenied = coordinator.screenRecorder.permissionDenied
        print("[Frame] Preflight permission check — screenRecordingPermissionDenied: \(denied), raw: \(rawDenied)")
        logger.info("Preflight permission check — denied: \(denied), raw: \(rawDenied)")
        if denied {
            logger.warning("Screen recording permission denied — aborting start")

            if mode == .recorder {
                showMainWindow()
            }

            recordingError = RecordingAppError(
                title: "Permission Required",
                message: "Screen recording permission is required. Please enable it in System Settings → Privacy & Security → Screen Recording, then restart Frame.",
                showOpenSettings: true,
                showRestartHint: true
            )
            showErrorAlert = true
            return
        }

        do {
            print("[Frame] Starting recording...")
            logger.info("Starting recording...")

            // Give the coordinator access to the webcam engine for separate recording
            coordinator.webcamEngine = webcamEngine

            try await coordinator.startRecording(
                recordWebcam: webcamEngine.isRunning
            )
            print("[Frame] Recording started successfully")
            logger.info("Recording started successfully")
        } catch let error as RecordingError {
            print("[Frame] Recording error: \(error.localizedDescription)")
            logger.error("Failed to start recording: \(error.localizedDescription)")
            let isPermissionIssue = (error == .screenRecordingPermissionDenied || error == .noDisplayAvailable)

            // Show main window so the user can see the alert — in recorder mode
            // the window is hidden, which swallows any SwiftUI .alert modifiers.
            if mode == .recorder {
                showMainWindow()
            }

            recordingError = RecordingAppError(
                title: "Recording Failed",
                message: error.recoverySuggestion ?? error.localizedDescription,
                showOpenSettings: isPermissionIssue,
                showRestartHint: error == .noDisplayAvailable
            )
            showErrorAlert = true
        } catch {
            logger.error("Failed to start recording: \(error.localizedDescription)")

            if mode == .recorder {
                showMainWindow()
            }

            recordingError = RecordingAppError(
                title: "Recording Failed",
                message: error.localizedDescription
            )
            showErrorAlert = true
        }
    }

    func stopRecording() {
        Task {
            await stopRecordingAsync()
        }
    }

    func stopRecordingAsync() async {
        // If recording was never actually started (e.g. permission denied),
        // don't show the misleading "no video file" error.
        guard coordinator.isRecording else {
            logger.warning("stopRecording called but not recording — ignoring")
            return
        }

        logger.info("Stopping recording...")

        let project = await coordinator.stopRecording()

        if let project {
            currentProject = project
            mode = .editor
            logger.info("Recording stopped, project created: \(project.name)")
        } else {
            logger.warning("Recording stopped but no project was created")

            // Show main window so the alert is visible
            if mode == .recorder {
                showMainWindow()
            }

            recordingError = RecordingAppError(
                title: screenRecordingPermissionDenied ? "Permission Denied" : "Recording Error",
                message: screenRecordingPermissionDenied
                    ? "Screen recording permission was denied. No video was captured. Please enable it in System Settings → Privacy & Security → Screen Recording, then restart Frame."
                    : "The recording completed but no video file was saved. The screen capture may have been interrupted.",
                showOpenSettings: screenRecordingPermissionDenied,
                showRestartHint: screenRecordingPermissionDenied
            )
            showErrorAlert = true
        }
    }

    func togglePause() {
        guard isRecording else { return }
        // TODO: Phase 3 — implement pause/resume via SCStream
    }

    // MARK: - Mode Lifecycle

    private func handleModeChange(from oldMode: Mode, to newMode: Mode) {
        if newMode == .editor {
            // Stop live webcam (we don't need it in editor — we play the recorded file)
            if webcamEngine.isRunning {
                let previous = webcamLifecycleTask
                webcamLifecycleTask = Task { [weak self] in
                    await previous?.value
                    guard let self else { return }
                    await self.webcamEngine.stop()
                    self.webcamImage = nil
                    logger.info("Webcam stopped (entering editor mode)")
                }
            }

            // Load webcam recording for playback in editor if available
            if let webcamURL = currentProject?.webcamRecordingURL {
                webcamPlayer = AVPlayer(url: webcamURL)
                logger.info("Webcam video loaded for editor playback")
            } else {
                webcamPlayer = nil
                webcamPlaybackImage = nil
            }
        } else if oldMode == .editor {
            // Leaving editor: clean up webcam playback
            webcamPlayer = nil
            webcamPlaybackImage = nil

            // Also stop live webcam if somehow still running
            let previous = webcamLifecycleTask
            webcamLifecycleTask = Task { [weak self] in
                await previous?.value
                guard let self else { return }
                await self.webcamEngine.stop()
                self.webcamImage = nil
                logger.info("Webcam stopped (left editor mode)")
            }
        }

        // Show/hide overlay panels based on mode
        if newMode == .recorder {
            // Hide main window, show floating panels only
            hideMainWindow()
            overlayManager.showOverlays(appState: self)
        } else if newMode == .editor {
            // Hide floating panels, show main window with editor
            overlayManager.hideOverlays()
            showMainWindow()
        }
    }

    // MARK: - Main Window Management

    /// Hide the main window — in recorder mode, only floating panels are visible.
    func hideMainWindow() {
        guard let window = mainWindowController else {
            logger.warning("hideMainWindow: no mainWindowController reference yet")
            return
        }
        window.orderOut(nil)
        logger.info("Main window hidden (recorder mode)")
    }

    /// Show the main window — after recording finishes, show the editor.
    func showMainWindow() {
        guard let window = mainWindowController else {
            logger.warning("showMainWindow: no mainWindowController reference")
            return
        }
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        logger.info("Main window shown (editor mode)")
    }

    // MARK: - Initial Setup

    /// Call once after the main window reference is captured by WindowAccessor.
    /// Hides the main window and shows floating overlays for recorder mode.
    func showInitialOverlays() {
        guard mode == .recorder else { return }
        guard mainWindowController != nil else {
            logger.warning("showInitialOverlays: called before window reference was set")
            return
        }
        hideMainWindow()
        overlayManager.showOverlays(appState: self)
        logger.info("Initial overlay panels shown, main window hidden")

        // Refresh permission state so the toolbar banner is accurate
        Task {
            await refreshSources()
        }
    }

    func switchToRecorder() {
        mode = .recorder
    }

    func switchToEditor() {
        guard currentProject != nil else { return }
        mode = .editor
    }
}
