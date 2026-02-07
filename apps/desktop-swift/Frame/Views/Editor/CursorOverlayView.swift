import SwiftUI

/// Renders cursor position, highlight circle, and click ripple effects
/// as an overlay on the preview canvas.
struct CursorOverlayView: View {
    let cursorEvents: [CursorEvent]
    let effects: EffectsConfig
    let currentTime: TimeInterval
    let videoSize: CGSize        // Original video resolution
    let canvasSize: CGSize       // Rendered canvas size (with padding)

    var body: some View {
        if effects.cursorEnabled {
            Canvas { context, size in
                guard let event = currentEvent else { return }

                let scale = canvasScale(in: size)
                let cursorPoint = scaledCursorPoint(event: event, in: size, scale: scale)

                // Click highlight ring
                if isClicking {
                    drawClickRipple(context: &context, at: cursorPoint, scale: scale)
                }

                // Cursor highlight circle
                if effects.cursorHighlight {
                    drawHighlight(context: &context, at: cursorPoint, scale: scale)
                }

                // Cursor dot (always visible)
                drawCursor(context: &context, at: cursorPoint, scale: scale)
            }
        }
    }

    // MARK: - Current Event

    /// Find the cursor event closest to current playback time
    private var currentEvent: CursorEvent? {
        guard !cursorEvents.isEmpty else { return nil }

        // Binary search for closest event
        var lo = 0
        var hi = cursorEvents.count - 1

        while lo < hi {
            let mid = (lo + hi) / 2
            if cursorEvents[mid].timestamp < currentTime {
                lo = mid + 1
            } else {
                hi = mid
            }
        }

        // Check neighbors for closest
        if lo > 0 {
            let prev = cursorEvents[lo - 1]
            let curr = cursorEvents[lo]
            if abs(prev.timestamp - currentTime) < abs(curr.timestamp - currentTime) {
                return prev
            }
        }
        return cursorEvents[lo]
    }

    /// Check if there's a click event near the current time
    private var isClicking: Bool {
        let clickWindow: TimeInterval = 0.3
        return cursorEvents.contains { event in
            (event.type == .leftClick || event.type == .rightClick) &&
            abs(event.timestamp - currentTime) < clickWindow
        }
    }

    // MARK: - Coordinate Mapping

    private func canvasScale(in size: CGSize) -> CGFloat {
        guard videoSize.width > 0 && videoSize.height > 0 else { return 1 }
        let scaleX = size.width / videoSize.width
        let scaleY = size.height / videoSize.height
        return min(scaleX, scaleY)
    }

    private func scaledCursorPoint(event: CursorEvent, in size: CGSize, scale: CGFloat) -> CGPoint {
        let padding = effects.padding * scale
        // Cursor coords are in screen space (origin bottom-left). 
        // Convert to view space (origin top-left).
        let x = event.x * scale + padding
        let y = (videoSize.height - event.y) * scale + padding
        return CGPoint(x: x, y: y)
    }

    // MARK: - Drawing

    private func drawCursor(context: inout GraphicsContext, at point: CGPoint, scale: CGFloat) {
        let radius = 4.0 * effects.cursorScale * scale
        let circle = Path(ellipseIn: CGRect(
            x: point.x - radius,
            y: point.y - radius,
            width: radius * 2,
            height: radius * 2
        ))

        // White cursor dot with border
        context.fill(circle, with: .color(.white))
        context.stroke(circle, with: .color(.black.opacity(0.3)), lineWidth: 1)
    }

    private func drawHighlight(context: inout GraphicsContext, at point: CGPoint, scale: CGFloat) {
        let radius = 20.0 * effects.cursorScale * scale
        let circle = Path(ellipseIn: CGRect(
            x: point.x - radius,
            y: point.y - radius,
            width: radius * 2,
            height: radius * 2
        ))

        let highlightColor = Color(
            red: Double(effects.cursorClickColor.red),
            green: Double(effects.cursorClickColor.green),
            blue: Double(effects.cursorClickColor.blue),
            opacity: 0.15
        )

        context.fill(circle, with: .color(highlightColor))
    }

    private func drawClickRipple(context: inout GraphicsContext, at point: CGPoint, scale: CGFloat) {
        // Expanding ripple rings on click
        for i in 0..<3 {
            let rippleRadius = (30.0 + Double(i) * 15.0) * effects.cursorScale * scale
            let opacity = 0.3 - Double(i) * 0.1

            let circle = Path(ellipseIn: CGRect(
                x: point.x - rippleRadius,
                y: point.y - rippleRadius,
                width: rippleRadius * 2,
                height: rippleRadius * 2
            ))

            let rippleColor = Color(
                red: Double(effects.cursorClickColor.red),
                green: Double(effects.cursorClickColor.green),
                blue: Double(effects.cursorClickColor.blue),
                opacity: opacity
            )

            context.stroke(circle, with: .color(rippleColor), lineWidth: 2 * scale)
        }
    }
}
