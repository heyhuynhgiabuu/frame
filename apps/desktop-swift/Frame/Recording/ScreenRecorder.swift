import Foundation
import ScreenCaptureKit
import AVFoundation
import CoreImage
import OSLog
import QuartzCore

private let logger = Logger(subsystem: "com.frame.app", category: "ScreenRecorder")

/// Core screen recording engine using ScreenCaptureKit + AVAssetWriter.
/// Records screen-only video (no webcam compositing).
/// Webcam is recorded separately by WebcamCaptureEngine.
@MainActor
final class ScreenRecorder: NSObject, ObservableObject {

    // MARK: - Published State

    @Published private(set) var isRecording = false
    @Published private(set) var isPaused = false
    @Published private(set) var recordingDuration: TimeInterval = 0

    /// Available displays and windows for capture
    @Published var availableDisplays: [SCDisplay] = []
    @Published var availableWindows: [SCWindow] = []

    /// All running applications (needed for includingApplications filter)
    private var availableApplications: [SCRunningApplication] = []

    /// Whether screen recording permission appears to be denied
    @Published var permissionDenied = false

    // MARK: - Private State

    private var stream: SCStream?
    private var streamOutput: RecordingStreamOutput?
    private var assetWriter: AVAssetWriter?
    private var videoInput: AVAssetWriterInput?
    private var systemAudioInput: AVAssetWriterInput?
    private var micAudioInput: AVAssetWriterInput?
    private var durationTimer: Timer?
    private var startTime: Date?
    private var pausedAt: Date?
    private var accumulatedPausedDuration: TimeInterval = 0
    private let pauseStateStore = PauseStateStore()

    /// Output file URL for the current recording
    private(set) var outputURL: URL?

    // MARK: - Refresh Available Content

    func refreshAvailableContent() async {
        do {
            let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
            availableDisplays = content.displays
            availableApplications = content.applications
            availableWindows = content.windows.filter { window in
                // Filter out system windows and our own app
                guard let app = window.owningApplication else { return false }
                return app.bundleIdentifier != Bundle.main.bundleIdentifier
                    && window.isOnScreen
                    && window.frame.width > 100
                    && window.frame.height > 100
            }
            permissionDenied = content.displays.isEmpty
            logger.info("Found \(self.availableDisplays.count) displays, \(self.availableWindows.count) windows")
        } catch {
            logger.error("Failed to get shareable content: \(error.localizedDescription)")
            // If SCShareableContent throws, permission is likely denied
            permissionDenied = true
            availableDisplays = []
            availableWindows = []
        }
    }

    // MARK: - Start Recording

