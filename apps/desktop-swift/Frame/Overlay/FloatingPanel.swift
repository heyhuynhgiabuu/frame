import AppKit
import SwiftUI

/// A reusable floating NSPanel that sits on top of all windows without stealing focus.
/// Use this as a base for recording overlays, toolbars, and webcam previews.
///
/// Key behaviors:
/// - Non-activating: Doesn't steal focus from the user's current app
/// - Always on top: Stays above all windows (including fullscreen apps)
/// - All spaces: Appears on every macOS Space/desktop
/// - Transparent: Allows custom-shaped SwiftUI content
/// - Self-excluding: Sets `sharingType = .none` so ScreenCaptureKit ignores it
class FloatingPanel<Content: View>: NSPanel {

    // MARK: - Initialization

    init(
        contentRect: NSRect = NSRect(x: 0, y: 0, width: 400, height: 60),
        @ViewBuilder content: @escaping () -> Content
    ) {
        super.init(
            contentRect: contentRect,
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )

        configurePanel()
        setupContent(content)
    }

    // MARK: - Panel Configuration

    private func configurePanel() {
        // Appearance
        isOpaque = false
        backgroundColor = .clear
        hasShadow = true
        titlebarAppearsTransparent = true
        titleVisibility = .hidden

        // Behavior — don't activate when clicked, stay floating
        level = .floating
        isFloatingPanel = true
        hidesOnDeactivate = false
        animationBehavior = .utilityWindow

        // Multi-space support — visible on all Spaces + fullscreen apps
        collectionBehavior = [
            .canJoinAllSpaces,
            .fullScreenAuxiliary,
            .stationary,
        ]

        // Self-exclude from screen capture (macOS 14+)
        // This ensures the panel doesn't appear in ScreenCaptureKit recordings
        sharingType = .none

        // Allow mouse events to pass through transparent areas
        ignoresMouseEvents = false
        isMovableByWindowBackground = false
    }

    private func setupContent(_ content: @escaping () -> Content) {
        let hostingView = FirstMouseHostingView(rootView: content())
        hostingView.translatesAutoresizingMaskIntoConstraints = false

        contentView = hostingView
    }

    // MARK: - Key Handling

    /// Allow the panel to become key so SwiftUI buttons can receive click events.
    /// The `.nonactivatingPanel` style mask prevents app activation (focus stealing).
    override var canBecomeKey: Bool { true }

    /// Prevent the panel from becoming the main window
    override var canBecomeMain: Bool { false }

    // MARK: - Positioning Helpers

    /// Centers the panel horizontally at the bottom of the given screen, above the Dock.
    func positionAtBottomCenter(of screen: NSScreen, yOffset: CGFloat = 80) {
        let screenFrame = screen.visibleFrame
        let panelSize = frame.size

        let x = screenFrame.midX - (panelSize.width / 2)
        let y = screenFrame.minY + yOffset

        setFrameOrigin(NSPoint(x: x, y: y))
    }

    /// Positions the panel at the bottom-right of the given screen.
    func positionAtBottomRight(of screen: NSScreen, xOffset: CGFloat = 20, yOffset: CGFloat = 80) {
        let screenFrame = screen.visibleFrame
        let panelSize = frame.size

        let x = screenFrame.maxX - panelSize.width - xOffset
        let y = screenFrame.minY + yOffset

        setFrameOrigin(NSPoint(x: x, y: y))
    }

    /// Updates the SwiftUI content and resizes the panel to fit.
    func updateContent(@ViewBuilder _ content: @escaping () -> Content) {
        let hostingView = NSHostingView(rootView: content())
        hostingView.translatesAutoresizingMaskIntoConstraints = false
        contentView = hostingView

        // Resize to fit content
        let fittingSize = hostingView.fittingSize
        setContentSize(fittingSize)
    }

    // MARK: - Show / Hide

    func show(on screen: NSScreen? = nil) {
        let targetScreen = screen ?? NSScreen.main ?? NSScreen.screens.first
        guard targetScreen != nil else { return }

        orderFrontRegardless()
    }

    func dismiss() {
        orderOut(nil)
    }
}

// MARK: - First-Mouse Hosting View

/// NSHostingView subclass that accepts the first mouse click immediately,
/// so buttons work even when the panel isn't the key window yet.
private final class FirstMouseHostingView<Content: View>: NSHostingView<Content> {
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool {
        return true
    }
}
