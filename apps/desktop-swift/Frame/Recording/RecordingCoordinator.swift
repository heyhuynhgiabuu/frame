import Foundation
import ScreenCaptureKit
import OSLog
import QuartzCore

private let logger = Logger(subsystem: "com.frame.app", category: "RecordingCoordinator")

/// Orchestrates all recording components: screen capture, cursor tracking, and audio.
/// This is the single entry point for the UI to control recording.
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

    // MARK: - Available Sources

    var availableDisplays: [SCDisplay] { screenRecorder.availableDisplays }
    var availableWindows: [SCWindow] { screenRecorder.availableWindows }

    // MARK: - Refresh

    func refreshSources() async {
        await screenRecorder.refreshAvailableContent()

        // Auto-select primary display if none selected
        if config.selectedDisplay == nil {
            config.selectedDisplay = screenRecorder.availableDisplays.first
        }
    }

    // MARK: - Start Recording

    func startRecording(webcamFrameBox: WebcamFrameBox? = nil, webcamConfig: WebcamOverlayConfig? = nil) async throws {
        guard !isRecording else { return }

        logger.info("Starting recording session...")

        // Configure webcam compositing on the screen recorder
        if let frameBox = webcamFrameBox {
            let now = CACurrentMediaTime()
            guard let snapshot = frameBox.snapshot,
                  now - snapshot.capturedAt <= 0.5 else {
                throw RecordingError.webcamFrameUnavailable
            }

            screenRecorder.webcamFrameProvider = { [weak frameBox] in
                frameBox?.snapshot
            }
            screenRecorder.webcamConfig = webcamConfig
        } else {
            screenRecorder.webcamFrameProvider = nil
            screenRecorder.webcamConfig = nil
        }

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
    func stopRecording() async -> Project? {
        guard isRecording else { return nil }

        logger.info("Stopping recording session...")

        // Stop screen recording
        let videoURL = await screenRecorder.stopRecording()

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
        project.duration = duration

        if let display = config.selectedDisplay ?? availableDisplays.first {
            let streamConfig = config.makeStreamConfiguration(for: display)
            project.resolutionWidth = Double(streamConfig.width)
            project.resolutionHeight = Double(streamConfig.height)
        }

        project.frameRate = Double(config.frameRate)

        logger.info("Recording session complete: \(duration)s, saved to \(videoURL.lastPathComponent)")

        return project
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
}