    func startRecording(config: RecordingConfig) async throws {
        guard !isRecording else {
            logger.warning("Already recording, ignoring start request")
            return
        }

        do {

        // Refresh available content
        await refreshAvailableContent()

        // Check permission first — give a clear error if denied
        if permissionDenied {
            throw RecordingError.screenRecordingPermissionDenied
        }

        // Determine what to capture
        let filter: SCContentFilter
        let display: SCDisplay
        var selectedWindowForCapture: SCWindow?

        switch config.captureType {
        case .display:
            guard let selectedDisplay = config.selectedDisplay ?? availableDisplays.first else {
                throw RecordingError.noDisplayAvailable
            }
            display = selectedDisplay
            // Use includingApplications instead of excludingApplications.
            // An empty excludingApplications array is known to cause SCStream failures
            // on some macOS versions, especially with system audio capture enabled.
            // We include all running apps except our own (to avoid capturing our own UI/audio).
            let appsToInclude = availableApplications.filter {
                $0.bundleIdentifier != Bundle.main.bundleIdentifier
            }
            filter = SCContentFilter(display: selectedDisplay, including: appsToInclude, exceptingWindows: [])

        case .window:
            guard let selectedWindow = config.selectedWindow else {
                throw RecordingError.noWindowSelected
            }
            guard let windowDisplay = availableDisplays.first else {
                throw RecordingError.noDisplayAvailable
            }
            display = windowDisplay
            selectedWindowForCapture = selectedWindow
            filter = SCContentFilter(desktopIndependentWindow: selectedWindow)

        case .area:
            guard let selectedDisplay = config.selectedDisplay ?? availableDisplays.first else {
                throw RecordingError.noDisplayAvailable
            }
            display = selectedDisplay
            let appsToInclude = availableApplications.filter {
                $0.bundleIdentifier != Bundle.main.bundleIdentifier
            }
            filter = SCContentFilter(display: selectedDisplay, including: appsToInclude, exceptingWindows: [])

        case .device:
            guard let selectedDisplay = config.selectedDisplay ?? availableDisplays.first else {
                throw RecordingError.noDisplayAvailable
            }
            display = selectedDisplay
            logger.warning("Device capture selected but unsupported; using full display")
            let appsToInclude = availableApplications.filter {
                $0.bundleIdentifier != Bundle.main.bundleIdentifier
            }
            filter = SCContentFilter(display: selectedDisplay, including: appsToInclude, exceptingWindows: [])
        }

        // Configure stream
        let streamConfig = config.makeStreamConfiguration(for: display)

        if config.captureType == .window,
           let selectedWindow = selectedWindowForCapture {
            let defaultWindowSize = RecordingConfig.clampedVideoSize(
                width: Int(selectedWindow.frame.width.rounded()),
                height: Int(selectedWindow.frame.height.rounded())
            )

            let targetSize: (width: Int, height: Int)
            if let outputSize = config.windowOutputSize {
                targetSize = RecordingConfig.clampedVideoSize(
                    width: Int(outputSize.width.rounded()),
                    height: Int(outputSize.height.rounded())
                )
            } else {
                targetSize = defaultWindowSize
            }

            streamConfig.width = targetSize.width
            streamConfig.height = targetSize.height
        }

        // Setup output file
        let outputURL = makeOutputURL()
        self.outputURL = outputURL

        // Capture values needed by the background task before entering Task.detached.
        // These Sendable values can safely cross actor boundaries.
        let captureSystemAudio = config.captureSystemAudio
        let captureMicrophone = config.captureMicrophone
        let frameRate = config.frameRate
        let pauseStateStore = self.pauseStateStore

        // Run the entire AVAssetWriter + SCStream setup off the main actor.
        // This prevents blocking the UI while the capture pipeline initialises
        // (AVAssetWriter creation, SCStream.startCapture hardware handshake).
        let result = try await Task.detached(priority: .userInitiated) {
            // Setup AVAssetWriter
            let writer = try AVAssetWriter(outputURL: outputURL, fileType: .mov)

            // Video input
            let videoSettings: [String: Any] = [
                AVVideoCodecKey: AVVideoCodecType.h264,
                AVVideoWidthKey: streamConfig.width,
                AVVideoHeightKey: streamConfig.height,
                AVVideoCompressionPropertiesKey: [
                    AVVideoAverageBitRateKey: streamConfig.width * streamConfig.height * 8,
                    AVVideoExpectedSourceFrameRateKey: frameRate,
                    AVVideoProfileLevelKey: AVVideoProfileLevelH264HighAutoLevel,
                ] as [String: Any],
            ]
            let vInput = AVAssetWriterInput(mediaType: .video, outputSettings: videoSettings)
            vInput.expectsMediaDataInRealTime = true
            guard writer.canAdd(vInput) else {
                throw RecordingError.writerNotReady
            }
            writer.add(vInput)

            // Audio inputs — SEPARATE inputs for system audio vs microphone.
            // System audio arrives as 2-channel stereo; microphone arrives as 1-channel mono.
            // Writing both formats to a single AVAssetWriterInput corrupts the encoder
            // (format mismatch → writer enters .failed state at frame #3).
            var sysAudioInput: AVAssetWriterInput?
            var micInput: AVAssetWriterInput?

            if captureSystemAudio {
                let systemAudioSettings: [String: Any] = [
                    AVFormatIDKey: kAudioFormatMPEG4AAC,
                    AVSampleRateKey: 48000,
                    AVNumberOfChannelsKey: 2,
                    AVEncoderBitRateKey: 192000,
                ]
                let input = AVAssetWriterInput(mediaType: .audio, outputSettings: systemAudioSettings)
                input.expectsMediaDataInRealTime = true
                guard writer.canAdd(input) else {
                    throw RecordingError.writerNotReady
                }
                writer.add(input)
                sysAudioInput = input
            }

            if captureMicrophone {
                let micAudioSettings: [String: Any] = [
                    AVFormatIDKey: kAudioFormatMPEG4AAC,
                    AVSampleRateKey: 48000,
                    AVNumberOfChannelsKey: 1,
                    AVEncoderBitRateKey: 96000,
                ]
                let input = AVAssetWriterInput(mediaType: .audio, outputSettings: micAudioSettings)
                input.expectsMediaDataInRealTime = true
                guard writer.canAdd(input) else {
                    throw RecordingError.writerNotReady
                }
                writer.add(input)
                micInput = input
            }

            // Create RecordingStreamOutput BEFORE calling startWriting().
            // AVAssetWriterInputPixelBufferAdaptor requires the writer to be in .unknown status.
            let output = RecordingStreamOutput(
                assetWriter: writer,
                videoInput: vInput,
                systemAudioInput: sysAudioInput,
                micAudioInput: micInput,
                pauseStateStore: pauseStateStore
            )

            // Now safe to start writing — all inputs are configured
            guard writer.startWriting() else {
                let errorDesc = writer.error?.localizedDescription ?? "unknown"
                logger.error("AVAssetWriter.startWriting() failed: \(errorDesc)")
                throw RecordingError.writerNotReady
            }
            logger.info("AVAssetWriter started writing successfully (status=\(writer.status.rawValue))")

            // Create and start SCStream
            // CRITICAL: Each output type MUST use a separate serial queue.
            // Using a shared concurrent queue (.global) causes data races on
            // isFirstVideoSample/firstSampleTime and can corrupt AVAssetWriter
            // state (double startSession calls → writer enters .failed state).
            let videoQueue = DispatchQueue(label: "dev.frame.scstream.video", qos: .userInteractive)
            let audioQueue = DispatchQueue(label: "dev.frame.scstream.audio", qos: .userInteractive)
            let micQueue = DispatchQueue(label: "dev.frame.scstream.mic", qos: .userInteractive)

            let captureStream = SCStream(filter: filter, configuration: streamConfig, delegate: output)
            try captureStream.addStreamOutput(output, type: .screen, sampleHandlerQueue: videoQueue)
            if captureSystemAudio {
                try captureStream.addStreamOutput(output, type: .audio, sampleHandlerQueue: audioQueue)
            }
            if captureMicrophone {
                if #available(macOS 15.0, *) {
                    try captureStream.addStreamOutput(output, type: .microphone, sampleHandlerQueue: micQueue)
                }
            }

            try await captureStream.startCapture()

            return (writer, vInput, sysAudioInput, micInput, output, captureStream)
        }.value

        // Back on @MainActor — assign state
        self.assetWriter = result.0
        self.videoInput = result.1
        self.systemAudioInput = result.2
        self.micAudioInput = result.3
        self.streamOutput = result.4
        self.stream = result.5

        // Update state
        isRecording = true
        isPaused = false
        startTime = Date()
        pausedAt = nil
        accumulatedPausedDuration = 0
        pauseStateStore.update(isPaused: false, pausedDuration: 0)

        // Start duration timer
        durationTimer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                guard let self, let startTime = self.startTime else { return }
                let now = Date()
                let inFlightPause = self.pausedAt.map { now.timeIntervalSince($0) } ?? 0
                self.recordingDuration = max(0, now.timeIntervalSince(startTime) - self.accumulatedPausedDuration - inFlightPause)
            }
        }
        durationTimer?.fire()

        logger.info("Recording started: \(streamConfig.width)x\(streamConfig.height) @ \(config.frameRate)fps → \(outputURL.lastPathComponent)")
        } catch {
            await cleanupAfterFailedStart()
            throw error
        }
    }

    private func cleanupAfterFailedStart() async {
        durationTimer?.invalidate()
        durationTimer = nil
        startTime = nil
        pausedAt = nil
        accumulatedPausedDuration = 0
        recordingDuration = 0
        isRecording = false
        isPaused = false
        pauseStateStore.update(isPaused: false, pausedDuration: 0)

        if let stream {
            try? await Task.detached(priority: .userInitiated) {
                try await stream.stopCapture()
            }.value
            self.stream = nil
        }

        streamOutput = nil

        if let writer = assetWriter {
            if writer.status == .writing {
                writer.cancelWriting()
            }
            assetWriter = nil
        }

        videoInput = nil
        systemAudioInput = nil
        micAudioInput = nil

        // Remove empty/invalid output file
        if let outputURL {
            try? FileManager.default.removeItem(at: outputURL)
            self.outputURL = nil
        }
    }

    // MARK: - Stop Recording

    func togglePause() {
        guard isRecording else { return }

        if isPaused {
            let pauseInterval = pausedAt.map { Date().timeIntervalSince($0) } ?? 0
            accumulatedPausedDuration += pauseInterval
            pausedAt = nil
            isPaused = false
            pauseStateStore.update(isPaused: false, pausedDuration: accumulatedPausedDuration)
            logger.info("Screen recording resumed")
        } else {
            pausedAt = Date()
            isPaused = true
            pauseStateStore.update(isPaused: true, pausedDuration: accumulatedPausedDuration)
            logger.info("Screen recording paused")
        }
    }

    func stopRecording() async -> URL? {
        guard isRecording else { return nil }

        // Stop the timer
        durationTimer?.invalidate()
        durationTimer = nil

        // Stop stream off the main actor to avoid blocking the UI
        if let stream {
            do {
                try await Task.detached(priority: .userInitiated) {
                    try await stream.stopCapture()
                }.value
            } catch {
                logger.error("Failed to stop capture: \(error.localizedDescription)")
            }
        }
        self.stream = nil

        // Finalize asset writer on the writerQueue to avoid races with pending appends.
        // This mirrors WebcamCaptureEngine's finalization pattern.
        if let output = streamOutput {
            let (vFrames, aFrames, sessionStarted) = output.stats
            logger.info("Recording stats — video frames: \(vFrames), audio frames: \(aFrames), session started: \(sessionStarted)")
            if let streamErr = output.streamError {
                logger.error("[SCStream] Stream error during recording: \(streamErr.localizedDescription)")
            }
        }

        if let writer = assetWriter {
            // Capture @MainActor-isolated properties before entering the non-isolated closure.
            // These are safe to use on writerQueue because the SCStream has already been stopped,
            // so no more callbacks will access the writer concurrently.
            let videoIn = videoInput
            let sysAudioIn = systemAudioInput
            let micAudioIn = micAudioInput
            nonisolated(unsafe) let unsafeWriter = writer

            await withCheckedContinuation { (continuation: CheckedContinuation<Void, Never>) in
                // streamOutput holds writerQueue — dispatch finalization there
                // so any in-flight appends complete before we finalize.
                let writerQueue = streamOutput?.writerQueueForFinalization
                    ?? DispatchQueue(label: "dev.frame.recording.writer.finalize")

                writerQueue.async {
                    videoIn?.markAsFinished()
                    sysAudioIn?.markAsFinished()
                    micAudioIn?.markAsFinished()

                    guard unsafeWriter.status == .writing else {
                        logger.warning("AVAssetWriter not in writing state (status=\(unsafeWriter.status.rawValue)), cannot finalize")
                        continuation.resume()
                        return
                    }

                    unsafeWriter.finishWriting {
                        if unsafeWriter.status == .completed {
                            logger.info("Recording saved successfully")
                        } else {
                            logger.error("AVAssetWriter finished with status \(unsafeWriter.status.rawValue), error: \(unsafeWriter.error?.localizedDescription ?? "none")")
                        }
                        continuation.resume()
                    }
                }
            }
        }

        // Reset state
        isRecording = false
        isPaused = false
        pausedAt = nil
        accumulatedPausedDuration = 0
        pauseStateStore.update(isPaused: false, pausedDuration: 0)
        streamOutput = nil

        // Only return URL if writer completed successfully
        guard let writer = assetWriter, writer.status == .completed else {
            let status = assetWriter?.status.rawValue ?? -1
            let writerErr = assetWriter?.error?.localizedDescription ?? "none"
            logger.error("Recording file invalid — writer status: \(status), error: \(writerErr)")
            // Clean up invalid/empty output file
            if let outputURL {
                try? FileManager.default.removeItem(at: outputURL)
            }
            return nil
        }

        let url = outputURL
        outputURL = nil
        assetWriter = nil
        videoInput = nil
        systemAudioInput = nil
        micAudioInput = nil
        return url
    }

    // MARK: - Helpers

    private func makeOutputURL() -> URL {
        let documentsDir = FileManager.default.urls(for: .moviesDirectory, in: .userDomainMask).first!
        let frameDir = documentsDir.appendingPathComponent("Frame Recordings", isDirectory: true)

        // Create directory if needed
        try? FileManager.default.createDirectory(at: frameDir, withIntermediateDirectories: true)

        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd_HH-mm-ss"
        let timestamp = formatter.string(from: Date())

        // AVAssetWriter fails if file already exists — ensure unique filename
        var url = frameDir.appendingPathComponent("Frame_\(timestamp).mov")
        var counter = 1
        while FileManager.default.fileExists(atPath: url.path) {
            url = frameDir.appendingPathComponent("Frame_\(timestamp)_\(counter).mov")
            counter += 1
        }

        return url
    }
}

