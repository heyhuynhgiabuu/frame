import Foundation
import SwiftUI

/// Computes an animated zoom transform at the current playback time,
/// zooming into cursor click positions with smooth ease-in/out.
@MainActor
final class ZoomEngine: ObservableObject {

    struct ZoomState {
        var scale: CGFloat = 1.0
        var offsetX: CGFloat = 0
        var offsetY: CGFloat = 0
        var isZoomed: Bool = false
    }

    @Published var currentZoom = ZoomState()

    // MARK: - Configuration

    /// Duration of the zoom-in animation in seconds
    var zoomInDuration: TimeInterval = 0.4

    /// Duration of the zoom-out animation in seconds
    var zoomOutDuration: TimeInterval = 0.3

    /// How long to hold the zoom before zooming out (seconds)
    var holdDuration: TimeInterval = 1.5

    // MARK: - Computation

    /// Compute the zoom state at a given playback time.
    ///
    /// - Parameters:
    ///   - time: Current playback time
    ///   - events: Recorded cursor events
    ///   - effects: Current effects config (zoom scale, animation speed, enabled)
    ///   - videoSize: Original video dimensions
    func update(
        time: TimeInterval,
        events: [CursorEvent],
        effects: EffectsConfig,
        videoSize: CGSize
    ) {
        guard effects.autoZoomEnabled else {
            currentZoom = ZoomState()
            return
        }

        let targetScale = CGFloat(effects.zoomScale)
        let speed = effects.zoomAnimationStyle.speedMultiplier

        // Find click events and compute zoom for the closest one
        let clickEvents = events.filter { $0.type == .leftClick || $0.type == .rightClick }
        guard !clickEvents.isEmpty else {
            currentZoom = ZoomState()
            return
        }

        // Find the active click (most recent click that is in zoom range)
        let totalZoomDuration = (zoomInDuration + holdDuration + zoomOutDuration) / speed
        var activeClick: CursorEvent?
        var timeSinceClick: TimeInterval = 0

        for click in clickEvents.reversed() {
            let dt = time - click.timestamp
            if dt >= 0 && dt < totalZoomDuration {
                activeClick = click
                timeSinceClick = dt
                break
            }
        }

        guard let click = activeClick else {
            currentZoom = ZoomState()
            return
        }

        // Compute animation phase
        let zoomIn = zoomInDuration / speed
        let hold = holdDuration / speed
        let zoomOut = zoomOutDuration / speed

        let animProgress: CGFloat
        if timeSinceClick < zoomIn {
            // Zooming in
            animProgress = easeInOut(CGFloat(timeSinceClick / zoomIn))
        } else if timeSinceClick < zoomIn + hold {
            // Holding
            animProgress = 1.0
        } else {
            // Zooming out
            let outProgress = (timeSinceClick - zoomIn - hold) / zoomOut
            animProgress = 1.0 - easeInOut(CGFloat(min(1, outProgress)))
        }

        let scale = 1.0 + (targetScale - 1.0) * animProgress

        // Compute offset to center the zoom on the click position
        // Normalized click position (0-1)
        let nx = CGFloat(click.x) / videoSize.width
        let ny = 1.0 - (CGFloat(click.y) / videoSize.height)   // Flip Y

        // Offset so the click point stays centered during zoom
        let offsetX = -(nx - 0.5) * videoSize.width * (scale - 1)
        let offsetY = -(ny - 0.5) * videoSize.height * (scale - 1)

        currentZoom = ZoomState(
            scale: scale,
            offsetX: offsetX,
            offsetY: offsetY,
            isZoomed: animProgress > 0.01
        )
    }

    // MARK: - Easing

    /// Cubic ease-in-out
    private func easeInOut(_ t: CGFloat) -> CGFloat {
        if t < 0.5 {
            return 4 * t * t * t
        } else {
            let p = 2 * t - 2
            return 0.5 * p * p * p + 1
        }
    }
}

// MARK: - ZoomAnimationStyle Extensions

extension ZoomAnimationStyle {
    /// Speed multiplier for animation timing
    var speedMultiplier: Double {
        switch self {
        case .slow: return 0.6
        case .mellow: return 1.0
        case .quick: return 1.5
        case .rapid: return 2.5
        }
    }
}
