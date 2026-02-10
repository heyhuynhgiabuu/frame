import SwiftUI
import ScreenCaptureKit
import AVFoundation
import Combine
import OSLog
import AppKit
import UniformTypeIdentifiers

private let logger = Logger(subsystem: "com.frame.app", category: "AppState")

struct RecorderToolbarSettings {
    enum CaptureMode: String, CaseIterable, Identifiable {
        case display
        case window
        case area
        case device

        var id: String { rawValue }
    }

    enum Countdown: Int, CaseIterable, Identifiable {
        case none = 0
        case three = 3
        case five = 5
        case ten = 10

        var id: Int { rawValue }

        var label: String {
            switch self {
            case .none: return "No countdown"
            case .three: return "3s"
            case .five: return "5s"
            case .ten: return "10s"
            }
        }
    }

    enum RecordingEngineMode: String, CaseIterable, Identifiable {
        case modernAuto
        case forceModern
        case forceLegacy

        var id: String { rawValue }

        var label: String {
            switch self {
            case .modernAuto: return "Use modern recording engine if available"
            case .forceModern: return "Force modern recording engine"
            case .forceLegacy: return "Force legacy recording engine"
            }
        }
    }

    enum CameraResolution: String, CaseIterable, Identifiable {
        case p720 = "720p"
        case p1080 = "1080p"
        case p4k = "4k"

        var id: String { rawValue }

        var label: String {
            switch self {
            case .p720: return "720p"
            case .p1080: return "1080p"
            case .p4k: return "4k"
            }
        }

        var preset: AVCaptureSession.Preset {
            switch self {
            case .p720: return .hd1280x720
            case .p1080: return .hd1920x1080
            case .p4k: return .hd4K3840x2160
            }
        }
    }

    enum AreaAspectRatio: String, CaseIterable, Identifiable {
        case any
        case ratio16x9 = "16:9"
        case ratio4x3 = "4:3"
        case ratio1x1 = "1:1"
        case ratio3x4 = "3:4"
        case ratio9x16 = "9:16"
        case custom

        var id: String { rawValue }

        var label: String {
            switch self {
            case .any: return "Any"
            case .ratio16x9: return "16:9"
            case .ratio4x3: return "4:3"
            case .ratio1x1: return "1:1"
            case .ratio3x4: return "3:4"
            case .ratio9x16: return "9:16"
            case .custom: return "Custom aspect ratio"
            }
        }

        var icon: String {
            switch self {
            case .any: return "rectangle.dashed"
            case .ratio16x9: return "rectangle"
            case .ratio4x3: return "rectangle.portrait"
            case .ratio1x1: return "square"
            case .ratio3x4: return "rectangle.portrait"
            case .ratio9x16: return "rectangle.portrait"
            case .custom: return "aspectratio"
            }
        }

        /// Returns the ratio as width/height, or nil for `.any` and `.custom`.
        var ratioValue: CGFloat? {
            switch self {
            case .any, .custom: return nil
            case .ratio16x9: return 16.0 / 9.0
            case .ratio4x3: return 4.0 / 3.0
            case .ratio1x1: return 1.0
            case .ratio3x4: return 3.0 / 4.0
            case .ratio9x16: return 9.0 / 16.0
            }
        }
    }

    struct SavedAreaDimension: Codable, Hashable, Identifiable {
        var id: String { "\(width)x\(height)" }
        let width: Int
        let height: Int

        var label: String { "\(width) × \(height)" }
    }

    var captureMode: CaptureMode = .display
    var cameraDeviceID: String?
    var cameraResolution: CameraResolution = .p1080
    var hideCameraPreview = false
    var fullFrameWebcamPreview = true
    var recordCamera = true
    var microphoneDeviceID: String?
    var reduceNoiseAndNormalizeVolume = false
    var disableAutoGainControl = false
    var recordMicrophone = false
    var recordSystemAudio = false
    var hideDesktopIcons = false
    var hideDockIconWhileRecording = false
    var highlightRecordedArea = false
    var openQuickShareWidgetAfterRecording = false
    var showSpeakerNotes = false
    var recordingCountdown: Countdown = .none
    var recordingEngineMode: RecordingEngineMode = .modernAuto

    // MARK: - Area capture settings
    var areaWidth: Int = 1920
    var areaHeight: Int = 1080
    var areaX: Int = 0
    var areaY: Int = 0
    var areaAspectRatio: AreaAspectRatio = .any
    var savedAreaDimensions: [SavedAreaDimension] = []

    static func load() -> RecorderToolbarSettings {
        let defaults = UserDefaults.standard
        var settings = RecorderToolbarSettings()
        if let raw = defaults.string(forKey: "toolbar.captureMode"), let mode = CaptureMode(rawValue: raw) {
            settings.captureMode = mode
        }
        settings.cameraDeviceID = defaults.string(forKey: "toolbar.camera.deviceID")
        if let raw = defaults.string(forKey: "toolbar.camera.resolution"), let value = CameraResolution(rawValue: raw) {
            settings.cameraResolution = value
        }
        if defaults.object(forKey: "toolbar.camera.hidePreview") != nil {
            settings.hideCameraPreview = defaults.bool(forKey: "toolbar.camera.hidePreview")
        }
        if defaults.object(forKey: "toolbar.camera.fullFramePreview") != nil {
            settings.fullFrameWebcamPreview = defaults.bool(forKey: "toolbar.camera.fullFramePreview")
        }
        if defaults.object(forKey: "toolbar.camera.record") != nil {
            settings.recordCamera = defaults.bool(forKey: "toolbar.camera.record")
        }
        settings.microphoneDeviceID = defaults.string(forKey: "toolbar.microphone.deviceID")
        if defaults.object(forKey: "toolbar.microphone.reduceNoiseAndNormalize") != nil {
            settings.reduceNoiseAndNormalizeVolume = defaults.bool(forKey: "toolbar.microphone.reduceNoiseAndNormalize")
        }
        if defaults.object(forKey: "toolbar.microphone.disableAutoGain") != nil {
            settings.disableAutoGainControl = defaults.bool(forKey: "toolbar.microphone.disableAutoGain")
        }
        if defaults.object(forKey: "toolbar.microphone.record") != nil {
            settings.recordMicrophone = defaults.bool(forKey: "toolbar.microphone.record")
        }
        if defaults.object(forKey: "toolbar.systemAudio.record") != nil {
            settings.recordSystemAudio = defaults.bool(forKey: "toolbar.systemAudio.record")
        }
        settings.hideDesktopIcons = defaults.bool(forKey: "toolbar.settings.hideDesktopIcons")
        settings.hideDockIconWhileRecording = defaults.bool(forKey: "toolbar.settings.hideDockIcon")
        settings.highlightRecordedArea = defaults.bool(forKey: "toolbar.settings.highlightArea")
        settings.openQuickShareWidgetAfterRecording = defaults.bool(forKey: "toolbar.settings.quickShare")
        settings.showSpeakerNotes = defaults.bool(forKey: "toolbar.settings.showSpeakerNotes")
        if let countdown = defaults.object(forKey: "toolbar.settings.countdown") as? Int,
           let value = Countdown(rawValue: countdown) {
            settings.recordingCountdown = value
        }
        if let raw = defaults.string(forKey: "toolbar.settings.recordingEngine"),
           let value = RecordingEngineMode(rawValue: raw) {
            settings.recordingEngineMode = value
        }
        // Area capture settings
        if defaults.object(forKey: "toolbar.area.width") != nil {
            settings.areaWidth = defaults.integer(forKey: "toolbar.area.width")
        }
        if defaults.object(forKey: "toolbar.area.height") != nil {
            settings.areaHeight = defaults.integer(forKey: "toolbar.area.height")
        }
        if defaults.object(forKey: "toolbar.area.x") != nil {
            settings.areaX = defaults.integer(forKey: "toolbar.area.x")
        }
        if defaults.object(forKey: "toolbar.area.y") != nil {
            settings.areaY = defaults.integer(forKey: "toolbar.area.y")
        }
        if let raw = defaults.string(forKey: "toolbar.area.aspectRatio"),
           let value = AreaAspectRatio(rawValue: raw) {
            settings.areaAspectRatio = value
        }
        if let data = defaults.data(forKey: "toolbar.area.savedDimensions"),
           let decoded = try? JSONDecoder().decode([SavedAreaDimension].self, from: data) {
            settings.savedAreaDimensions = decoded
        }
        return settings
    }

