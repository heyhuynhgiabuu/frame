import SwiftUI

/// A live preview canvas that renders the video with background, padding,
/// corner radius, and shadow effects applied in real-time — WYSIWYG editing.
struct PreviewCanvas: View {
    let player: AVPlayer
    let effects: EffectsConfig
    let isReady: Bool
    var loadError: String?
    var cursorEvents: [CursorEvent] = []
    var currentTime: TimeInterval = 0
    var videoSize: CGSize = CGSize(width: 1920, height: 1080)
    var webcamImage: NSImage?
    var webcamPlayer: AVPlayer?
    var zoomState: ZoomEngine.ZoomState = ZoomEngine.ZoomState()
    var keystrokeEvents: [KeystrokeEvent] = []
    var isEditable: Bool = false  // Enable drag gestures for webcam, etc.
    var onWebcamOffsetChanged: ((Double, Double) -> Void)? = nil  // Callback for position updates

    var body: some View {
        GeometryReader { geometry in
            // Center the styled frame in the available space
            let scale = fitScale(
                canvasSize: geometry.size,
                contentPadding: CGFloat(effects.padding)
            )

            ZStack {
                // Dark workspace background
                Color(nsColor: .controlBackgroundColor)
                    .opacity(0.3)

                // The styled frame: background + video
                styledFrame(scale: scale)
                    .frame(
                        width: geometry.size.width * 0.85,
                        height: geometry.size.height * 0.85
                    )
                    .scaleEffect(zoomState.scale)
                    .offset(
                        x: zoomState.offsetX * scale,
                        y: zoomState.offsetY * scale
                    )
                    .animation(
                        zoomState.isZoomed
                            ? .easeInOut(duration: 0.3)
                            : .easeOut(duration: 0.2),
                        value: zoomState.scale
                    )
                    .clipped()
            }
        }
    }

    // MARK: - Styled Frame

    private func styledFrame(scale: CGFloat) -> some View {
        ZStack {
            // Shadow layer
            RoundedRectangle(cornerRadius: CGFloat(effects.cornerRadius) * scale)
                .fill(Color.black.opacity(0.01))   // Invisible carrier for shadow
                .shadow(
                    color: .black.opacity(Double(effects.shadowOpacity)),
                    radius: CGFloat(effects.shadowBlur) * scale * 0.5,
                    x: 0,
                    y: CGFloat(effects.shadowOffsetY) * scale
                )
                .padding(CGFloat(effects.padding) * scale)

            // Background layer (gradient, solid, or wallpaper)
            backgroundLayer
                .clipShape(RoundedRectangle(cornerRadius: 0))

            // Video layer
            VideoPlayerView(player: player)
                .clipShape(
                    RoundedRectangle(cornerRadius: CGFloat(effects.cornerRadius) * scale)
                )
                .padding(CGFloat(effects.padding) * scale)
                .overlay {
                    // Cursor overlay (rendered on top of video)
                    if !cursorEvents.isEmpty {
                        CursorOverlayView(
                            cursorEvents: cursorEvents,
                            effects: effects,
                            currentTime: currentTime,
                            videoSize: videoSize,
                            canvasSize: CGSize(
                                width: videoSize.width + effects.padding * 2,
                                height: videoSize.height + effects.padding * 2
                            )
                        )
                    }
                }

            // Webcam PiP overlay
            // In editor mode: uses webcamPlayer (plays recorded webcam .mov, synced with main video)
            // In recorder mode: uses webcamImage (live preview from capture engine)
            if webcamPlayer != nil || webcamImage != nil {
                WebcamOverlayView(
                    effects: effects,
                    webcamImage: webcamImage,
                    webcamPlayer: webcamPlayer,
                    containerSize: CGSize(
                        width: videoSize.width + effects.padding * 2,
                        height: videoSize.height + effects.padding * 2
                    ),
                    isEditable: isEditable,
                    onOffsetChanged: onWebcamOffsetChanged
                )
            }

            // Keystroke overlay
            KeystrokeOverlayView(
                keystrokeEvents: keystrokeEvents,
                effects: effects,
                currentTime: currentTime
            )

            // Loading / error overlay
            if let loadError {
                VStack(spacing: 12) {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .font(.system(size: 36))
                        .foregroundStyle(.yellow)
                    Text("Failed to load video")
                        .font(.headline)
                        .foregroundStyle(.white)
                    Text(loadError)
                        .font(.caption)
                        .foregroundStyle(.white.opacity(0.7))
                        .multilineTextAlignment(.center)
                        .frame(maxWidth: 300)
                }
            } else if !isReady {
                ProgressView("Loading…")
                    .foregroundStyle(.white)
            }
        }
    }

    // MARK: - Background

    @ViewBuilder
    private var backgroundLayer: some View {
        switch effects.backgroundType {
        case .gradient:
            gradientBackground
        case .solid:
            Color(
                red: Double(effects.backgroundColor.red),
                green: Double(effects.backgroundColor.green),
                blue: Double(effects.backgroundColor.blue),
                opacity: Double(effects.backgroundColor.alpha)
            )
        case .wallpaper:
            // Wallpaper: dark neutral fallback
            LinearGradient(
                colors: [Color(white: 0.15), Color(white: 0.08)],
                startPoint: .top,
                endPoint: .bottom
            )
        case .image:
            if let url = effects.backgroundImageURL {
                AsyncImage(url: url) { image in
                    image.resizable().scaledToFill()
                } placeholder: {
                    Color(white: 0.1)
                }
            } else {
                Color(white: 0.1)
            }
        }
    }

    private var gradientBackground: some View {
        Group {
            if let preset = GradientPreset.allPresets.first(where: { $0.id == effects.gradientPresetID }) {
                LinearGradient(
                    colors: preset.colors,
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )
            } else {
                LinearGradient(
                    colors: [.blue, .purple],
                    startPoint: .topLeading,
                    endPoint: .bottomTrailing
                )
            }
        }
    }

    // MARK: - Scaling

    /// Calculate scale factor to fit the styled content within the canvas
    private func fitScale(canvasSize: CGSize, contentPadding: CGFloat) -> CGFloat {
        // Use a reference resolution (1920×1080) to normalize scale
        let refWidth: CGFloat = 1920
        let refHeight: CGFloat = 1080

        let totalWidth = refWidth + contentPadding * 2
        let totalHeight = refHeight + contentPadding * 2

        let scaleX = canvasSize.width * 0.85 / totalWidth
        let scaleY = canvasSize.height * 0.85 / totalHeight

        return min(scaleX, scaleY, 1.0)
    }
}

import AVFoundation

#Preview {
    PreviewCanvas(
        player: AVPlayer(),
        effects: EffectsConfig(),
        isReady: true
    )
}
