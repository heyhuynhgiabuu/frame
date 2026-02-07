import Foundation
import ScreenCaptureKit
import AVFoundation
import CoreImage
import OSLog
import QuartzCore

private let logger = Logger(subsystem: "com.frame.app", category: "ScreenRecorder")

/// Core screen recording engine using ScreenCaptureKit + AVAssetWriter.
@MainActor
final class ScreenRecorder: NSObject, ObservableObject {

    // MARK: - Published State

    @Published private(set) var isRecording = false
    @Published private(set) var isPaused = false
    @Published private(set) var recordingDuration: TimeInterval = 0

    /// Available displays and windows for capture
    @Published var availableDisplays: [SCDisplay] = []
    @Published var availableWindows: [SCWindow] = []

    /// Whether screen recording permission appears to be denied
    @Published var permissionDenied = false

    // MARK: - Private State

    private var stream: SCStream?
    private var streamOutput: RecordingStreamOutput?
    private var assetWriter: AVAssetWriter?
    private var videoInput: AVAssetWriterInput?
    private var audioInput: AVAssetWriterInput?
    private var durationTimer: Timer?
    private var startTime: Date?

    /// Output file URL for the current recording
    private(set) var outputURL: URL?

    /// External webcam frame provider for real-time compositing.
    /// Set this before starting recording to composite webcam into the video.
    var webcamFrameProvider: (() -> WebcamFrameSnapshot?)?

    /// Webcam overlay configuration.
    var webcamConfig: WebcamOverlayConfig?

    // MARK: - Refresh Available Content

    func refreshAvailableContent() async {
        do {
            let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
            availableDisplays = content.displays
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

        switch config.captureType {
        case .display:
            guard let selectedDisplay = config.selectedDisplay ?? availableDisplays.first else {
                throw RecordingError.noDisplayAvailable
            }
            display = selectedDisplay
            filter = SCContentFilter(display: selectedDisplay, excludingApplications: [], exceptingWindows: [])

        case .window:
            guard let selectedWindow = config.selectedWindow else {
                throw RecordingError.noWindowSelected
            }
            guard let windowDisplay = availableDisplays.first else {
                throw RecordingError.noDisplayAvailable
            }
            display = windowDisplay
            filter = SCContentFilter(desktopIndependentWindow: selectedWindow)
        }

        // Configure stream
        let streamConfig = config.makeStreamConfiguration(for: display)

        // Setup output file
        let outputURL = makeOutputURL()
        self.outputURL = outputURL

        // Capture values needed by the background task before entering Task.detached.
        // These Sendable values can safely cross actor boundaries.
        let captureSystemAudio = config.captureSystemAudio
        let captureMicrophone = config.captureMicrophone
        let frameRate = config.frameRate
        let webcamFrameProvider = self.webcamFrameProvider
        let webcamConfig = self.webcamConfig

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

            // Audio input (if capturing audio)
            var aInput: AVAssetWriterInput?
            if captureSystemAudio || captureMicrophone {
                let audioSettings: [String: Any] = [
                    AVFormatIDKey: kAudioFormatMPEG4AAC,
                    AVSampleRateKey: 48000,
                    AVNumberOfChannelsKey: 2,
                    AVEncoderBitRateKey: 192000,
                ]
                let audioInput = AVAssetWriterInput(mediaType: .audio, outputSettings: audioSettings)
                audioInput.expectsMediaDataInRealTime = true
                guard writer.canAdd(audioInput) else {
                    throw RecordingError.writerNotReady
                }
                writer.add(audioInput)
                aInput = audioInput
            }

            // IMPORTANT: Create RecordingStreamOutput BEFORE calling startWriting().
            // AVAssetWriterInputPixelBufferAdaptor requires the writer to be in .unknown status.
            // Creating it after startWriting() throws an ObjC exception that bypasses Swift error handling.
            let output = RecordingStreamOutput(
                assetWriter: writer,
                videoInput: vInput,
                audioInput: aInput,
                webcamFrameProvider: webcamFrameProvider,
                webcamConfig: webcamConfig
            )

            // Now safe to start writing — all inputs and adaptor are configured
            writer.startWriting()

            // Create and start SCStream
            let captureStream = SCStream(filter: filter, configuration: streamConfig, delegate: nil)
            try captureStream.addStreamOutput(output, type: .screen, sampleHandlerQueue: .global(qos: .userInitiated))
            if captureSystemAudio {
                try captureStream.addStreamOutput(output, type: .audio, sampleHandlerQueue: .global(qos: .userInitiated))
            }

            try await captureStream.startCapture()

            return (writer, vInput, aInput, output, captureStream)
        }.value

        // Back on @MainActor — assign state
        self.assetWriter = result.0
        self.videoInput = result.1
        self.audioInput = result.2
        self.streamOutput = result.3
        self.stream = result.4

        // Update state
        isRecording = true
        isPaused = false
        startTime = Date()

        // Start duration timer
        durationTimer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                guard let self, let startTime = self.startTime else { return }
                self.recordingDuration = Date().timeIntervalSince(startTime)
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
        recordingDuration = 0
        isRecording = false
        isPaused = false

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
        audioInput = nil
    }

