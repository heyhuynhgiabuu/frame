import SwiftUI

/// Renders captions synced to video playback on the preview canvas.
///
/// Follows the same overlay pattern as `KeystrokeOverlayView`:
/// positioned at the bottom of the frame using `GeometryReader` + `Spacer`.
struct CaptionOverlayView: View {
    let captionSegments: [CaptionSegment]
    let effects: EffectsConfig
    let currentTime: TimeInterval

    var body: some View {
        if effects.captionsEnabled, let activeSegment = activeSegment {
            GeometryReader { geometry in
                VStack {
                    Spacer()

                    Text(activeSegment.text)
                        .font(.system(size: effects.captionFontSize, weight: .semibold))
                        .foregroundStyle(.white)
                        .shadow(color: .black.opacity(0.8), radius: 2, x: 0, y: 1)
                        .shadow(color: .black.opacity(0.4), radius: 4, x: 0, y: 2)
                        .multilineTextAlignment(.center)
                        .lineLimit(3)
                        .padding(.horizontal, 20)
                        .padding(.vertical, 8)
                        .background(
                            RoundedRectangle(cornerRadius: 6)
                                .fill(.black.opacity(0.6))
                        )
                        .frame(maxWidth: geometry.size.width * 0.85)
                        .padding(.bottom, geometry.size.height * 0.08)
                        .frame(maxWidth: .infinity)
                        .transition(.opacity)
                        .animation(.easeInOut(duration: 0.15), value: activeSegment.id)
                }
            }
        }
    }

    /// Find the caption segment active at the current playback time.
    private var activeSegment: CaptionSegment? {
        captionSegments.first { $0.isActive(at: currentTime) }
    }
}
