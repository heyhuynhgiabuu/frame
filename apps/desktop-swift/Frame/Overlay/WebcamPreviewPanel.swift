import AppKit
import CoreImage
import SwiftUI

// MARK: - WebcamPreviewPanel

/// Floating panel at bottom-right showing live webcam feed.
/// Draggable, rounded rectangle, with shadow.
final class WebcamPreviewPanel {
    private var panel: FloatingPanel<WebcamPreviewContent>?

    /// Full-frame: 16:9 rectangular (matches webcam). Square: 1:1 (center-cropped).
    private static let fullFrameSize = NSSize(width: 200, height: 112)
    private static let squareSize = NSSize(width: 200, height: 200)

    @MainActor
    func show(
        webcamEngine: WebcamCaptureEngine,
        appState: AppState,
        on screen: NSScreen? = nil
    ) {
        // Don't create a new panel if one is already visible
        if panel != nil { return }

        let isFullFrame = appState.recorderToolbarSettings.fullFrameWebcamPreview
        let panelSize = isFullFrame ? Self.fullFrameSize : Self.squareSize

        let content = WebcamPreviewContent(webcamEngine: webcamEngine, appState: appState)
        let panel = FloatingPanel(
            contentRect: NSRect(origin: .zero, size: panelSize)
        ) {
            content
        }

        // Make this panel draggable
        panel.isMovableByWindowBackground = true

        panel.positionAtBottomRight(of: screen ?? NSScreen.main ?? NSScreen.screens[0])
        panel.show()
        self.panel = panel
    }

    func dismiss() {
        panel?.dismiss()
        panel = nil
    }

    /// Returns the NSWindow number for SCContentFilter exclusion.
    var windowNumber: Int? {
        panel?.windowNumber
    }

    var nsWindow: NSWindow? {
        panel
    }
}

// MARK: - Webcam Preview SwiftUI Content

struct WebcamPreviewContent: View {
    var webcamEngine: WebcamCaptureEngine
    var appState: AppState

    private var isFullFrame: Bool {
        appState.recorderToolbarSettings.fullFrameWebcamPreview
    }

    /// Full-frame: 16:9 rectangular. Square: 1:1 (center-cropped).
    private var previewSize: NSSize {
        isFullFrame ? NSSize(width: 200, height: 112) : NSSize(width: 200, height: 200)
    }

    var body: some View {
        ZStack {
            if webcamEngine.isRunning {
                CIImageView(frameBox: webcamEngine.frameBox)
                    .frame(width: previewSize.width, height: previewSize.height)
                    .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
            } else {
                // Placeholder when no camera feed
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .fill(.black.opacity(0.6))
                    .frame(width: previewSize.width, height: previewSize.height)
                    .overlay {
                        VStack(spacing: 8) {
                            Image(systemName: "video.slash.fill")
                                .font(.system(size: 24))
                                .foregroundStyle(.white.opacity(0.5))
                            Text("No Camera")
                                .font(.caption)
                                .foregroundStyle(.white.opacity(0.5))
                        }
                    }
            }
        }
        .animation(.easeInOut(duration: 0.25), value: isFullFrame)
        .shadow(color: .black.opacity(0.4), radius: 8, x: 0, y: 4)
        .contextMenu {
            Toggle(
                "Full-frame webcam preview",
                isOn: Binding(
                    get: { appState.recorderToolbarSettings.fullFrameWebcamPreview },
                    set: { newValue in
                        appState.recorderToolbarSettings.fullFrameWebcamPreview = newValue
                        appState.recorderToolbarSettings.save()
                        // Resize the panel window to match the new preview size
                        appState.overlayManager.resizeWebcamPreview(fullFrame: newValue)
                    }
                )
            )

            Button("Hide camera preview") {
                appState.setHideCameraPreview(true)
            }
        }
    }
}

// MARK: - CIImageView — GPU-backed webcam preview

/// Renders CIImage frames directly onto a CALayer using CIContext.
/// This avoids the costly CIImage → NSImage → SwiftUI Image roundtrip
/// that causes freezes when the main thread is busy during recording.
struct CIImageView: NSViewRepresentable {
    let frameBox: WebcamFrameBox

    func makeNSView(context: Context) -> WebcamLayerView {
        let view = WebcamLayerView(frameBox: frameBox)
        return view
    }

    func updateNSView(_ nsView: WebcamLayerView, context: Context) {
        // frameBox is always the same reference; the view's display link
        // handles frame updates independently of SwiftUI's render cycle.
    }
}

/// NSView subclass that uses CVDisplayLink to drive webcam frame rendering
/// at display refresh rate, completely off the main thread's SwiftUI cycle.
final class WebcamLayerView: NSView {
    private let frameBox: WebcamFrameBox
    private let ciContext: CIContext
    private var displayLink: CVDisplayLink?
    private var lastRenderedTimestamp: CFTimeInterval = 0

    init(frameBox: WebcamFrameBox) {
        self.frameBox = frameBox
        // Use default Metal device for GPU-accelerated CIImage rendering
        self.ciContext = CIContext(options: [
            .useSoftwareRenderer: false,
            .cacheIntermediates: false,
        ])
        super.init(frame: .zero)

        wantsLayer = true
        layer?.contentsGravity = .resizeAspectFill
        layer?.masksToBounds = true
        layer?.backgroundColor = NSColor.black.cgColor

        startDisplayLink()
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) is not supported")
    }

    deinit {
        stopDisplayLink()
    }

    // MARK: - Display Link

    private func startDisplayLink() {
        var link: CVDisplayLink?
        CVDisplayLinkCreateWithActiveCGDisplays(&link)
        guard let link else { return }

        // Use a block-based callback via an opaque pointer to self
        let opaquePointer = Unmanaged.passUnretained(self).toOpaque()
        CVDisplayLinkSetOutputCallback(link, { _, _, _, _, _, userInfo -> CVReturn in
            guard let userInfo else { return kCVReturnError }
            let view = Unmanaged<WebcamLayerView>.fromOpaque(userInfo).takeUnretainedValue()
            view.renderFrame()
            return kCVReturnSuccess
        }, opaquePointer)

        CVDisplayLinkStart(link)
        self.displayLink = link
    }

    private func stopDisplayLink() {
        if let displayLink {
            CVDisplayLinkStop(displayLink)
        }
        displayLink = nil
    }

    /// Called from the CVDisplayLink thread — renders the latest webcam frame
    /// directly onto the layer's contents as a CGImage.
    private func renderFrame() {
        guard let snapshot = frameBox.snapshot else { return }

        // Skip if we already rendered this exact frame
        guard snapshot.capturedAt > lastRenderedTimestamp else { return }
        lastRenderedTimestamp = snapshot.capturedAt

        let ciImage = snapshot.image
        let bounds = ciImage.extent

        guard let cgImage = ciContext.createCGImage(ciImage, from: bounds) else { return }

        // CALayer.contents is thread-safe for CGImage assignment
        // CATransaction suppresses implicit animations for smooth video
        DispatchQueue.main.async { [weak self] in
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            self?.layer?.contents = cgImage
            CATransaction.commit()
        }
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if window == nil {
            stopDisplayLink()
        } else if displayLink == nil {
            startDisplayLink()
        }
    }
}