    // MARK: - Stop Recording

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

        // Finalize asset writer
        videoInput?.markAsFinished()
        audioInput?.markAsFinished()

        if let writer = assetWriter, writer.status == .writing {
            await writer.finishWriting()
            logger.info("Recording saved: \(self.outputURL?.lastPathComponent ?? "unknown")")
        }

        // Reset state
        isRecording = false
        isPaused = false
        streamOutput = nil

        let url = outputURL
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

        return frameDir.appendingPathComponent("Frame_\(timestamp).mov")
    }
}

// MARK: - Recording Errors

enum RecordingError: LocalizedError {
    case noDisplayAvailable
    case screenRecordingPermissionDenied
    case noWindowSelected
    case writerNotReady
    case alreadyRecording
    case webcamFrameUnavailable

    var errorDescription: String? {
        switch self {
        case .noDisplayAvailable: return "No display available for recording"
        case .screenRecordingPermissionDenied: return "Screen recording permission is required"
        case .noWindowSelected: return "No window selected for recording"
        case .writerNotReady: return "Video writer is not ready"
        case .alreadyRecording: return "Already recording"
        case .webcamFrameUnavailable: return "Webcam is enabled but no live webcam frames are available"
        }
    }

    /// Detailed recovery suggestion for the user
    var recoverySuggestion: String? {
        switch self {
        case .screenRecordingPermissionDenied:
            return "Please enable Screen Recording in System Settings → Privacy & Security → Screen Recording, then restart Frame."
        case .noDisplayAvailable:
            return "No displays were found. This can happen if screen recording permission was just granted — macOS requires an app restart for the change to take effect."
        case .webcamFrameUnavailable:
            return "Frame needs a live webcam feed before recording. Check camera permission and that webcam preview is moving, then try again."
        default:
            return nil
        }
    }
}

// MARK: - Webcam Overlay Config

/// Configuration for compositing webcam onto screen recording in real-time.
struct WebcamOverlayConfig {
    let position: WebcamPosition
    let size: Double           // Relative to video width (0.1 - 0.4)
    let shape: WebcamShape
    let padding: Double        // Points from edge

    init(position: WebcamPosition = .bottomLeft, size: Double = 0.2, shape: WebcamShape = .circle, padding: Double = 24) {
        self.position = position
        self.size = size
        self.shape = shape
        self.padding = padding
    }
}

// MARK: - Stream Output Handler

private class RecordingStreamOutput: NSObject, SCStreamOutput {
    private let assetWriter: AVAssetWriter
    private let videoInput: AVAssetWriterInput
    private let audioInput: AVAssetWriterInput?
    private let pixelBufferAdaptor: AVAssetWriterInputPixelBufferAdaptor?

    private var isFirstVideoSample = true
    private var firstSampleTime: CMTime = .zero

    /// Thread-safe webcam frame provider
    private let webcamFrameProvider: (() -> WebcamFrameSnapshot?)?
    private let webcamConfig: WebcamOverlayConfig?
    private let ciContext = CIContext(options: [.useSoftwareRenderer: false])
    private var lastGoodWebcamFrame: CIImage?
    private var lastGoodWebcamTimestamp: CFTimeInterval = 0
    private let webcamFrameGraceWindow: CFTimeInterval = 0.15

    init(assetWriter: AVAssetWriter, videoInput: AVAssetWriterInput, audioInput: AVAssetWriterInput?,
         webcamFrameProvider: (() -> WebcamFrameSnapshot?)?, webcamConfig: WebcamOverlayConfig?) {
        self.assetWriter = assetWriter
        self.videoInput = videoInput
        self.audioInput = audioInput
        self.webcamFrameProvider = webcamFrameProvider
        self.webcamConfig = webcamConfig

        // Create pixel buffer adaptor for composited output
        if webcamFrameProvider != nil {
            let attrs: [String: Any] = [
                kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
            ]
            self.pixelBufferAdaptor = AVAssetWriterInputPixelBufferAdaptor(
                assetWriterInput: videoInput,
                sourcePixelBufferAttributes: attrs
            )
        } else {
            self.pixelBufferAdaptor = nil
        }

        super.init()
    }