// MARK: - Pause State

private final class PauseStateStore: @unchecked Sendable {
    private let lock = NSLock()
    private var paused = false
    private var pausedDuration: TimeInterval = 0

    func update(isPaused: Bool, pausedDuration: TimeInterval) {
        lock.lock()
        paused = isPaused
        self.pausedDuration = pausedDuration
        lock.unlock()
    }

    func snapshot() -> (isPaused: Bool, pausedDuration: TimeInterval) {
        lock.lock()
        let value = (paused, pausedDuration)
        lock.unlock()
        return value
    }
}

// MARK: - Recording Errors

enum RecordingError: LocalizedError {
    case noDisplayAvailable
    case screenRecordingPermissionDenied
    case noWindowSelected
    case writerNotReady
    case alreadyRecording

    var errorDescription: String? {
        switch self {
        case .noDisplayAvailable: return "No display available for recording"
        case .screenRecordingPermissionDenied: return "Screen recording permission is required"
        case .noWindowSelected: return "No window selected for recording"
        case .writerNotReady: return "Video writer is not ready"
        case .alreadyRecording: return "Already recording"
        }
    }

    /// Detailed recovery suggestion for the user
    var recoverySuggestion: String? {
        switch self {
        case .screenRecordingPermissionDenied:
            return "Please enable Screen Recording in System Settings → Privacy & Security → Screen Recording, then restart Frame."
        case .noDisplayAvailable:
            return "No displays were found. This can happen if screen recording permission was just granted — macOS requires an app restart for the change to take effect."
        default:
            return nil
        }
    }
}

