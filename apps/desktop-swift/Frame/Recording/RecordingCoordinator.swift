import Foundation
import ScreenCaptureKit
import OSLog
import QuartzCore

private let logger = Logger(subsystem: "com.frame.app", category: "RecordingCoordinator")

/// Orchestrates all recording components: screen capture, cursor tracking, webcam, and audio.
/// Screen and webcam are recorded to separate files for independent editing.
@MainActor
final class RecordingCoordinator: ObservableObject {

    // MARK: - Components

    let screenRecorder = ScreenRecorder()
    let cursorRecorder = CursorRecorder()
    let keystrokeRecorder = KeystrokeRecorder()

    // MARK: - Published State

    @Published private(set) var isRecording = false
    @Published private(set) var recordingDuration: TimeInterval = 0
    @Published var config = RecordingConfig()

    var isPaused: Bool {
        screenRecorder.isPaused
    }

    // MARK: - Available Sources

    var availableDisplays: [SCDisplay] { screenRecorder.availableDisplays }
    var availableWindows: [SCWindow] { screenRecorder.availableWindows }

    // MARK: - Webcam Recording

    /// The webcam engine to record from (set by AppState before starting)
    weak var webcamEngine: WebcamCaptureEngine?

    /// Whether the webcam was active during this recording session
    private var isWebcamRecording = false

    // MARK: - Refresh

    func refreshSources() async {
        await screenRecorder.refreshAvailableContent()

        if let selectedDisplay = config.selectedDisplay,
           let refreshedSelection = screenRecorder.availableDisplays.first(where: { $0.displayID == selectedDisplay.displayID }) {
            config.selectedDisplay = refreshedSelection
        }
    }

    // MARK: - Start Recording

    func startRecording(recordWebcam: Bool = false) async throws {
        guard !isRecording else { return }

        logger.info("Starting recording session...")

        // Start screen recording
        do {
            try await screenRecorder.startRecording(config: config)
        } catch {
            durationTimer?.invalidate()
            durationTimer = nil
            recordingDuration = 0
            isRecording = false
            throw error
        }

        // Start webcam recording to separate file (if webcam is running)
        if recordWebcam, let webcamEngine {
            let webcamURL = makeWebcamOutputURL()
            do {
                try await webcamEngine.startRecording(to: webcamURL)
                isWebcamRecording = true
                logger.info("Webcam recording started → \(webcamURL.lastPathComponent)")
            } catch {
                logger.error("Failed to start webcam recording: \(error.localizedDescription)")
                // Non-fatal: screen recording continues without webcam
            }
        }

        // Start cursor recording
        cursorRecorder.startRecording()

        // Start keystroke recording
        keystrokeRecorder.startRecording()

        // Update state
        isRecording = true

        // Sync duration from screen recorder
        startDurationSync()

        logger.info("Recording session started successfully")
    }

    // MARK: - Stop Recording

    /// Stops recording and returns a Project with all metadata.
    func stopRecording(discard: Bool = false) async -> Project? {
        guard isRecording else { return nil }

        logger.info("Stopping recording session...")

        // Stop screen recording
        let videoURL = await screenRecorder.stopRecording()

        // Stop webcam recording
        var webcamURL: URL?
        if isWebcamRecording, let webcamEngine {
            webcamURL = await webcamEngine.stopRecording()
            isWebcamRecording = false
            if let webcamURL {
                logger.info("Webcam recording saved: \(webcamURL.lastPathComponent)")
            }
        }

        // Stop cursor recording
        _ = cursorRecorder.stopRecording()

        // Stop keystroke recording
        _ = keystrokeRecorder.stopRecording()

        // Stop duration sync
        durationTimer?.invalidate()
        durationTimer = nil

        // Update state
        isRecording = false
        let duration = recordingDuration
        recordingDuration = 0

        guard let videoURL else {
            logger.error("No video URL after stopping recording")
            return nil
        }

        if discard {
            try? FileManager.default.removeItem(at: videoURL)
            if let webcamURL {
                try? FileManager.default.removeItem(at: webcamURL)
            }
            logger.info("Recording session discarded")
            return nil
        }

        // Save cursor events alongside video
        let cursorURL = videoURL.deletingPathExtension().appendingPathExtension("cursor.json")
        do {
            try cursorRecorder.saveEvents(to: cursorURL)
        } catch {
            logger.error("Failed to save cursor events: \(error.localizedDescription)")
        }

        // Save keystroke events alongside video
        let keystrokeURL = videoURL.deletingPathExtension().appendingPathExtension("keystrokes.json")
        do {
            try keystrokeRecorder.saveEvents(to: keystrokeURL)
        } catch {
            logger.error("Failed to save keystroke events: \(error.localizedDescription)")
        }

        // Create project
        var project = Project(name: videoURL.deletingPathExtension().lastPathComponent)
        project.recordingURL = videoURL
        project.webcamRecordingURL = webcamURL
        project.duration = duration

        if let display = config.selectedDisplay ?? availableDisplays.first {
            let streamConfig = config.makeStreamConfiguration(for: display)
            project.resolutionWidth = Double(streamConfig.width)
            project.resolutionHeight = Double(streamConfig.height)
        }

        project.frameRate = Double(config.frameRate)

        // Mark webcam as enabled in effects if we have a webcam recording
        if webcamURL != nil {
            project.effects.webcamEnabled = true
        }

        logger.info("Recording session complete: \(duration)s, saved to \(videoURL.lastPathComponent)")

        return project
    }

    func togglePause() {
        guard isRecording else { return }
        screenRecorder.togglePause()
        objectWillChange.send()
    }

    // MARK: - Duration Sync

    private var durationTimer: Timer?

    private func startDurationSync() {
        durationTimer?.invalidate()
        durationTimer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                guard let self else { return }
                self.recordingDuration = self.screenRecorder.recordingDuration
            }
        }
    }

    // MARK: - Helpers

    private func makeWebcamOutputURL() -> URL {
        let documentsDir = FileManager.default.urls(for: .moviesDirectory, in: .userDomainMask).first!
        let frameDir = documentsDir.appendingPathComponent("Frame Recordings", isDirectory: true)
        try? FileManager.default.createDirectory(at: frameDir, withIntermediateDirectories: true)

        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd_HH-mm-ss"
        let timestamp = formatter.string(from: Date())

        return frameDir.appendingPathComponent("Frame_\(timestamp)_webcam.mov")
    }
}