    func stream(_ stream: SCStream, didOutputSampleBuffer sampleBuffer: CMSampleBuffer, of type: SCStreamOutputType) {
        guard sampleBuffer.isValid else { return }
        guard assetWriter.status == .writing else { return }

        let timestamp = CMSampleBufferGetPresentationTimeStamp(sampleBuffer)

        switch type {
        case .screen:
            handleVideoSample(sampleBuffer, timestamp: timestamp)
        case .audio:
            handleAudioSample(sampleBuffer, timestamp: timestamp)
        case .microphone:
            handleAudioSample(sampleBuffer, timestamp: timestamp)
        @unknown default:
            break
        }
    }

    private func handleVideoSample(_ sampleBuffer: CMSampleBuffer, timestamp: CMTime) {
        guard isCompleteScreenFrame(sampleBuffer) else { return }

        // Start session on first video frame
        if isFirstVideoSample {
            firstSampleTime = timestamp
            assetWriter.startSession(atSourceTime: .zero)
            isFirstVideoSample = false
        }

        guard videoInput.isReadyForMoreMediaData else { return }

        // Calculate adjusted timestamp
        let adjustedTime = CMTimeSubtract(timestamp, firstSampleTime)
        guard adjustedTime.seconds >= 0 else { return }

        var didAppend = false

        // If webcam is enabled, composite webcam onto screen frame.
        if let webcamProvider = webcamFrameProvider,
           let webcamConfig = webcamConfig,
           let pixelBuffer = CMSampleBufferGetImageBuffer(sampleBuffer) {

            let screenImage = CIImage(cvPixelBuffer: pixelBuffer)
            let now = CACurrentMediaTime()
            var webcamImage: CIImage?

            if let snapshot = webcamProvider() {
                webcamImage = snapshot.image
                lastGoodWebcamFrame = snapshot.image
                lastGoodWebcamTimestamp = snapshot.capturedAt
            } else if let cachedFrame = lastGoodWebcamFrame,
                      now - lastGoodWebcamTimestamp <= webcamFrameGraceWindow {
                webcamImage = cachedFrame
            }

            if let webcamImage,
               let outputBuffer = renderToPixelBuffer(
                compositeWebcam(webcamImage, onto: screenImage, config: webcamConfig),
                width: CVPixelBufferGetWidth(pixelBuffer),
                height: CVPixelBufferGetHeight(pixelBuffer)
               ),
               let adaptor = pixelBufferAdaptor {
                didAppend = adaptor.append(outputBuffer, withPresentationTime: adjustedTime)
                if !didAppend {
                    logger.error("Failed to append composited webcam frame; falling back to screen frame")
                }
            } else {
                logger.warning("Webcam frame unavailable or stale while recording; falling back to screen frame")
            }
        }

        // Always append the original screen frame if compositing did not append.
        if !didAppend,
           let retimedBuffer = retimeSampleBuffer(sampleBuffer, offsetBy: firstSampleTime) {
            let appended = videoInput.append(retimedBuffer)
            if !appended {
                logger.error("Failed to append retimed screen frame")
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

    // MARK: - Webcam Compositing

    private func compositeWebcam(_ webcamImage: CIImage, onto screenImage: CIImage, config: WebcamOverlayConfig) -> CIImage {
        let screenWidth = screenImage.extent.width
        let screenHeight = screenImage.extent.height

        // Calculate webcam overlay size
        let webcamDiameter = screenWidth * CGFloat(config.size)

        // Scale webcam to fit the target size
        let webcamExtent = webcamImage.extent
        let webcamScale = webcamDiameter / min(webcamExtent.width, webcamExtent.height)
        let scaledWebcam = webcamImage.transformed(by: CGAffineTransform(scaleX: webcamScale, y: webcamScale))

        // Crop to square (center crop)
        let scaledExtent = scaledWebcam.extent
        let cropSize = min(scaledExtent.width, scaledExtent.height)
        let cropX = scaledExtent.origin.x + (scaledExtent.width - cropSize) / 2
        let cropY = scaledExtent.origin.y + (scaledExtent.height - cropSize) / 2
        var croppedWebcam = scaledWebcam.cropped(to: CGRect(x: cropX, y: cropY, width: cropSize, height: cropSize))

        // Apply circular mask if needed
        if config.shape == .circle {
            croppedWebcam = applyCircleMask(to: croppedWebcam)
        } else if config.shape == .roundedRectangle {
            croppedWebcam = applyRoundedRectMask(to: croppedWebcam, cornerRadius: cropSize * 0.15)
        }

        // Calculate position
        let padding = CGFloat(config.padding) * (screenWidth / 1920.0)  // Scale padding relative to 1080p
        let webcamOrigin: CGPoint
        switch config.position {
        case .bottomLeft:
            webcamOrigin = CGPoint(x: padding, y: padding)
        case .bottomRight:
            webcamOrigin = CGPoint(x: screenWidth - cropSize - padding, y: padding)
        case .topLeft:
            webcamOrigin = CGPoint(x: padding, y: screenHeight - cropSize - padding)
        case .topRight:
            webcamOrigin = CGPoint(x: screenWidth - cropSize - padding, y: screenHeight - cropSize - padding)
        }

        // Translate webcam to position
        let translatedWebcam = croppedWebcam.transformed(by: CGAffineTransform(
            translationX: webcamOrigin.x - croppedWebcam.extent.origin.x,
            y: webcamOrigin.y - croppedWebcam.extent.origin.y
        ))

        // Composite: webcam on top of screen
        return translatedWebcam.composited(over: screenImage)
    }

    private func applyCircleMask(to image: CIImage) -> CIImage {
        let extent = image.extent
        let radius = min(extent.width, extent.height) / 2

        // Create radial gradient as circle mask
        guard let radialGradient = CIFilter(name: "CIRadialGradient", parameters: [
            "inputCenter": CIVector(x: extent.midX, y: extent.midY),
            "inputRadius0": radius - 1,  // Solid area
            "inputRadius1": radius,       // Feather edge (1px)
            "inputColor0": CIColor.white,
            "inputColor1": CIColor.clear,
        ])?.outputImage else { return image }

        let mask = radialGradient.cropped(to: extent)

        guard let blendFilter = CIFilter(name: "CIBlendWithMask", parameters: [
            kCIInputImageKey: image,
            kCIInputBackgroundImageKey: CIImage.empty(),
            kCIInputMaskImageKey: mask,
        ])?.outputImage else { return image }

        return blendFilter.cropped(to: extent)
    }

    private func applyRoundedRectMask(to image: CIImage, cornerRadius: CGFloat) -> CIImage {
        let extent = image.extent

        // Create rounded rect path as mask
        guard let generator = CIFilter(name: "CIRoundedRectangleGenerator", parameters: [
            "inputExtent": CIVector(cgRect: extent),
            "inputRadius": cornerRadius,
            "inputColor": CIColor.white,
        ])?.outputImage else { return image }

        guard let blendFilter = CIFilter(name: "CIBlendWithMask", parameters: [
            kCIInputImageKey: image,
            kCIInputBackgroundImageKey: CIImage.empty(),
            kCIInputMaskImageKey: generator,
        ])?.outputImage else { return image }

        return blendFilter.cropped(to: extent)
    }

    private func renderToPixelBuffer(_ image: CIImage, width: Int, height: Int) -> CVPixelBuffer? {
        var pixelBuffer: CVPixelBuffer?
        let attrs: [String: Any] = [
            kCVPixelBufferCGImageCompatibilityKey as String: true,
            kCVPixelBufferCGBitmapContextCompatibilityKey as String: true,
        ]

        let status = CVPixelBufferCreate(kCFAllocatorDefault, width, height, kCVPixelFormatType_32BGRA, attrs as CFDictionary, &pixelBuffer)
        guard status == kCVReturnSuccess, let buffer = pixelBuffer else { return nil }

        ciContext.render(image, to: buffer)
        return buffer
    }

    // MARK: - Audio

    private func handleAudioSample(_ sampleBuffer: CMSampleBuffer, timestamp: CMTime) {
        guard !isFirstVideoSample else { return }
        guard let audioInput, audioInput.isReadyForMoreMediaData else { return }

        if let retimedBuffer = retimeSampleBuffer(sampleBuffer, offsetBy: firstSampleTime) {
            audioInput.append(retimedBuffer)
        }
    }

    /// Retime a sample buffer so timestamps start from zero.
    private func retimeSampleBuffer(_ buffer: CMSampleBuffer, offsetBy offset: CMTime) -> CMSampleBuffer? {
        let originalTime = CMSampleBufferGetPresentationTimeStamp(buffer)
        let adjustedTime = CMTimeSubtract(originalTime, offset)

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