    func save() {
        let defaults = UserDefaults.standard
        defaults.set(captureMode.rawValue, forKey: "toolbar.captureMode")
        defaults.set(cameraDeviceID, forKey: "toolbar.camera.deviceID")
        defaults.set(cameraResolution.rawValue, forKey: "toolbar.camera.resolution")
        defaults.set(hideCameraPreview, forKey: "toolbar.camera.hidePreview")
        defaults.set(fullFrameWebcamPreview, forKey: "toolbar.camera.fullFramePreview")
        defaults.set(recordCamera, forKey: "toolbar.camera.record")
        defaults.set(microphoneDeviceID, forKey: "toolbar.microphone.deviceID")
        defaults.set(reduceNoiseAndNormalizeVolume, forKey: "toolbar.microphone.reduceNoiseAndNormalize")
        defaults.set(disableAutoGainControl, forKey: "toolbar.microphone.disableAutoGain")
        defaults.set(recordMicrophone, forKey: "toolbar.microphone.record")
        defaults.set(recordSystemAudio, forKey: "toolbar.systemAudio.record")
        defaults.set(hideDesktopIcons, forKey: "toolbar.settings.hideDesktopIcons")
        defaults.set(hideDockIconWhileRecording, forKey: "toolbar.settings.hideDockIcon")
        defaults.set(highlightRecordedArea, forKey: "toolbar.settings.highlightArea")
        defaults.set(openQuickShareWidgetAfterRecording, forKey: "toolbar.settings.quickShare")
        defaults.set(showSpeakerNotes, forKey: "toolbar.settings.showSpeakerNotes")
        defaults.set(recordingCountdown.rawValue, forKey: "toolbar.settings.countdown")
        defaults.set(recordingEngineMode.rawValue, forKey: "toolbar.settings.recordingEngine")
        // Area capture settings
        defaults.set(areaWidth, forKey: "toolbar.area.width")
        defaults.set(areaHeight, forKey: "toolbar.area.height")
        defaults.set(areaX, forKey: "toolbar.area.x")
        defaults.set(areaY, forKey: "toolbar.area.y")
        defaults.set(areaAspectRatio.rawValue, forKey: "toolbar.area.aspectRatio")
        if let data = try? JSONEncoder().encode(savedAreaDimensions) {
            defaults.set(data, forKey: "toolbar.area.savedDimensions")
        }
    }
}

struct WindowCaptureSizePreset: Codable, Hashable, Identifiable {
    let width: Int
    let height: Int

    var id: String { "\(width)x\(height)" }
    var label: String { "\(width) x \(height)" }
}

struct WindowCaptureSizeGroup: Identifiable {
    let title: String
    let presets: [WindowCaptureSizePreset]

    var id: String { title }
}

enum AudioInputDeviceService {
    struct InputDevice: Identifiable {
        let id: String
        let name: String
    }

    static func inputDevices() -> [InputDevice] {
        let discovery = AVCaptureDevice.DiscoverySession(
            deviceTypes: [.microphone],
            mediaType: .audio,
            position: .unspecified
        )
        return discovery.devices.map { InputDevice(id: $0.uniqueID, name: $0.localizedName) }
    }
}

/// The global application state, shared across all views.
@Observable
@MainActor
final class AppState {

    enum PostRecordingExportMode: String, CaseIterable, Identifiable {
        case none
        case clipboard
        case shareableLink
        case saveToFile

        var id: String { rawValue }
    }

    struct PostRecordingSettingsSnapshot {
        var createProject: Bool
        var autoCreateZooms: Bool
        var exportMode: PostRecordingExportMode
        var quickExportFormat: ExportConfig.ExportFormat
        var quickExportQuality: ExportConfig.ExportQuality
        var quickExportResolution: ExportConfig.ExportResolution
        var quickExportFrameRate: ExportConfig.ExportFrameRate
    }

    // MARK: - Mode

    enum Mode: String, CaseIterable {
        case recorder
        case editor
    }

    static let windowCaptureSizeGroups: [WindowCaptureSizeGroup] = [
        WindowCaptureSizeGroup(
            title: "16:9",
            presets: [
                WindowCaptureSizePreset(width: 1280, height: 720),
                WindowCaptureSizePreset(width: 1920, height: 1080),
                WindowCaptureSizePreset(width: 2560, height: 1440),
            ]
        ),
        WindowCaptureSizeGroup(
            title: "4:3",
            presets: [
                WindowCaptureSizePreset(width: 640, height: 480),
                WindowCaptureSizePreset(width: 800, height: 600),
                WindowCaptureSizePreset(width: 1024, height: 768),
                WindowCaptureSizePreset(width: 1280, height: 960),
                WindowCaptureSizePreset(width: 1600, height: 1200),
            ]
        ),
        WindowCaptureSizeGroup(
            title: "9:16",
            presets: [
                WindowCaptureSizePreset(width: 720, height: 1280),
                WindowCaptureSizePreset(width: 900, height: 1600),
                WindowCaptureSizePreset(width: 1080, height: 1920),
                WindowCaptureSizePreset(width: 1440, height: 2560),
            ]
        ),
        WindowCaptureSizeGroup(
            title: "16:10",
            presets: [
                WindowCaptureSizePreset(width: 640, height: 400),
                WindowCaptureSizePreset(width: 800, height: 500),
                WindowCaptureSizePreset(width: 1024, height: 640),
                WindowCaptureSizePreset(width: 1280, height: 800),
                WindowCaptureSizePreset(width: 1440, height: 900),
                WindowCaptureSizePreset(width: 1680, height: 1050),
                WindowCaptureSizePreset(width: 1920, height: 1200),
                WindowCaptureSizePreset(width: 2560, height: 1600),
            ]
        ),
        WindowCaptureSizeGroup(
            title: "Square",
            presets: [
                WindowCaptureSizePreset(width: 640, height: 640),
                WindowCaptureSizePreset(width: 800, height: 800),
                WindowCaptureSizePreset(width: 1024, height: 1024),
                WindowCaptureSizePreset(width: 1280, height: 1280),
                WindowCaptureSizePreset(width: 1600, height: 1600),
            ]
        ),
    ]

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