// MARK: - Stream Output Handler

private class RecordingStreamOutput: NSObject, SCStreamOutput, SCStreamDelegate {
    private let assetWriter: AVAssetWriter
    private let videoInput: AVAssetWriterInput
    private let systemAudioInput: AVAssetWriterInput?
    private let micAudioInput: AVAssetWriterInput?
    private let pauseStateStore: PauseStateStore

    /// Serial queue that serializes ALL AVAssetWriter operations (startSession, append, stats).
    /// Video and audio callbacks arrive on different SCStream queues — without serialization,
    /// concurrent `append()` calls corrupt the writer and push it into `.failed` state.
    /// This mirrors WebcamCaptureEngine.writerQueue.
    private let writerQueue = DispatchQueue(label: "dev.frame.recording.writer", qos: .userInitiated)
    private var isSessionStarted = false
    private var firstSampleTime: CMTime = .zero
    private var videoFrameCount = 0
    private var audioFrameCount = 0
    /// Error reported by SCStreamDelegate when the stream stops unexpectedly
    private(set) var streamError: Error?

    init(
        assetWriter: AVAssetWriter,
        videoInput: AVAssetWriterInput,
        systemAudioInput: AVAssetWriterInput?,
        micAudioInput: AVAssetWriterInput?,
        pauseStateStore: PauseStateStore
    ) {
        self.assetWriter = assetWriter
        self.videoInput = videoInput
        self.systemAudioInput = systemAudioInput
        self.micAudioInput = micAudioInput
        self.pauseStateStore = pauseStateStore
        super.init()
    }

