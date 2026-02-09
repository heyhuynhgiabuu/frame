import AppKit
import SwiftUI

// MARK: - AreaSelectionManager

/// Manages the area selection overlay lifecycle.
/// Shows a full-screen transparent panel where the user can click-and-drag
/// to define a recording region (Screen Studio-style spotlight pattern).
@MainActor
final class AreaSelectionManager {
    private var panels: [AreaSelectionWindow] = []
    private weak var appState: AppState?
    var isActive: Bool { !panels.isEmpty }
    /// Whether the user has drawn an area selection in this session.
    private(set) var hasDrawnSelection: Bool = false

    func startSelection(appState: AppState) {
        // Clean up any existing panels without resetting card state
        cleanUpPanels()
        self.appState = appState
        hasDrawnSelection = false

        // Create one panel per screen
        for screen in NSScreen.screens {
            let panel = AreaSelectionWindow(screen: screen, manager: self)
            panel.makeKeyAndOrderFront(nil)
            panels.append(panel)
        }

        // Set crosshair cursor
        NSCursor.crosshair.push()
    }

    /// Full dismiss: removes panels AND hides the info card (called by Escape / hideRecorderOverlays).
    func dismiss() {
        cleanUpPanels()
        hasDrawnSelection = false

        // Also dismiss the area info card and reset card state
        if let appState {
            appState.hasUserExplicitlySelectedCaptureModeForCard = false
            appState.overlayManager.updateDisplayCardVisibility(appState: appState)
        }
    }

    /// Internal cleanup: removes panels without touching card state.
    private func cleanUpPanels() {
        NSCursor.pop()
        for panel in panels {
            panel.orderOut(nil)
        }
        panels.removeAll()
    }

    /// Called when the user finishes dragging a selection rectangle.
    /// Updates AppState area settings but keeps the overlay alive for
    /// further adjustments (resize handles, move). The overlay is dismissed
    /// when the user starts recording, switches capture mode, or presses Escape.
    func didFinishSelection(rect: NSRect, on screen: NSScreen) {
        guard let appState else { return }

        hasDrawnSelection = true
        // Ensure the info card can show now that we have a selection
        appState.hasUserExplicitlySelectedCaptureModeForCard = true

        // Convert global screen coordinates to display-local coordinates.
        // The selection rect is in AppKit global coords (bottom-left origin).
        // SCStreamConfiguration.sourceRect expects display-local coords (top-left origin, in points).
        let screenFrame = screen.frame

        // Step 1: Get display-local position (subtract screen origin)
        let localX = rect.origin.x - screenFrame.origin.x
        // Step 2: Flip Y from bottom-left to top-left, relative to this screen's height
        let localY = screenFrame.height - (rect.origin.y - screenFrame.origin.y) - rect.height

        // Step 3: Clamp to display bounds
        let clampedX = max(0, min(localX, screenFrame.width - rect.width))
        let clampedY = max(0, min(localY, screenFrame.height - rect.height))
        let clampedW = max(2, min(rect.width, screenFrame.width - clampedX))
        let clampedH = max(2, min(rect.height, screenFrame.height - clampedY))

        let relativeX = Int(clampedX)
        let relativeY = Int(clampedY)
        let width = Int(clampedW)
        let height = Int(clampedH)

        appState.recorderToolbarSettings.areaX = relativeX
        appState.recorderToolbarSettings.areaY = relativeY
        appState.recorderToolbarSettings.areaWidth = width
        appState.recorderToolbarSettings.areaHeight = height
        appState.recorderToolbarSettings.save()

        // Match the SCDisplay for the screen where the area was drawn
        let screenNumber = screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber
        if let screenID = screenNumber?.uint32Value,
           let matchedDisplay = appState.availableDisplays.first(where: { $0.displayID == screenID }) {
            appState.setSelectedDisplay(matchedDisplay)
        } else {
            // Fallback: match by frame position
            let matchedByFrame = appState.availableDisplays.first(where: { display in
                abs(CGFloat(display.frame.origin.x) - screenFrame.origin.x) < 2 &&
                abs(CGFloat(display.frame.origin.y) - screenFrame.origin.y) < 2
            })
            if let matched = matchedByFrame {
                appState.setSelectedDisplay(matched)
            }
        }

        // Update the info card to reflect the new values
        appState.overlayManager.updateDisplayCardVisibility(appState: appState)
    }
}

// MARK: - AreaSelectionWindow

/// Full-screen borderless window for area selection.
/// Uses `sharingType = .none` to self-exclude from screen capture.
private class AreaSelectionWindow: NSPanel {
    let manager: AreaSelectionManager
    let selectionView: AreaSelectionView

    init(screen: NSScreen, manager: AreaSelectionManager) {
        self.manager = manager
        self.selectionView = AreaSelectionView(manager: manager, screen: screen)

        super.init(
            contentRect: screen.frame,
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )

        isOpaque = false
        backgroundColor = .clear
        // Above normal windows but below our toolbar/info card panels (.floating = 3)
        level = NSWindow.Level(rawValue: NSWindow.Level.floating.rawValue - 1)
        hasShadow = false
        ignoresMouseEvents = false
        collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        sharingType = .none  // Exclude from screen capture

        contentView = selectionView
    }

