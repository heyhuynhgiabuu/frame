import AVFoundation
import CoreImage
import AppKit
import QuartzCore
import OSLog

private let logger = Logger(subsystem: "com.frame.app", category: "WebcamCaptureEngine")

struct WebcamFrameSnapshot {
    let image: CIImage
    let capturedAt: CFTimeInterval
}

/// Thread-safe container for sharing webcam frames between threads.
final class WebcamFrameBox: @unchecked Sendable {
    private let lock = NSLock()
    private var _snapshot: WebcamFrameSnapshot?

    var snapshot: WebcamFrameSnapshot? {
        get {
            lock.lock()
            defer { lock.unlock() }
            return _snapshot
        }
        set {
            lock.lock()
            _snapshot = newValue
            lock.unlock()
        }
    }

    var frame: CIImage? {
        get {
            snapshot?.image
        }
        set {
            if let newValue {
                snapshot = WebcamFrameSnapshot(image: newValue, capturedAt: CACurrentMediaTime())
            } else {
                snapshot = nil
            }
        }
    }
}

/// Captures live webcam video using AVCaptureSession.
/// Provides the latest frame as a CIImage for overlay compositing.
@MainActor
final class WebcamCaptureEngine: NSObject, ObservableObject {

    @Published var isRunning = false
    @Published var latestFrame: CIImage?
    @Published var availableCameras: [AVCaptureDevice] = []
    @Published var selectedCameraID: String?
    @Published var maxResolution: RecorderToolbarSettings.CameraResolution = .p1080
    @Published var permissionGranted = false

    private var captureSession: AVCaptureSession?
    private var videoOutput: AVCaptureVideoDataOutput?
    private let outputQueue = DispatchQueue(label: "com.frame.webcam-output", qos: .userInteractive)

    /// Thread-safe frame box for live preview by CIImageView display link.
    let frameBox = WebcamFrameBox()

    // MARK: - Recording State (file recording to .mov)

    /// Serialises all AVAssetWriter operations.
    private let writerQueue = DispatchQueue(label: "com.frame.webcam-writer", qos: .userInitiated)
    private var assetWriter: AVAssetWriter?
    private var writerInput: AVAssetWriterInput?
    private var pixelBufferAdaptor: AVAssetWriterInputPixelBufferAdaptor?
    private var isWriting = false
    private var firstFrameTime: CMTime = .zero
    private var recordingOutputURL: URL?

    override init() {
        super.init()
        refreshCameras()
    }

    // MARK: - Camera Discovery

    func refreshCameras() {
        let discovery = AVCaptureDevice.DiscoverySession(
            deviceTypes: [.builtInWideAngleCamera, .external],
            mediaType: .video,
            position: .unspecified
        )
        availableCameras = discovery.devices
        if selectedCameraID == nil {
            selectedCameraID = discovery.devices.first?.uniqueID
        }
    }

    // MARK: - Permissions

    func requestPermission() async {
        let status = AVCaptureDevice.authorizationStatus(for: .video)
        switch status {
        case .authorized:
            permissionGranted = true
        case .notDetermined:
            permissionGranted = await AVCaptureDevice.requestAccess(for: .video)
        default:
            permissionGranted = false
        }
    }

    // MARK: - Start / Stop

    func start() async {
        guard !isRunning else { return }
        await requestPermission()
        guard permissionGranted else { return }

        guard let cameraID = selectedCameraID,
              let device = AVCaptureDevice(uniqueID: cameraID) else {
            return
        }

        let session = AVCaptureSession()
        let requestedPreset = maxResolution.preset
        if session.canSetSessionPreset(requestedPreset) {
            session.sessionPreset = requestedPreset
        } else {
            session.sessionPreset = .high
        }

        do {
            let input = try AVCaptureDeviceInput(device: device)
            guard session.canAddInput(input) else { return }
            session.addInput(input)

            let output = AVCaptureVideoDataOutput()
            output.videoSettings = [
                kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA
            ]
            output.alwaysDiscardsLateVideoFrames = true
            output.setSampleBufferDelegate(self, queue: outputQueue)

            guard session.canAddOutput(output) else { return }
            session.addOutput(output)

            // Note: We handle mirroring in captureOutput via CIImage transforms
            // rather than connection.isVideoMirrored, which doesn't work reliably
            // with all camera types (e.g. external webcams).

            self.captureSession = session
            self.videoOutput = output

            // Dispatch startRunning() to the dedicated output queue to avoid
            // blocking the main thread while the camera hardware initialises.
            await withCheckedContinuation { (continuation: CheckedContinuation<Void, Never>) in
                self.outputQueue.async {
                    session.startRunning()
                    continuation.resume()
                }
            }
            isRunning = true
        } catch {
            print("WebcamCaptureEngine: Failed to start — \(error.localizedDescription)")
        }
    }

    func stop() async {
        guard let session = captureSession else {
            isRunning = false
            latestFrame = nil
            return
        }

        // Dispatch stopRunning() to the dedicated output queue to avoid
        // blocking the main thread while the camera hardware shuts down.
        await withCheckedContinuation { (continuation: CheckedContinuation<Void, Never>) in
            self.outputQueue.async {
                session.stopRunning()
                continuation.resume()
            }
        }
        captureSession = nil
        videoOutput = nil
        isRunning = false
        latestFrame = nil
    }

    // MARK: - File Recording (separate webcam .mov)

