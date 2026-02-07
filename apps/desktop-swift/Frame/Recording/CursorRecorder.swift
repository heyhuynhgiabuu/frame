import Foundation
import AppKit
import OSLog

private let logger = Logger(subsystem: "com.frame.app", category: "CursorRecorder")

/// A single recorded cursor event (position + type).
struct CursorEvent: Codable {
    let timestamp: TimeInterval       // Seconds since recording start
    let x: Double                     // Screen X coordinate
    let y: Double                     // Screen Y coordinate
    let type: EventType

    enum EventType: String, Codable {
        case move
        case leftClick
        case rightClick
        case leftRelease
        case rightRelease
        case scroll
    }
}

/// Records cursor position and click events as metadata during recording.
/// This data is used post-recording for cursor smoothing, auto-zoom, and click effects.
@MainActor
final class CursorRecorder: ObservableObject {

    // MARK: - State

    @Published private(set) var events: [CursorEvent] = []
    @Published private(set) var isRecording = false

    private var startTime: Date?
    private var pollTimer: Timer?
    private var globalClickMonitor: Any?
    private var lastPosition: CGPoint = .zero

    // MARK: - Configuration

    /// How often to sample cursor position (Hz)
    var sampleRate: Double = 60

    /// Minimum pixel distance to record a move event (filters micro-jitter)
    var moveThreshold: Double = 1.0

    // MARK: - Start / Stop

    func startRecording() {
        guard !isRecording else { return }

        events = []
        startTime = Date()
        isRecording = true
        lastPosition = NSEvent.mouseLocation

        // Poll cursor position at sampleRate
        let interval = 1.0 / sampleRate
        pollTimer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.sampleCursorPosition()
            }
        }

        // Monitor click events globally
        globalClickMonitor = NSEvent.addGlobalMonitorForEvents(
            matching: [.leftMouseDown, .leftMouseUp, .rightMouseDown, .rightMouseUp, .scrollWheel]
        ) { [weak self] event in
            Task { @MainActor [weak self] in
                self?.handleClickEvent(event)
            }
        }

        logger.info("Cursor recording started at \(self.sampleRate)Hz")
    }

    func stopRecording() -> [CursorEvent] {
        guard isRecording else { return [] }

        pollTimer?.invalidate()
        pollTimer = nil

        if let monitor = globalClickMonitor {
            NSEvent.removeMonitor(monitor)
            globalClickMonitor = nil
        }

        isRecording = false
        let recordedEvents = events

        logger.info("Cursor recording stopped: \(recordedEvents.count) events captured")
        return recordedEvents
    }

    // MARK: - Sampling

    private func sampleCursorPosition() {
        guard let startTime else { return }

        let position = NSEvent.mouseLocation
        let distance = hypot(position.x - lastPosition.x, position.y - lastPosition.y)

        // Only record if cursor moved beyond threshold
        guard distance >= moveThreshold else { return }

        let elapsed = Date().timeIntervalSince(startTime)
        let event = CursorEvent(
            timestamp: elapsed,
            x: position.x,
            y: position.y,
            type: .move
        )
        events.append(event)
        lastPosition = position
    }

    private func handleClickEvent(_ nsEvent: NSEvent) {
        guard let startTime else { return }

        let position = NSEvent.mouseLocation
        let elapsed = Date().timeIntervalSince(startTime)

        let eventType: CursorEvent.EventType
        switch nsEvent.type {
        case .leftMouseDown:  eventType = .leftClick
        case .leftMouseUp:    eventType = .leftRelease
        case .rightMouseDown: eventType = .rightClick
        case .rightMouseUp:   eventType = .rightRelease
        case .scrollWheel:    eventType = .scroll
        default: return
        }

        let event = CursorEvent(
            timestamp: elapsed,
            x: position.x,
            y: position.y,
            type: eventType
        )
        events.append(event)

        if eventType == .leftClick || eventType == .rightClick {
            logger.debug("Click at (\(position.x), \(position.y)) t=\(elapsed)")
        }
    }

    // MARK: - Persistence

    /// Save cursor events to a JSON file alongside the recording
    func saveEvents(to url: URL) throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try encoder.encode(events)
        try data.write(to: url)
        logger.info("Saved \(self.events.count) cursor events to \(url.lastPathComponent)")
    }

    /// Load cursor events from a JSON file
    static func loadEvents(from url: URL) -> [CursorEvent] {
        guard let data = try? Data(contentsOf: url) else { return [] }
        return (try? JSONDecoder().decode([CursorEvent].self, from: data)) ?? []
    }
}