    /// Thread-safe stats for diagnostics
    var stats: (videoFrames: Int, audioFrames: Int, sessionStarted: Bool) {
        writerQueue.sync { (videoFrameCount, audioFrameCount, isSessionStarted) }
    }

    /// Expose writerQueue for finalization — ensures pending appends drain before markAsFinished/finishWriting.
    var writerQueueForFinalization: DispatchQueue { writerQueue }

    // MARK: - SCStreamDelegate

    /// Called when the stream stops unexpectedly (e.g., audio capture failure, permission revoked).
    /// Without this delegate, stream errors are silently lost.
    func stream(_ stream: SCStream, didStopWithError error: Error) {
        logger.error("[SCStream] Stream stopped with error: \(error.localizedDescription)")
        streamError = error
    }

    // MARK: - SCStreamOutput

    func stream(_ stream: SCStream, didOutputSampleBuffer sampleBuffer: CMSampleBuffer, of type: SCStreamOutputType) {
        guard sampleBuffer.isValid else {
            logger.warning("Received invalid sample buffer (type: \(String(describing: type)))")
            return
        }

        // Pre-filter non-complete screen frames BEFORE entering writerQueue
        // to avoid serialization overhead for frames we'd discard anyway.
        if type == .screen, !isCompleteScreenFrame(sampleBuffer) {
            return
        }

        let timestamp = CMSampleBufferGetPresentationTimeStamp(sampleBuffer)

        // ALL writer operations serialized on writerQueue.
        // Video and audio callbacks arrive on separate SCStream queues —
        // without this, concurrent append() calls corrupt AVAssetWriter.
        writerQueue.async { [weak self] in
            guard let self else { return }
            guard self.assetWriter.status == .writing else {
                logger.warning("Writer not in writing state (\(self.assetWriter.status.rawValue)), dropping \(String(describing: type)) frame. Error: \(self.assetWriter.error?.localizedDescription ?? "none")")
                return
            }

            let pauseState = self.pauseStateStore.snapshot()
            if pauseState.isPaused {
                return
            }

            switch type {
            case .screen:
                self.handleVideoSample(sampleBuffer, timestamp: timestamp, pausedDuration: pauseState.pausedDuration)
            case .audio:
                self.handleAudioSample(
                    sampleBuffer,
                    timestamp: timestamp,
                    input: self.systemAudioInput,
                    label: "system",
                    pausedDuration: pauseState.pausedDuration
                )
            case .microphone:
                self.handleAudioSample(
                    sampleBuffer,
                    timestamp: timestamp,
                    input: self.micAudioInput,
                    label: "mic",
                    pausedDuration: pauseState.pausedDuration
                )
            @unknown default:
                break
            }
        }
    }