    // Allow this window to become key to receive mouse events
    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { false }

    // Escape key cancels selection
    override func keyDown(with event: NSEvent) {
        if event.keyCode == 53 { // Escape
            manager.dismiss()
        } else {
            super.keyDown(with: event)
        }
    }
}

// MARK: - AreaSelectionView

/// Custom NSView that handles the click-drag interaction and draws
/// the dimmed overlay with a spotlight "hole" for the selection.
private class AreaSelectionView: NSView {
    private let manager: AreaSelectionManager
    private let screen: NSScreen

    // Selection state
    private var dragOrigin: NSPoint?
    private var currentRect: NSRect?
    private var selectionPhase: SelectionPhase = .idle

    // Resize handles
    private var activeHandle: ResizeHandle?
    private var handleRects: [ResizeHandle: NSRect] = [:]

    enum SelectionPhase {
        case idle       // Waiting for initial click-drag
        case drawing    // Currently dragging to create rectangle
        case adjusting  // Rectangle drawn, user can resize/move
        case moving     // Dragging the entire selection
        case resizing   // Resizing via a handle
    }

    enum ResizeHandle: CaseIterable {
        case topLeft, topCenter, topRight
        case middleLeft, middleRight
        case bottomLeft, bottomCenter, bottomRight
    }

    init(manager: AreaSelectionManager, screen: NSScreen) {
        self.manager = manager
        self.screen = screen
        super.init(frame: screen.frame)

        // Track mouse movement for cursor changes
        let trackingArea = NSTrackingArea(
            rect: .zero,
            options: [.activeAlways, .mouseMoved, .inVisibleRect],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(trackingArea)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    // MARK: - Drawing

    override func draw(_ dirtyRect: NSRect) {
        guard let context = NSGraphicsContext.current?.cgContext else { return }

        // Draw the dim overlay with spotlight hole
        let dimColor = NSColor.black.withAlphaComponent(0.45)

        if let rect = currentRect, rect.width > 1, rect.height > 1 {
            // Even-odd fill: dim everywhere except the selection rectangle
            let path = NSBezierPath(rect: bounds)
            path.append(NSBezierPath(roundedRect: rect, xRadius: 2, yRadius: 2))
            path.windingRule = .evenOdd
            dimColor.set()
            path.fill()

            // Selection border — white with slight glow
            context.saveGState()
            context.setShadow(offset: .zero, blur: 6, color: NSColor.white.withAlphaComponent(0.3).cgColor)
            NSColor.white.withAlphaComponent(0.9).set()
            let borderPath = NSBezierPath(roundedRect: rect, xRadius: 2, yRadius: 2)
            borderPath.lineWidth = 1.5
            borderPath.stroke()
            context.restoreGState()

            // Resize handles (only in adjusting phase)
            if selectionPhase == .adjusting {
                drawResizeHandles(for: rect)
            }
        } else {
            // No selection yet — dim everything
            dimColor.set()
            bounds.fill()
        }
    }

    private func drawResizeHandles(for rect: NSRect) {
        let handleSize: CGFloat = 8
        let half = handleSize / 2

        let positions: [ResizeHandle: NSPoint] = [
            .topLeft: NSPoint(x: rect.minX, y: rect.maxY),
            .topCenter: NSPoint(x: rect.midX, y: rect.maxY),
            .topRight: NSPoint(x: rect.maxX, y: rect.maxY),
            .middleLeft: NSPoint(x: rect.minX, y: rect.midY),
            .middleRight: NSPoint(x: rect.maxX, y: rect.midY),
            .bottomLeft: NSPoint(x: rect.minX, y: rect.minY),
            .bottomCenter: NSPoint(x: rect.midX, y: rect.minY),
            .bottomRight: NSPoint(x: rect.maxX, y: rect.minY),
        ]

        handleRects.removeAll()

        for (handle, center) in positions {
            let handleRect = NSRect(
                x: center.x - half,
                y: center.y - half,
                width: handleSize,
                height: handleSize
            )
            handleRects[handle] = handleRect

            // Draw handle
            let path = NSBezierPath(
                roundedRect: handleRect,
                xRadius: 2,
                yRadius: 2
            )
            NSColor.white.set()
            path.fill()
            NSColor.white.withAlphaComponent(0.3).set()
            path.lineWidth = 1
            path.stroke()
        }
    }

    // MARK: - Mouse Events

    override func mouseDown(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)

        switch selectionPhase {
        case .idle:
            // Start drawing a new selection
            dragOrigin = point
            currentRect = NSRect(origin: point, size: .zero)
            selectionPhase = .drawing

        case .adjusting:
            // Check if clicking on a resize handle
            if let handle = handleAtPoint(point) {
                activeHandle = handle
                dragOrigin = point
                selectionPhase = .resizing
            } else if let rect = currentRect, rect.contains(point) {
                // Clicking inside the selection — start moving
                dragOrigin = point
                selectionPhase = .moving
            } else {
                // Clicking outside — start a new selection
                dragOrigin = point
                currentRect = NSRect(origin: point, size: .zero)
                selectionPhase = .drawing
            }

        default:
            break
        }
    }

    override func mouseDragged(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)

        switch selectionPhase {
        case .drawing:
            guard let origin = dragOrigin else { return }
            currentRect = rectFromPoints(origin, point)
            needsDisplay = true

        case .moving:
            guard let origin = dragOrigin, var rect = currentRect else { return }
            let delta = NSPoint(x: point.x - origin.x, y: point.y - origin.y)
            rect.origin.x += delta.x
            rect.origin.y += delta.y
            currentRect = rect
            dragOrigin = point
            needsDisplay = true

        case .resizing:
            guard let handle = activeHandle, let origin = dragOrigin, var rect = currentRect else { return }
            let delta = NSPoint(x: point.x - origin.x, y: point.y - origin.y)
            rect = applyResize(handle: handle, delta: delta, to: rect)
            currentRect = rect
            dragOrigin = point
            needsDisplay = true

        default:
            break
        }
    }

