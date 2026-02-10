import SwiftUI
import CoreImage
import AVFoundation

/// Renders a draggable webcam PiP overlay in the preview canvas.
/// Supports two modes:
/// - Live preview: Shows NSImage from WebcamCaptureEngine (during recording)
/// - Playback: Shows recorded webcam video via AVPlayer (in editor mode)
/// - Draggable: Users can drag to reposition the webcam overlay
struct WebcamOverlayView: View {
    let effects: EffectsConfig
    let webcamImage: NSImage?
    let webcamPlayer: AVPlayer?
    let containerSize: CGSize
    let isEditable: Bool  // Enable dragging in editor mode
    var onOffsetChanged: ((Double, Double) -> Void)? = nil  // Callback when drag ends

    @State private var dragOffset: CGSize = .zero

    private static let ciContext = CIContext(options: [.useSoftwareRenderer: false])

    var body: some View {
        if effects.webcamEnabled, hasContent {
            let pipSize = computePiPSize()

            webcamContent
                .aspectRatio(contentMode: .fill)
                .frame(width: pipSize.width, height: pipSize.height)
                .clipShape(clipShape(for: pipSize))
                .overlay {
                    borderShape(for: pipSize)
                }
                .shadow(color: .black.opacity(0.3), radius: 8, y: 4)
                .padding(16)
                .offset(x: effectiveOffsetX, y: effectiveOffsetY)
                .gesture(
                    isEditable ? dragGesture : nil
                )
                .frame(
                    maxWidth: .infinity,
                    maxHeight: .infinity,
                    alignment: webcamAlignment
                )
        }
    }

    // MARK: - Content

    private var hasContent: Bool {
        webcamImage != nil || webcamPlayer != nil
    }

    @ViewBuilder
    private var webcamContent: some View {
        if let player = webcamPlayer {
            // Editor mode: play recorded webcam video
            VideoPlayerView(player: player)
        } else if let image = webcamImage {
            // Live mode: show current webcam frame
            Image(nsImage: image)
                .resizable()
        }
    }

    // MARK: - Shape

    private func clipShape(for size: CGSize) -> some Shape {
        WebcamClipShape(style: effects.webcamShape, cornerRadius: size.width * 0.15)
    }

    @ViewBuilder
    private func borderShape(for size: CGSize) -> some View {
        WebcamClipShape(style: effects.webcamShape, cornerRadius: size.width * 0.15)
            .stroke(.white.opacity(0.3), lineWidth: 2)
    }

    // MARK: - Alignment

    private var webcamAlignment: Alignment {
        switch effects.webcamPosition {
        case .topLeft: return .topLeading
        case .topRight: return .topTrailing
        case .bottomLeft: return .bottomLeading
        case .bottomRight: return .bottomTrailing
        }
    }

    // MARK: - Sizing

    private func computePiPSize() -> CGSize {
        let fraction = effects.webcamSize
        let side = containerSize.width * fraction
        return CGSize(width: max(40, side), height: max(40, side))
    }

    // MARK: - Draggable Offset

    private var effectiveOffsetX: CGFloat {
        let baseOffset = effects.webcamOffsetX * containerSize.width * 0.3
        return baseOffset + dragOffset.width
    }

    private var effectiveOffsetY: CGFloat {
        let baseOffset = effects.webcamOffsetY * containerSize.height * 0.3
        return baseOffset + dragOffset.height
    }

    private var dragGesture: some Gesture {
        DragGesture()
            .onChanged { value in
                dragOffset = value.translation
            }
            .onEnded { value in
                // Convert final position to normalized offset
                let normalizedX = (effects.webcamOffsetX * containerSize.width * 0.3 + value.translation.width) / (containerSize.width * 0.3)
                let normalizedY = (effects.webcamOffsetY * containerSize.height * 0.3 + value.translation.height) / (containerSize.height * 0.3)

                // Clamp to reasonable bounds (-1 to 1)
                let clampedX = max(-1.0, min(1.0, normalizedX))
                let clampedY = max(-1.0, min(1.0, normalizedY))

                // Notify parent via callback
                onOffsetChanged?(clampedX, clampedY)

                // Reset drag offset since we've committed to the model
                dragOffset = .zero
            }
    }

    // MARK: - CIImage → NSImage Conversion

    /// Convert a CIImage webcam frame to NSImage for display.
    static func convertToNSImage(_ ciImage: CIImage) -> NSImage? {
        let extent = ciImage.extent
        guard let cgImage = ciContext.createCGImage(ciImage, from: extent) else {
            return nil
        }
        return NSImage(
            cgImage: cgImage,
            size: NSSize(width: extent.width, height: extent.height)
        )
    }
}

// MARK: - Webcam Shape (Shape protocol)

/// A shape that can be circle, rounded rect, or rectangle based on the webcam config.
struct WebcamClipShape: Shape {
    let style: WebcamShape
    let cornerRadius: CGFloat

    func path(in rect: CGRect) -> Path {
        switch style {
        case .circle:
            return Circle().path(in: rect)
        case .roundedRectangle:
            return RoundedRectangle(cornerRadius: cornerRadius).path(in: rect)
        case .rectangle:
            return Rectangle().path(in: rect)
        }
    }
}