    /// Called on writerQueue — all state access is safe without additional synchronization.
    private func handleVideoSample(_ sampleBuffer: CMSampleBuffer, timestamp: CMTime, pausedDuration: TimeInterval) {
        // Start session on first complete video frame
        if !isSessionStarted {
            firstSampleTime = timestamp
            isSessionStarted = true
            assetWriter.startSession(atSourceTime: .zero)
            logger.info("First video frame received — session started at \(timestamp.seconds)s")
            // Log format description for debugging
            if let formatDesc = CMSampleBufferGetFormatDescription(sampleBuffer) {
                let dimensions = CMVideoFormatDescriptionGetDimensions(formatDesc)
                logger.info("Video format: \(dimensions.width)x\(dimensions.height), codec: \(CMFormatDescriptionGetMediaSubType(formatDesc))")
            }
        }

        guard videoInput.isReadyForMoreMediaData else { return }

        if let retimedBuffer = retimeSampleBuffer(sampleBuffer, offsetBy: firstSampleTime, pausedDuration: pausedDuration) {
            let appended = videoInput.append(retimedBuffer)
            if appended {
                videoFrameCount += 1
                // Log progress every 30 frames (roughly once per second at 30fps)
                if videoFrameCount % 30 == 0 {
                    logger.debug("Video: \(self.videoFrameCount) frames, audio: \(self.audioFrameCount) frames, writer: \(self.assetWriter.status.rawValue)")
                }
            } else {
                logger.error("Failed to append screen frame #\(self.videoFrameCount). Writer status: \(self.assetWriter.status.rawValue), error: \(self.assetWriter.error?.localizedDescription ?? "none")")
            }
        }
    }