    override func mouseUp(with event: NSEvent) {
        switch selectionPhase {
        case .drawing:
            if let rect = currentRect, rect.width > 10, rect.height > 10 {
                selectionPhase = .adjusting
                needsDisplay = true
                // Report the selection
                manager.didFinishSelection(rect: rect, on: screen)
            } else {
                // Too small, reset
                currentRect = nil
                selectionPhase = .idle
                needsDisplay = true
            }

        case .moving, .resizing:
            selectionPhase = .adjusting
            activeHandle = nil
            needsDisplay = true
            if let rect = currentRect {
                manager.didFinishSelection(rect: rect, on: screen)
            }

        default:
            break
        }
    }

    override func mouseMoved(with event: NSEvent) {
        guard selectionPhase == .adjusting else { return }
        let point = convert(event.locationInWindow, from: nil)

        if let handle = handleAtPoint(point) {
            setCursorForHandle(handle)
        } else if let rect = currentRect, rect.contains(point) {
            NSCursor.openHand.set()
        } else {
            NSCursor.crosshair.set()
        }
    }

    // MARK: - Helpers

    private func rectFromPoints(_ a: NSPoint, _ b: NSPoint) -> NSRect {
        let x = min(a.x, b.x)
        let y = min(a.y, b.y)
        let w = abs(a.x - b.x)
        let h = abs(a.y - b.y)
        return NSRect(x: x, y: y, width: w, height: h)
    }

    private func handleAtPoint(_ point: NSPoint) -> ResizeHandle? {
        let hitPadding: CGFloat = 6
        for (handle, rect) in handleRects {
            if rect.insetBy(dx: -hitPadding, dy: -hitPadding).contains(point) {
                return handle
            }
        }
        return nil
    }

    private func applyResize(handle: ResizeHandle, delta: NSPoint, to rect: NSRect) -> NSRect {
        var r = rect
        switch handle {
        case .topLeft:
            r.origin.x += delta.x
            r.size.width -= delta.x
            r.size.height += delta.y
        case .topCenter:
            r.size.height += delta.y
        case .topRight:
            r.size.width += delta.x
            r.size.height += delta.y
        case .middleLeft:
            r.origin.x += delta.x
            r.size.width -= delta.x
        case .middleRight:
            r.size.width += delta.x
        case .bottomLeft:
            r.origin.x += delta.x
            r.origin.y += delta.y
            r.size.width -= delta.x
            r.size.height -= delta.y
        case .bottomCenter:
            r.origin.y += delta.y
            r.size.height -= delta.y
        case .bottomRight:
            r.size.width += delta.x
            r.origin.y += delta.y
            r.size.height -= delta.y
        }
        // Enforce minimum size
        if r.width < 20 { r.size.width = 20 }
        if r.height < 20 { r.size.height = 20 }
        return r
    }

    private func setCursorForHandle(_ handle: ResizeHandle) {
        switch handle {
        case .topLeft, .bottomRight:
            if let image = NSImage(systemSymbolName: "arrow.up.left.and.arrow.down.right", accessibilityDescription: nil) {
                NSCursor(image: image, hotSpot: NSPoint(x: 8, y: 8)).set()
            } else {
                NSCursor.crosshair.set()
            }
        case .topRight, .bottomLeft:
            if let image = NSImage(systemSymbolName: "arrow.up.right.and.arrow.down.left", accessibilityDescription: nil) {
                NSCursor(image: image, hotSpot: NSPoint(x: 8, y: 8)).set()
            } else {
                NSCursor.crosshair.set()
            }
        case .topCenter, .bottomCenter:
            NSCursor.resizeUpDown.set()
        case .middleLeft, .middleRight:
            NSCursor.resizeLeftRight.set()
        }
    }
}
