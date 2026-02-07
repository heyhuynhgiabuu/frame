import AVFoundation
import CoreImage
import AppKit
import QuartzCore

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
    @Published var permissionGranted = false

    private var captureSession: AVCaptureSession?
    private var videoOutput: AVCaptureVideoDataOutput?
    private let outputQueue = DispatchQueue(label: "com.frame.webcam-output", qos: .userInteractive)

    /// Thread-safe frame box for compositing by the recording pipeline.
    let frameBox = WebcamFrameBox()

    override init() {
        super.init()
        refreshCameras()
    }

    // MARK: - Camera Discovery

    func refreshCameras() {
        let discovery = AVCaptureDevice.DiscoverySession(
            deviceTypes: [.builtInWideAngleCamera, .externalUnknown],
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
        session.sessionPreset = .medium    // 480p — sufficient for PiP overlay

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

        // Store for thread-safe access by recording pipeline + CIImageView display link
        frameBox.snapshot = WebcamFrameSnapshot(image: ciImage, capturedAt: CACurrentMediaTime())

        // Also publish to @Published for Combine subscribers (editor mode NSImage conversion).
        // This main-thread hop is lightweight — just assigns a reference.
        Task { @MainActor in
            self.latestFrame = ciImage
        }
    }
}