    private func isCompleteScreenFrame(_ sampleBuffer: CMSampleBuffer) -> Bool {
        guard let attachments = CMSampleBufferGetSampleAttachmentsArray(sampleBuffer, createIfNecessary: false) as? [[SCStreamFrameInfo: Any]],
              let attachment = attachments.first,
              let statusRaw = attachment[.status] as? Int,
              let status = SCFrameStatus(rawValue: statusRaw) else {
            return true
        }

        return status == .complete
    }

    private func debugFrameStatus(_ sampleBuffer: CMSampleBuffer) -> String {
        guard let attachments = CMSampleBufferGetSampleAttachmentsArray(sampleBuffer, createIfNecessary: false) as? [[SCStreamFrameInfo: Any]],
              let attachment = attachments.first,
              let statusRaw = attachment[.status] as? Int else {
            return "no-attachment"
        }
        switch SCFrameStatus(rawValue: statusRaw) {
        case .idle: return "idle"
        case .blank: return "blank"
        case .suspended: return "suspended"
        case .started: return "started"
        case .stopped: return "stopped"
        case .complete: return "complete"
        default: return "unknown(\(statusRaw))"
        }
    }

    // MARK: - Audio

    /// Called on writerQueue — all state access is safe without additional synchronization.
    private func handleAudioSample(
        _ sampleBuffer: CMSampleBuffer,
        timestamp: CMTime,
        input: AVAssetWriterInput?,
        label: String,
        pausedDuration: TimeInterval
    ) {
        // Wait until session is started by the first video frame
        guard isSessionStarted else { return }
        guard let input, input.isReadyForMoreMediaData else { return }

        // Log first audio frame details for debugging
        if audioFrameCount == 0 {
            if let formatDesc = CMSampleBufferGetFormatDescription(sampleBuffer) {
                if let asbd = CMAudioFormatDescriptionGetStreamBasicDescription(formatDesc)?.pointee {
                    logger.info("First \(label) audio frame — sampleRate: \(asbd.mSampleRate), channels: \(asbd.mChannelsPerFrame), format: \(asbd.mFormatID), bitsPerChannel: \(asbd.mBitsPerChannel)")
                }
            }
        }

        if let retimedBuffer = retimeSampleBuffer(sampleBuffer, offsetBy: firstSampleTime, pausedDuration: pausedDuration) {
            let appended = input.append(retimedBuffer)
            if appended {
                audioFrameCount += 1
            } else {
                logger.error("Failed to append \(label) audio frame #\(self.audioFrameCount). Writer status: \(self.assetWriter.status.rawValue), error: \(self.assetWriter.error?.localizedDescription ?? "none")")
            }
        }
    }

    /// Retime a sample buffer so timestamps start from zero.
    private func retimeSampleBuffer(
        _ buffer: CMSampleBuffer,
        offsetBy offset: CMTime,
        pausedDuration: TimeInterval
    ) -> CMSampleBuffer? {
        let originalTime = CMSampleBufferGetPresentationTimeStamp(buffer)
        let adjustedByStart = CMTimeSubtract(originalTime, offset)
        let adjustedTime = CMTimeSubtract(adjustedByStart, CMTime(seconds: pausedDuration, preferredTimescale: 600))

        guard adjustedTime.seconds >= 0 else { return nil }

        var timingInfo = CMSampleTimingInfo(
            duration: CMSampleBufferGetDuration(buffer),
            presentationTimeStamp: adjustedTime,
            decodeTimeStamp: .invalid
        )

        var newBuffer: CMSampleBuffer?
        CMSampleBufferCreateCopyWithNewTiming(
            allocator: kCFAllocatorDefault,
            sampleBuffer: buffer,
            sampleTimingEntryCount: 1,
            sampleTimingArray: &timingInfo,
            sampleBufferOut: &newBuffer
        )

        return newBuffer
    }
}