    var isPaused: Bool {
        _ = _observationBump
        return coordinator.isPaused
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
    let areaSelectionManager = AreaSelectionManager()
    var menuBarManager: MenuBarManager?

    var recorderToolbarSettings = RecorderToolbarSettings.load() {
        didSet {
            recorderToolbarSettings.save()
            applyRecorderToolbarSettingsToRuntime()
            menuBarManager?.refresh()
        }
    }

    // MARK: - Zoom

    let zoomEngine = ZoomEngine()

    // MARK: - Mic Level Monitor

    let micLevelMonitor = MicLevelMonitor()
    var micLevel: Float = 0
    private var micLevelCancellable: AnyCancellable?

    /// Start monitoring mic levels for the selected device.
    func startMicLevelMonitoring() {
        let deviceID = recorderToolbarSettings.microphoneDeviceID
        micLevelMonitor.start(deviceID: deviceID)
        micLevelCancellable = micLevelMonitor.$level
            .receive(on: DispatchQueue.main)
            .sink { [weak self] newLevel in
                self?.micLevel = newLevel
            }
    }

    /// Stop monitoring mic levels.
    func stopMicLevelMonitoring() {
        micLevelMonitor.stop()
        micLevelCancellable?.cancel()
        micLevelCancellable = nil
        micLevel = 0
    }

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
    var showQuickExportSettings = false
    var hideWindowAfterQuickExportSheet = false
    var selectedInspectorTab: InspectorTab = .background
    var recordingError: RecordingAppError?
    var showErrorAlert = false
    var hasUserExplicitlySelectedCaptureModeForCard = false
    var hasUserManuallySelectedDisplayForSession = false
    var isAwaitingDisplayClickSelection = false
    var isAwaitingWindowClickSelection = false
    var hoveredDisplayForSelection: SCDisplay?
    var hoveredWindowForSelection: SCWindow?
    var selectedWindowOutputSize: WindowCaptureSizePreset?
    var savedWindowOutputSizes: [WindowCaptureSizePreset] = []
    var showWindowCustomSizeSheet = false
    var customWindowOutputWidth = 1280
    var customWindowOutputHeight = 720

    // MARK: - Post-Recording Settings

    var postCreateProject: Bool = true {
        didSet { UserDefaults.standard.set(postCreateProject, forKey: "postRecording.createProject") }
    }

    var postAutoCreateZooms: Bool = true {
        didSet { UserDefaults.standard.set(postAutoCreateZooms, forKey: "postRecording.autoCreateZooms") }
    }

    var postExportMode: PostRecordingExportMode = .none {
        didSet { UserDefaults.standard.set(postExportMode.rawValue, forKey: "postRecording.exportMode") }
    }

    var quickExportFormat: ExportConfig.ExportFormat = .mp4 {
        didSet { UserDefaults.standard.set(quickExportFormat.rawValue, forKey: "postRecording.quickExport.format") }
    }

    var quickExportQuality: ExportConfig.ExportQuality = .high {
        didSet { UserDefaults.standard.set(quickExportQuality.rawValue, forKey: "postRecording.quickExport.quality") }
    }

    var quickExportResolution: ExportConfig.ExportResolution = .original {
        didSet { UserDefaults.standard.set(quickExportResolution.rawValue, forKey: "postRecording.quickExport.resolution") }
    }

    var quickExportFrameRate: ExportConfig.ExportFrameRate = .fps30 {
        didSet { UserDefaults.standard.set(quickExportFrameRate.rawValue, forKey: "postRecording.quickExport.frameRate") }
    }

    private var activePostRecordingSettings: PostRecordingSettingsSnapshot?
    private var recordingActionInFlight = false
    private var displaySelectionGlobalEventMonitor: Any?
    private var displaySelectionLocalEventMonitor: Any?
    private var displaySelectionTrackingTimer: Timer?
    private static let windowOutputSizeWidthDefaultsKey = "toolbar.window.outputSize.width"
    private static let windowOutputSizeHeightDefaultsKey = "toolbar.window.outputSize.height"
    private static let savedWindowOutputSizesDefaultsKey = "toolbar.window.savedOutputSizes"

    enum InspectorTab: String, CaseIterable, Identifiable {
        case background = "Background"
        case cursor = "Cursor"
        case keyboard = "Keyboard"
        case camera = "Camera"
        case zoom = "Zoom"
        case audio = "Audio"
        case captions = "Captions"

        var id: String { rawValue }

        var icon: String {
            switch self {
            case .background: return "photo.on.rectangle"
            case .cursor: return "cursorarrow.motionlines"
            case .keyboard: return "keyboard"
            case .camera: return "camera.fill"
            case .zoom: return "plus.magnifyingglass"
            case .audio: return "speaker.wave.2.fill"
            case .captions: return "captions.bubble.fill"
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
        loadPostRecordingSettings()
        loadWindowCaptureSizeSettings()
        applyRecorderToolbarSettingsToRuntime()
        applyWindowOutputSizeToRuntime()

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
        syncDisplaySelectionWithAvailableContent()
        syncWindowSelectionWithAvailableContent()
        overlayManager.updateDisplayCardVisibility(appState: self)
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
            self.menuBarManager?.refresh()
        }
    }

    func hideRecorderOverlays() {
        overlayManager.hideOverlays()
        areaSelectionManager.dismiss()
        stopMicLevelMonitoring()
        hasUserExplicitlySelectedCaptureModeForCard = false
        isAwaitingDisplayClickSelection = false
        isAwaitingWindowClickSelection = false
        stopDisplaySelectionTracking(clearHover: true)
        hideMainWindow()
        menuBarManager?.refresh()
    }

    func showRecorderOverlays() {
        guard mode == .recorder else { return }
        overlayManager.showOverlays(appState: self)
        menuBarManager?.refresh()
    }

    func bringOverlaysToFront() {
        overlayManager.bringToFront()
    }

    var areOverlaysVisible: Bool {
        overlayManager.isShowing
    }

    func setCaptureMode(_ mode: RecorderToolbarSettings.CaptureMode) {
        recorderToolbarSettings.captureMode = mode
        hasUserExplicitlySelectedCaptureModeForCard = true

        switch mode {
        case .display:
            isAwaitingWindowClickSelection = false
            hoveredWindowForSelection = nil
            beginDisplayClickSelection()
        case .window:
            isAwaitingDisplayClickSelection = false
            hoveredDisplayForSelection = nil
            beginWindowClickSelection()
        case .area:
            isAwaitingDisplayClickSelection = false
            isAwaitingWindowClickSelection = false
            stopDisplaySelectionTracking(clearHover: true)
            beginAreaSelection()
        default:
            isAwaitingDisplayClickSelection = false
            isAwaitingWindowClickSelection = false
            stopDisplaySelectionTracking(clearHover: true)
        }

        overlayManager.updateDisplayCardVisibility(appState: self)
    }

    func setSelectedDisplay(_ display: SCDisplay) {
        coordinator.config.selectedDisplay = display
        hoveredDisplayForSelection = display
        hasUserManuallySelectedDisplayForSession = true
        isAwaitingDisplayClickSelection = false
        isAwaitingWindowClickSelection = false
        stopDisplaySelectionTracking()
        overlayManager.updateDisplayCardVisibility(appState: self)
    }

    func setSelectedWindow(_ window: SCWindow) {
        coordinator.config.selectedWindow = window
        hoveredWindowForSelection = window
        isAwaitingWindowClickSelection = false
        isAwaitingDisplayClickSelection = false
        stopDisplaySelectionTracking()
        overlayManager.updateDisplayCardVisibility(appState: self)
    }

    func setCameraDevice(_ id: String?) {
        recorderToolbarSettings.cameraDeviceID = id
        webcamEngine.selectedCameraID = id

        if webcamEngine.isRunning {
            Task {
                await webcamEngine.stop()
                await webcamEngine.start()
                overlayManager.updateWebcamVisibility(appState: self)
            }
        }
    }

    func setCameraResolution(_ resolution: RecorderToolbarSettings.CameraResolution) {
        recorderToolbarSettings.cameraResolution = resolution
        webcamEngine.maxResolution = resolution

        if webcamEngine.isRunning {
            Task {
                await webcamEngine.stop()
                await webcamEngine.start()
                overlayManager.updateWebcamVisibility(appState: self)
            }
        }
    }

    func setHideCameraPreview(_ hide: Bool) {
        recorderToolbarSettings.hideCameraPreview = hide
        overlayManager.updateWebcamVisibility(appState: self)
    }

    func setRecordCamera(_ enabled: Bool) {
        recorderToolbarSettings.recordCamera = enabled
    }

    func setMicrophoneDevice(_ id: String?) {
        recorderToolbarSettings.microphoneDeviceID = id
    }

    func setRecordMicrophone(_ enabled: Bool) {
        recorderToolbarSettings.recordMicrophone = enabled
    }

    func setRecordSystemAudio(_ enabled: Bool) {
        recorderToolbarSettings.recordSystemAudio = enabled
    }

    func setReduceNoiseAndNormalize(_ enabled: Bool) {
        recorderToolbarSettings.reduceNoiseAndNormalizeVolume = enabled
    }

    func setDisableAutoGainControl(_ enabled: Bool) {
        recorderToolbarSettings.disableAutoGainControl = enabled
    }

    // MARK: - Area capture helpers

    /// Adjusts area height to match the current aspect ratio (after width changes).
    func applyAreaAspectRatioFromWidth() {
        guard let ratio = recorderToolbarSettings.areaAspectRatio.ratioValue else { return }
        let newHeight = Int(round(Double(recorderToolbarSettings.areaWidth) / ratio))
        recorderToolbarSettings.areaHeight = max(1, newHeight)
    }

    /// Adjusts area width to match the current aspect ratio (after height changes).
    func applyAreaAspectRatioFromHeight() {
        guard let ratio = recorderToolbarSettings.areaAspectRatio.ratioValue else { return }
        let newWidth = Int(round(Double(recorderToolbarSettings.areaHeight) * ratio))
        recorderToolbarSettings.areaWidth = max(1, newWidth)
    }

    /// Saves the current area width×height to saved dimensions.
    func saveCurrentAreaDimension() {
        let dim = RecorderToolbarSettings.SavedAreaDimension(
            width: recorderToolbarSettings.areaWidth,
            height: recorderToolbarSettings.areaHeight
        )
        // Avoid duplicates
        if !recorderToolbarSettings.savedAreaDimensions.contains(dim) {
            recorderToolbarSettings.savedAreaDimensions.append(dim)
            recorderToolbarSettings.save()
        }
    }

    /// Launches the full-screen area selection overlay for click-drag region picking.
    func beginAreaSelection() {
        areaSelectionManager.startSelection(appState: self)
    }

    /// Dismisses the area selection overlay if active.
    func cancelAreaSelection() {
        areaSelectionManager.dismiss()
    }

    var availableDisplays: [SCDisplay] { coordinator.availableDisplays }
    var availableWindows: [SCWindow] { coordinator.availableWindows }

    private var displayPreviewForCard: SCDisplay? {
        if recorderToolbarSettings.captureMode == .display, isAwaitingDisplayClickSelection {
            return hoveredDisplayForSelection ?? coordinator.config.selectedDisplay ?? availableDisplays.first
        }
        return coordinator.config.selectedDisplay
    }

    private var windowPreviewForCard: SCWindow? {
        if recorderToolbarSettings.captureMode == .window, isAwaitingWindowClickSelection {
            return hoveredWindowForSelection ?? coordinator.config.selectedWindow ?? availableWindows.first
        }
        return coordinator.config.selectedWindow
    }

    var selectedDisplayNameForToolbar: String {
        if recorderToolbarSettings.captureMode == .display, isAwaitingDisplayClickSelection {
            if let previewDisplay = displayPreviewForCard {
                return displayName(for: previewDisplay)
            }
            return "Click a display"
        }

        guard let selectedDisplay = coordinator.config.selectedDisplay else {
            return "Select display"
        }

        return displayName(for: selectedDisplay)
    }

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

    func isSelectedDisplay(_ display: SCDisplay) -> Bool {
        coordinator.config.selectedDisplay?.displayID == display.displayID
    }

    var isSelectedDisplayActive: Bool {
        recorderToolbarSettings.captureMode == .display && hasUserExplicitlySelectedCaptureModeForCard
    }

    var selectedDisplayResolutionForToolbar: String {
        if recorderToolbarSettings.captureMode == .display,
           isAwaitingDisplayClickSelection,
           let previewDisplay = displayPreviewForCard {
            return "\(previewDisplay.width)x\(previewDisplay.height)"
        }

        guard let selectedDisplay = coordinator.config.selectedDisplay else {
            return "--"
        }
        return "\(selectedDisplay.width)x\(selectedDisplay.height)"
    }

    var selectedDisplayFPSForToolbar: String {
        "\(coordinator.config.frameRate)FPS"
    }

    var selectedWindowNameForToolbar: String {
        if recorderToolbarSettings.captureMode == .window, isAwaitingWindowClickSelection {
            if let previewWindow = windowPreviewForCard {
                return windowName(for: previewWindow)
            }
            return "Click a window"
        }

        guard let selectedWindow = coordinator.config.selectedWindow else {
            return "Select window"
        }
        return windowName(for: selectedWindow)
    }

    var selectedWindowAppNameForCard: String {
        guard let window = windowPreviewForCard else {
            return "Window"
        }

        if let appName = window.owningApplication?.applicationName.trimmingCharacters(in: .whitespacesAndNewlines),
           !appName.isEmpty {
            return appName
        }

        return windowName(for: window)
    }

    var selectedWindowAppIconForCard: NSImage? {
        guard let window = windowPreviewForCard else {
            return nil
        }
        return windowAppIcon(for: window)
    }

    var windowResizeButtonTitle: String {
        selectedWindowOutputSize?.label ?? "Resize"
    }

    var hasSavedWindowOutputSizes: Bool {
        !savedWindowOutputSizes.isEmpty
    }

    var canSaveCurrentWindowSizePreset: Bool {
        currentWindowNativeSizePreset != nil
    }

    var allWindowPresetGroups: [WindowCaptureSizeGroup] {
        Self.windowCaptureSizeGroups
    }

    func windowName(for window: SCWindow) -> String {
        let trimmedTitle = window.title?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if !trimmedTitle.isEmpty {
            return trimmedTitle
        }

        if let appName = window.owningApplication?.applicationName,
           !appName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return appName
        }

        return "Window"
    }

    func windowAppIcon(for window: SCWindow) -> NSImage? {
        guard let application = window.owningApplication else {
            return nil
        }

        let processIdentifier = pid_t(application.processID)
        guard processIdentifier > 0 else {
            return nil
        }

        return NSRunningApplication(processIdentifier: processIdentifier)?.icon
    }

    func isSelectedWindowOutputSize(_ preset: WindowCaptureSizePreset) -> Bool {
        selectedWindowOutputSize == preset
    }

    func selectWindowOutputSize(_ preset: WindowCaptureSizePreset) {
        guard let normalized = normalizedWindowCaptureSize(width: preset.width, height: preset.height) else {
            return
        }

        selectedWindowOutputSize = normalized
        applyWindowOutputSizeToRuntime()
        persistWindowCaptureSizeSettings()
        overlayManager.updateDisplayCardVisibility(appState: self)
    }

    func saveCurrentWindowOutputSizePreset() {
        guard let currentPreset = currentWindowNativeSizePreset else {
            return
        }

        if !savedWindowOutputSizes.contains(currentPreset) {
            savedWindowOutputSizes.append(currentPreset)
            savedWindowOutputSizes.sort { lhs, rhs in
                if lhs.width == rhs.width {
                    return lhs.height < rhs.height
                }
                return lhs.width < rhs.width
            }
            persistWindowCaptureSizeSettings()
        }
    }

    func openCustomWindowSizeSheet() {
        let baseSize = selectedWindowOutputSize ?? currentWindowNativeSizePreset ?? WindowCaptureSizePreset(width: 1280, height: 720)
        customWindowOutputWidth = baseSize.width
        customWindowOutputHeight = baseSize.height
        showWindowCustomSizeSheet = true
    }

    func applyCustomWindowSize() {
        guard let customPreset = normalizedWindowCaptureSize(width: customWindowOutputWidth, height: customWindowOutputHeight) else {
            return
        }
        selectWindowOutputSize(customPreset)
        showWindowCustomSizeSheet = false
    }

    private var currentWindowNativeSizePreset: WindowCaptureSizePreset? {
        guard let window = windowPreviewForCard else {
            return nil
        }

        return normalizedWindowCaptureSize(
            width: Int(window.frame.width.rounded()),
            height: Int(window.frame.height.rounded())
        )
    }

    private func normalizedWindowCaptureSize(width: Int, height: Int) -> WindowCaptureSizePreset? {
        guard width > 0, height > 0 else {
            return nil
        }

        let clampedWidth = min(max(width, 64), 8192)
        let clampedHeight = min(max(height, 64), 8192)
        return WindowCaptureSizePreset(width: clampedWidth, height: clampedHeight)
    }

    func isSelectedWindow(_ window: SCWindow) -> Bool {
        coordinator.config.selectedWindow?.windowID == window.windowID
    }

    var isSelectedWindowActive: Bool {
        recorderToolbarSettings.captureMode == .window && hasUserExplicitlySelectedCaptureModeForCard
    }

    var isAwaitingCaptureClickSelection: Bool {
        switch recorderToolbarSettings.captureMode {
        case .display:
            return isAwaitingDisplayClickSelection
        case .window:
            return isAwaitingWindowClickSelection
        default:
            return false
        }
    }

    var displaySelectionHintForCard: String? {
        switch recorderToolbarSettings.captureMode {
        case .display where isAwaitingDisplayClickSelection:
            return "Click a display to confirm"
        case .window where isAwaitingWindowClickSelection:
            return "Click a window to confirm"
        default:
            return nil
        }
    }

    var canStartRecordingFromCard: Bool {
        guard !isStartingRecording, !screenRecordingPermissionDenied else {
            return false
        }

        switch recorderToolbarSettings.captureMode {
        case .display:
            return !isAwaitingDisplayClickSelection && coordinator.config.selectedDisplay != nil
        case .window:
            return !isAwaitingWindowClickSelection && coordinator.config.selectedWindow != nil
        default:
            return true
        }
    }

    var selectedCaptureTitleForCard: String {
        switch recorderToolbarSettings.captureMode {
        case .display:
            return selectedDisplayNameForToolbar
        case .window:
            if let window = windowPreviewForCard {
                return windowName(for: window)
            }
            return "Window"
        case .area:
            return "Selected Area"
        case .device:
            return "Device Capture"
        }
    }

    var selectedCaptureResolutionForCard: String {
        switch recorderToolbarSettings.captureMode {
        case .display:
            return selectedDisplayResolutionForToolbar
        case .window:
            if let outputSize = selectedWindowOutputSize {
                return outputSize.label
            }
            guard let window = windowPreviewForCard else {
                return "--"
            }
            return "\(Int(window.frame.width)) x \(Int(window.frame.height))"
        case .area:
            return selectedDisplayResolutionForToolbar
        case .device:
            return "--"
        }
    }

    var selectedCaptureScreen: NSScreen? {
        switch recorderToolbarSettings.captureMode {
        case .display:
            if let previewDisplay = displayPreviewForCard ?? availableDisplays.first {
                return screenForDisplay(previewDisplay) ?? NSScreen.main ?? NSScreen.screens.first
            }
        case .window:
            if let windowFrame = windowPreviewForCard?.frame {
                let center = CGPoint(x: windowFrame.midX, y: windowFrame.midY)
                if let screen = NSScreen.screens.first(where: { $0.frame.contains(center) }) {
                    return screen
                }
            }
            if isAwaitingWindowClickSelection,
               let hoveredScreen = NSScreen.screens.first(where: { $0.frame.contains(NSEvent.mouseLocation) }) {
                return hoveredScreen
            }
        default:
            break
        }

        guard let selectedDisplay = coordinator.config.selectedDisplay ?? availableDisplays.first else {
            return NSScreen.main ?? NSScreen.screens.first
        }

        return screenForDisplay(selectedDisplay) ?? NSScreen.main ?? NSScreen.screens.first
    }

    var windowSelectionHighlightFrame: CGRect? {
        guard recorderToolbarSettings.captureMode == .window,
              hasUserExplicitlySelectedCaptureModeForCard,
              let windowFrame = windowPreviewForCard?.frame,
              windowFrame.width > 8,
              windowFrame.height > 8
        else {
            return nil
        }

        return windowFrame.insetBy(dx: -3, dy: -3).integral
    }

    private func screenForDisplay(_ selectedDisplay: SCDisplay) -> NSScreen? {
        if let matchedByID = NSScreen.screens.first(where: {
            guard let screenNumber = $0.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber else {
                return false
            }
            return screenNumber.uint32Value == selectedDisplay.displayID
        }) {
            return matchedByID
        }

        let targetFrame = selectedDisplay.frame
        return NSScreen.screens.first(where: { screen in
            let frame = screen.frame
            return abs(frame.minX - targetFrame.minX) < 2 &&
                abs(frame.minY - targetFrame.minY) < 2 &&
                abs(frame.width - targetFrame.width) < 2 &&
                abs(frame.height - targetFrame.height) < 2
        })
    }

    private func beginDisplayClickSelection() {
        syncDisplaySelectionWithAvailableContent()
        isAwaitingWindowClickSelection = false
        hoveredWindowForSelection = nil

        guard !screenRecordingPermissionDenied else {
            isAwaitingDisplayClickSelection = false
            stopDisplaySelectionTracking(clearHover: true)
            return
        }

        let initialDisplay = displayForMouseLocation(NSEvent.mouseLocation)
            ?? coordinator.config.selectedDisplay
            ?? availableDisplays.first

        hoveredDisplayForSelection = initialDisplay
        isAwaitingDisplayClickSelection = initialDisplay != nil

        guard isAwaitingDisplayClickSelection else {
            stopDisplaySelectionTracking(clearHover: true)
            return
        }

        installDisplaySelectionTracking()
    }

    private func beginWindowClickSelection() {
        syncWindowSelectionWithAvailableContent()
        isAwaitingDisplayClickSelection = false
        hoveredDisplayForSelection = nil

        guard !screenRecordingPermissionDenied else {
            isAwaitingWindowClickSelection = false
            stopDisplaySelectionTracking(clearHover: true)
            return
        }

        let initialWindow = windowForMouseLocation(NSEvent.mouseLocation)
            ?? coordinator.config.selectedWindow
            ?? availableWindows.first

        hoveredWindowForSelection = initialWindow
        isAwaitingWindowClickSelection = initialWindow != nil

        guard isAwaitingWindowClickSelection else {
            stopDisplaySelectionTracking(clearHover: true)
            return
        }

        installDisplaySelectionTracking()
    }

    private func syncDisplaySelectionWithAvailableContent() {
        if let selectedDisplay = coordinator.config.selectedDisplay,
           let refreshedSelection = availableDisplays.first(where: { $0.displayID == selectedDisplay.displayID }) {
            coordinator.config.selectedDisplay = refreshedSelection
        } else {
            coordinator.config.selectedDisplay = nil
        }

        guard isAwaitingDisplayClickSelection else { return }

        if screenRecordingPermissionDenied {
            isAwaitingDisplayClickSelection = false
            stopDisplaySelectionTracking(clearHover: true)
            return
        }

        if let hoveredDisplay = hoveredDisplayForSelection,
           let refreshedHover = availableDisplays.first(where: { $0.displayID == hoveredDisplay.displayID }) {
            hoveredDisplayForSelection = refreshedHover
        } else {
            hoveredDisplayForSelection = displayForMouseLocation(NSEvent.mouseLocation) ?? availableDisplays.first
        }
    }

    private func syncWindowSelectionWithAvailableContent() {
        if let selectedWindow = coordinator.config.selectedWindow,
           let refreshedSelection = availableWindows.first(where: { $0.windowID == selectedWindow.windowID }) {
            coordinator.config.selectedWindow = refreshedSelection
        } else {
            coordinator.config.selectedWindow = nil
        }

        guard isAwaitingWindowClickSelection else { return }

        if screenRecordingPermissionDenied {
            isAwaitingWindowClickSelection = false
            stopDisplaySelectionTracking(clearHover: true)
            return
        }

        if let hoveredWindow = hoveredWindowForSelection,
           let refreshedHover = availableWindows.first(where: { $0.windowID == hoveredWindow.windowID }) {
            hoveredWindowForSelection = refreshedHover
        } else {
            hoveredWindowForSelection = windowForMouseLocation(NSEvent.mouseLocation) ?? availableWindows.first
        }
    }

    private func installDisplaySelectionTracking() {
        stopDisplaySelectionTracking(clearHover: false)

        displaySelectionTrackingTimer = Timer.scheduledTimer(withTimeInterval: 0.12, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.updatePendingCaptureSelectionFromMouseLocation()
            }
        }
        if let timer = displaySelectionTrackingTimer {
            RunLoop.main.add(timer, forMode: .common)
        }

        displaySelectionLocalEventMonitor = NSEvent.addLocalMonitorForEvents(matching: [.leftMouseDown]) { [weak self] event in
            self?.confirmPendingCaptureSelectionFromMouseLocation()
            return event
        }

        displaySelectionGlobalEventMonitor = NSEvent.addGlobalMonitorForEvents(matching: [.leftMouseDown]) { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.confirmPendingCaptureSelectionFromMouseLocation()
            }
        }
    }

    private func stopDisplaySelectionTracking(clearHover: Bool = false) {
        displaySelectionTrackingTimer?.invalidate()
        displaySelectionTrackingTimer = nil

        if let monitor = displaySelectionLocalEventMonitor {
            NSEvent.removeMonitor(monitor)
            displaySelectionLocalEventMonitor = nil
        }

        if let monitor = displaySelectionGlobalEventMonitor {
            NSEvent.removeMonitor(monitor)
            displaySelectionGlobalEventMonitor = nil
        }

        if clearHover {
            hoveredDisplayForSelection = nil
            hoveredWindowForSelection = nil
        }
    }

    private func updatePendingCaptureSelectionFromMouseLocation() {
        updateHoveredDisplayFromMouseLocation()
        updateHoveredWindowFromMouseLocation()
    }

    private func confirmPendingCaptureSelectionFromMouseLocation() {
        switch recorderToolbarSettings.captureMode {
        case .display:
            confirmDisplaySelectionFromMouseLocation()
        case .window:
            confirmWindowSelectionFromMouseLocation()
        default:
            return
        }
    }

    private func updateHoveredDisplayFromMouseLocation() {
        guard recorderToolbarSettings.captureMode == .display, isAwaitingDisplayClickSelection else { return }
        guard let hoveredDisplay = displayForMouseLocation(NSEvent.mouseLocation) else { return }

        if hoveredDisplayForSelection?.displayID != hoveredDisplay.displayID {
            hoveredDisplayForSelection = hoveredDisplay
            overlayManager.updateDisplayCardVisibility(appState: self)
        }
    }

    private func updateHoveredWindowFromMouseLocation() {
        guard recorderToolbarSettings.captureMode == .window, isAwaitingWindowClickSelection else { return }
        guard let hoveredWindow = windowForMouseLocation(NSEvent.mouseLocation) else { return }

        if hoveredWindowForSelection?.windowID != hoveredWindow.windowID {
            hoveredWindowForSelection = hoveredWindow
            overlayManager.updateDisplayCardVisibility(appState: self)
        }
    }

    private func confirmDisplaySelectionFromMouseLocation() {
        guard recorderToolbarSettings.captureMode == .display, isAwaitingDisplayClickSelection else { return }

        guard let selectedDisplay = displayForMouseLocation(NSEvent.mouseLocation) ?? hoveredDisplayForSelection else {
            return
        }

        setSelectedDisplay(selectedDisplay)
    }

    private func confirmWindowSelectionFromMouseLocation() {
        guard recorderToolbarSettings.captureMode == .window, isAwaitingWindowClickSelection else { return }

        guard let selectedWindow = windowForMouseLocation(NSEvent.mouseLocation) ?? hoveredWindowForSelection else {
            return
        }

        setSelectedWindow(selectedWindow)
    }

    private func displayForMouseLocation(_ location: CGPoint) -> SCDisplay? {
        guard let hoveredScreen = NSScreen.screens.first(where: { $0.frame.contains(location) }) else {
            return nil
        }

        if let screenNumber = hoveredScreen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber,
           let matchedByID = availableDisplays.first(where: { $0.displayID == screenNumber.uint32Value }) {
            return matchedByID
        }

        let hoveredFrame = hoveredScreen.frame
        return availableDisplays.first(where: { display in
            let frame = display.frame
            return abs(frame.minX - hoveredFrame.minX) < 2 &&
                abs(frame.minY - hoveredFrame.minY) < 2 &&
                abs(frame.width - hoveredFrame.width) < 2 &&
                abs(frame.height - hoveredFrame.height) < 2
        })
    }

    private func windowForMouseLocation(_ location: CGPoint) -> SCWindow? {
        var belowWindowNumber = 0

        for _ in 0 ..< 12 {
            let windowNumber = NSWindow.windowNumber(at: location, belowWindowWithWindowNumber: belowWindowNumber)
            guard windowNumber > 0 else { break }

            if let matchedWindow = availableWindows.first(where: { Int($0.windowID) == windowNumber }) {
                return matchedWindow
            }

            belowWindowNumber = windowNumber
        }

        return availableWindows.first(where: { window in
            window.frame.insetBy(dx: -2, dy: -2).contains(location)
        })
    }

    var availableCameraDevices: [AVCaptureDevice] {
        webcamEngine.availableCameras
    }

    var availableMicrophoneDevices: [AudioInputDeviceService.InputDevice] {
        AudioInputDeviceService.inputDevices()
    }

    var selectedCameraName: String {
        guard let id = recorderToolbarSettings.cameraDeviceID,
              let device = webcamEngine.availableCameras.first(where: { $0.uniqueID == id })
        else {
            return "No Camera"
        }
        return device.localizedName
    }

    var selectedMicrophoneName: String {
        guard recorderToolbarSettings.recordMicrophone else {
            return "No microphone"
        }
        guard let id = recorderToolbarSettings.microphoneDeviceID,
              let device = availableMicrophoneDevices.first(where: { $0.id == id })
        else {
            return "Default microphone"
        }
        return device.name
    }

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

            activePostRecordingSettings = snapshotPostRecordingSettings()

            if recorderToolbarSettings.hideDockIconWhileRecording {
                NSApp.setActivationPolicy(.accessory)
            }

            let countdown = recorderToolbarSettings.recordingCountdown.rawValue
            if countdown > 0 {
                try? await Task.sleep(nanoseconds: UInt64(countdown) * 1_000_000_000)
            }

            // Give the coordinator access to the webcam engine for separate recording
            coordinator.webcamEngine = webcamEngine

            applyRecorderToolbarSettingsToRuntime()

            try await coordinator.startRecording(
                recordWebcam: webcamEngine.isRunning && recorderToolbarSettings.recordCamera
            )
            print("[Frame] Recording started successfully")
            logger.info("Recording started successfully")
            areaSelectionManager.dismiss()
            overlayManager.updateDisplayCardVisibility(appState: self)
            menuBarManager?.refresh()
        } catch let error as RecordingError {
            activePostRecordingSettings = nil
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
            activePostRecordingSettings = nil
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
        let settings = activePostRecordingSettings ?? snapshotPostRecordingSettings()
        activePostRecordingSettings = nil

        if var project {
            project.effects.autoZoomEnabled = settings.autoCreateZooms

            if settings.createProject {
                currentProject = project
                mode = .editor
            }

            await executePostRecordingExportIfNeeded(project: project, settings: settings)

            logger.info("Recording stopped, project ready: \(project.name)")
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

        if recorderToolbarSettings.hideDockIconWhileRecording {
            NSApp.setActivationPolicy(.regular)
        }
        overlayManager.updateDisplayCardVisibility(appState: self)
        menuBarManager?.refresh()
    }

    func togglePause() {
        guard isRecording else { return }
        coordinator.togglePause()
        menuBarManager?.refresh()
    }

    func resetRecording() {
        guard isRecording, !recordingActionInFlight else { return }
        recordingActionInFlight = true

        Task {
            defer { recordingActionInFlight = false }

            let shouldRecordWebcam = webcamEngine.isRunning
            _ = await coordinator.stopRecording(discard: true)

            do {
                coordinator.webcamEngine = webcamEngine
                try await coordinator.startRecording(recordWebcam: shouldRecordWebcam)
                logger.info("Recording reset and restarted")
                overlayManager.updateDisplayCardVisibility(appState: self)
            } catch {
                logger.error("Failed to restart recording after reset: \(error.localizedDescription)")
                recordingError = RecordingAppError(
                    title: "Reset Failed",
                    message: error.localizedDescription
                )
                showErrorAlert = true
            }
            menuBarManager?.refresh()
        }
    }

    func deleteRecording() {
        guard isRecording, !recordingActionInFlight else { return }
        recordingActionInFlight = true

        Task {
            defer { recordingActionInFlight = false }
            _ = await coordinator.stopRecording(discard: true)
            currentProject = nil
            logger.info("Recording deleted and returned to setup")
            overlayManager.updateDisplayCardVisibility(appState: self)
            menuBarManager?.refresh()
        }
    }

    func openQuickExportSettings() {
        if mode == .recorder {
            showMainWindow()
            hideWindowAfterQuickExportSheet = true
        }
        showQuickExportSettings = true
    }

    func handleQuickExportSettingsDismissed() {
        if hideWindowAfterQuickExportSheet && mode == .recorder {
            hideMainWindow()
            hideWindowAfterQuickExportSheet = false
        }
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
            isAwaitingDisplayClickSelection = false
            isAwaitingWindowClickSelection = false
            stopDisplaySelectionTracking(clearHover: true)

            // Hide floating panels, show main window with editor
            overlayManager.hideOverlays()
            stopMicLevelMonitoring()
            showMainWindow()
        }
        overlayManager.updateDisplayCardVisibility(appState: self)
        menuBarManager?.refresh()
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

        // Start mic level monitoring for toolbar meter
        startMicLevelMonitoring()
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

    // MARK: - Recorder Toolbar Settings

    private func applyRecorderToolbarSettingsToRuntime() {
        coordinator.config.captureSystemAudio = recorderToolbarSettings.recordSystemAudio
        coordinator.config.captureMicrophone = recorderToolbarSettings.recordMicrophone

        switch recorderToolbarSettings.captureMode {
        case .display:
            coordinator.config.captureType = .display
            coordinator.config.areaRect = nil
        case .window:
            coordinator.config.captureType = .window
            coordinator.config.areaRect = nil
        case .area:
            coordinator.config.captureType = .area
            coordinator.config.areaRect = CGRect(
                x: recorderToolbarSettings.areaX,
                y: recorderToolbarSettings.areaY,
                width: recorderToolbarSettings.areaWidth,
                height: recorderToolbarSettings.areaHeight
            )
        case .device:
            coordinator.config.captureType = .device
            coordinator.config.areaRect = nil
        }

        applyWindowOutputSizeToRuntime()

        webcamEngine.maxResolution = recorderToolbarSettings.cameraResolution
        if let selectedCameraID = recorderToolbarSettings.cameraDeviceID {
            webcamEngine.selectedCameraID = selectedCameraID
        }

        overlayManager.updateWebcamVisibility(appState: self)
        overlayManager.updateDisplayCardVisibility(appState: self)
    }

    private func applyWindowOutputSizeToRuntime() {
        if let selectedSize = selectedWindowOutputSize {
            coordinator.config.windowOutputSize = CGSize(width: selectedSize.width, height: selectedSize.height)
        } else {
            coordinator.config.windowOutputSize = nil
        }
    }

    private func loadWindowCaptureSizeSettings() {
        let defaults = UserDefaults.standard

        if let width = defaults.object(forKey: Self.windowOutputSizeWidthDefaultsKey) as? Int,
           let height = defaults.object(forKey: Self.windowOutputSizeHeightDefaultsKey) as? Int {
            selectedWindowOutputSize = normalizedWindowCaptureSize(width: width, height: height)
        }

        if let data = defaults.data(forKey: Self.savedWindowOutputSizesDefaultsKey),
           let decodedPresets = try? JSONDecoder().decode([WindowCaptureSizePreset].self, from: data) {
            var uniquePresets: [WindowCaptureSizePreset] = []

            for preset in decodedPresets {
                guard let normalized = normalizedWindowCaptureSize(width: preset.width, height: preset.height) else {
                    continue
                }

                if !uniquePresets.contains(normalized) {
                    uniquePresets.append(normalized)
                }
            }

            savedWindowOutputSizes = uniquePresets.sorted { lhs, rhs in
                if lhs.width == rhs.width {
                    return lhs.height < rhs.height
                }
                return lhs.width < rhs.width
            }
        }
    }

    private func persistWindowCaptureSizeSettings() {
        let defaults = UserDefaults.standard

        if let selectedSize = selectedWindowOutputSize {
            defaults.set(selectedSize.width, forKey: Self.windowOutputSizeWidthDefaultsKey)
            defaults.set(selectedSize.height, forKey: Self.windowOutputSizeHeightDefaultsKey)
        } else {
            defaults.removeObject(forKey: Self.windowOutputSizeWidthDefaultsKey)
            defaults.removeObject(forKey: Self.windowOutputSizeHeightDefaultsKey)
        }

        if let encoded = try? JSONEncoder().encode(savedWindowOutputSizes) {
            defaults.set(encoded, forKey: Self.savedWindowOutputSizesDefaultsKey)
        } else {
            defaults.removeObject(forKey: Self.savedWindowOutputSizesDefaultsKey)
        }
    }

    // MARK: - Post-Recording Helpers

    private func loadPostRecordingSettings() {
        let defaults = UserDefaults.standard

        if defaults.object(forKey: "postRecording.createProject") != nil {
            postCreateProject = defaults.bool(forKey: "postRecording.createProject")
        }
        if defaults.object(forKey: "postRecording.autoCreateZooms") != nil {
            postAutoCreateZooms = defaults.bool(forKey: "postRecording.autoCreateZooms")
        }
        if let raw = defaults.string(forKey: "postRecording.exportMode"),
           let mode = PostRecordingExportMode(rawValue: raw) {
            postExportMode = mode
        }

        if let raw = defaults.string(forKey: "postRecording.quickExport.format"),
           let value = ExportConfig.ExportFormat(rawValue: raw) {
            quickExportFormat = value
        }
        if let raw = defaults.string(forKey: "postRecording.quickExport.quality"),
           let value = ExportConfig.ExportQuality(rawValue: raw) {
            quickExportQuality = value
        }
        if let raw = defaults.string(forKey: "postRecording.quickExport.resolution"),
           let value = ExportConfig.ExportResolution(rawValue: raw) {
            quickExportResolution = value
        }
        if let frameRate = defaults.object(forKey: "postRecording.quickExport.frameRate") as? Int,
           let value = ExportConfig.ExportFrameRate(rawValue: frameRate) {
            quickExportFrameRate = value
        }
    }

    private func snapshotPostRecordingSettings() -> PostRecordingSettingsSnapshot {
        PostRecordingSettingsSnapshot(
            createProject: postCreateProject,
            autoCreateZooms: postAutoCreateZooms,
            exportMode: postExportMode,
            quickExportFormat: quickExportFormat,
            quickExportQuality: quickExportQuality,
            quickExportResolution: quickExportResolution,
            quickExportFrameRate: quickExportFrameRate
        )
    }

    private func quickExportConfig(from settings: PostRecordingSettingsSnapshot) -> ExportConfig {
        var config = ExportConfig()
        config.format = settings.quickExportFormat
        config.quality = settings.quickExportQuality
        config.resolution = settings.quickExportResolution
        config.frameRate = settings.quickExportFrameRate
        return config
    }

    private func executePostRecordingExportIfNeeded(
        project: Project,
        settings: PostRecordingSettingsSnapshot
    ) async {
        guard settings.exportMode != .none else { return }

        let shouldRestoreRecorderWindow = mode == .recorder && !settings.createProject
        if shouldRestoreRecorderWindow {
            showMainWindow()
        }
        defer {
            if shouldRestoreRecorderWindow {
                hideMainWindow()
            }
        }

        let config = quickExportConfig(from: settings)
        switch settings.exportMode {
        case .none:
            return
        case .saveToFile:
            guard let outputURL = promptSaveURL(project: project, config: config) else { return }
            _ = await exportProject(project: project, config: config, outputURL: outputURL)
        case .clipboard:
            guard let outputURL = makeTemporaryExportURL(project: project, config: config),
                  let exportedURL = await exportProject(project: project, config: config, outputURL: outputURL)
            else { return }
            NSPasteboard.general.clearContents()
            NSPasteboard.general.writeObjects([exportedURL as NSURL, exportedURL.path as NSString])
            logger.info("Exported file copied to clipboard: \(exportedURL.lastPathComponent)")
        case .shareableLink:
            guard let outputURL = makeTemporaryExportURL(project: project, config: config),
                  let exportedURL = await exportProject(project: project, config: config, outputURL: outputURL)
            else { return }
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(exportedURL.absoluteString, forType: .string)
            NSWorkspace.shared.activateFileViewerSelecting([exportedURL])
            logger.info("Exported file URL copied as shareable link: \(exportedURL.absoluteString)")
        }
    }

    private func exportProject(
        project: Project,
        config: ExportConfig,
        outputURL: URL
    ) async -> URL? {
        guard !exportEngine.isExporting else { return nil }

        exportEngine.export(project: project, config: config, outputURL: outputURL)

        while exportEngine.isExporting {
            try? await Task.sleep(nanoseconds: 100_000_000)
        }

        return exportEngine.currentPhase == .complete ? outputURL : nil
    }

    private func promptSaveURL(project: Project, config: ExportConfig) -> URL? {
        let panel = NSSavePanel()
        panel.title = "Quick Export"
        panel.nameFieldStringValue = config.defaultFilename(projectName: project.name)
        panel.canCreateDirectories = true
        panel.allowedContentTypes = [
            config.format == .mp4 ? .mpeg4Movie :
            config.format == .mov ? .quickTimeMovie :
                .gif
        ]
        return panel.runModal() == .OK ? panel.url : nil
    }

    private func makeTemporaryExportURL(project: Project, config: ExportConfig) -> URL? {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("frame-quick-export", isDirectory: true)
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        } catch {
            logger.error("Failed to create temporary export directory: \(error.localizedDescription)")
            return nil
        }

        let filename = config.defaultFilename(projectName: project.name)
        return directory.appendingPathComponent(filename)
    }
}
