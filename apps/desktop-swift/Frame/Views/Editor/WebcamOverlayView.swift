import SwiftUI
import CoreImage
import AVFoundation

/// Renders a webcam PiP overlay in the specified corner of the preview canvas.
/// Converts CIImage frames from WebcamCaptureEngine to a displayable NSImage.
struct WebcamOverlayView: View {
    let effects: EffectsConfig
    let webcamImage: NSImage?
    let containerSize: CGSize

    private static let ciContext = CIContext(options: [.useSoftwareRenderer: false])

    var body: some View {
        if effects.webcamEnabled, let image = webcamImage {
            let pipSize = computePiPSize()

            Image(nsImage: image)
                .resizable()
                .aspectRatio(contentMode: .fill)
                .frame(width: pipSize.width, height: pipSize.height)
                .clipShape(clipShape(for: pipSize))
                .overlay {
                    borderShape(for: pipSize)
                }
                .shadow(color: .black.opacity(0.3), radius: 8, y: 4)
                .padding(16)
                .frame(
                    maxWidth: .infinity,
                    maxHeight: .infinity,
                    alignment: webcamAlignment
                )
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
        let fraction = effects.webcamSize / 100.0
        let side = containerSize.width * fraction
        return CGSize(width: max(40, side), height: max(40, side))
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