    /// Start recording webcam frames to a separate .mov file.
    /// The webcam capture session must already be running.
    func startRecording(to outputURL: URL) async throws {
        guard isRunning else {
            throw WebcamRecordingError.notRunning
        }

        let writer = try AVAssetWriter(outputURL: outputURL, fileType: .mov)

        // Use the webcam's native resolution from the session
        let width: Int
        let height: Int
        if let connection = videoOutput?.connection(with: .video),
           let port = connection.inputPorts.first,
           let desc = port.formatDescription {
            let dims = CMVideoFormatDescriptionGetDimensions(desc)
            width = Int(dims.width)
            height = Int(dims.height)
        } else {
            // Fallback — medium preset is typically 480p
            width = 640
            height = 480
        }

        let videoSettings: [String: Any] = [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: width,
            AVVideoHeightKey: height,
            AVVideoCompressionPropertiesKey: [
                AVVideoAverageBitRateKey: width * height * 4,
                AVVideoExpectedSourceFrameRateKey: 30,
                AVVideoProfileLevelKey: AVVideoProfileLevelH264HighAutoLevel,
            ] as [String: Any],
        ]

        let input = AVAssetWriterInput(mediaType: .video, outputSettings: videoSettings)
        input.expectsMediaDataInRealTime = true

        let adaptor = AVAssetWriterInputPixelBufferAdaptor(
            assetWriterInput: input,
            sourcePixelBufferAttributes: [
                kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
                kCVPixelBufferWidthKey as String: width,
                kCVPixelBufferHeightKey as String: height,
            ]
        )

        guard writer.canAdd(input) else {
            throw WebcamRecordingError.writerSetupFailed
        }
        writer.add(input)

        // Assign state before starting writing (on writer queue for safety)
        writerQueue.sync {
            self.assetWriter = writer
            self.writerInput = input
            self.pixelBufferAdaptor = adaptor
            self.recordingOutputURL = outputURL
            self.firstFrameTime = .zero

            writer.startWriting()
            // Gate: captureOutput checks isWriting first, so frames only enter
            // the writer pipeline after startWriting() has completed.
            self.isWriting = true
        }

        logger.info("Webcam recording ready: \(width)x\(height) → \(outputURL.lastPathComponent)")
    }

    /// Stop recording and finalize the .mov file.
    /// Returns the output URL of the recorded file, or nil if nothing was written.
    func stopRecording() async -> URL? {
        let url = recordingOutputURL

        // Finalize on the writer queue to avoid races with captureOutput
        await withCheckedContinuation { (continuation: CheckedContinuation<Void, Never>) in
            writerQueue.async { [weak self] in
                guard let self else {
                    continuation.resume()
                    return
                }

                self.isWriting = false

                self.writerInput?.markAsFinished()

                guard let writer = self.assetWriter, writer.status == .writing else {
                    self.assetWriter = nil
                    self.writerInput = nil
                    self.pixelBufferAdaptor = nil
                    self.recordingOutputURL = nil
                    continuation.resume()
                    return
                }

                writer.finishWriting {
                    self.assetWriter = nil
                    self.writerInput = nil
                    self.pixelBufferAdaptor = nil
                    self.recordingOutputURL = nil
                    logger.info("Webcam recording finalized: \(url?.lastPathComponent ?? "nil")")
                    continuation.resume()
                }
            }
        }

        return url
    }
}

// MARK: - Webcam Recording Errors

enum WebcamRecordingError: LocalizedError {
    case notRunning
    case writerSetupFailed

    var errorDescription: String? {
        switch self {
        case .notRunning: return "Webcam is not running"
        case .writerSetupFailed: return "Failed to setup webcam video writer"
        }
    }
}

// MARK: - AVCaptureVideoDataOutputSampleBufferDelegate

extension WebcamCaptureEngine: AVCaptureVideoDataOutputSampleBufferDelegate {

    nonisolated func captureOutput(
        _ output: AVCaptureOutput,
        didOutput sampleBuffer: CMSampleBuffer,
        from connection: AVCaptureConnection
    ) {
        guard let pixelBuffer = CMSampleBufferGetImageBuffer(sampleBuffer) else { return }

        let ciImage = CIImage(cvPixelBuffer: pixelBuffer)
            .oriented(.upMirrored)

        // Store for thread-safe access by CIImageView display link (live preview)
        frameBox.snapshot = WebcamFrameSnapshot(image: ciImage, capturedAt: CACurrentMediaTime())

        // Also publish to @Published for Combine subscribers (editor mode NSImage conversion).
        // This main-thread hop is lightweight — just assigns a reference.
        Task { @MainActor in
            self.latestFrame = ciImage
        }

        // Write frame to file if recording
        let timestamp = CMSampleBufferGetPresentationTimeStamp(sampleBuffer)
        writerQueue.async { [weak self] in
            guard let self,
                  self.isWriting,
                  let writer = self.assetWriter,
                  writer.status == .writing,
                  let input = self.writerInput,
                  let adaptor = self.pixelBufferAdaptor else { return }

            // Start session on first frame
            if self.firstFrameTime == .zero {
                self.firstFrameTime = timestamp
                writer.startSession(atSourceTime: .zero)
            }

            guard input.isReadyForMoreMediaData else { return }

            let adjustedTime = CMTimeSubtract(timestamp, self.firstFrameTime)
            guard adjustedTime.seconds >= 0 else { return }

            adaptor.append(pixelBuffer, withPresentationTime: adjustedTime)
        }
    }
}
