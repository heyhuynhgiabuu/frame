import Foundation
import AppKit
import Carbon.HIToolbox
import OSLog

private let logger = Logger(subsystem: "com.frame.app", category: "KeystrokeRecorder")

/// A single recorded keystroke event.
struct KeystrokeEvent: Codable, Identifiable {
    let id: UUID
    let timestamp: TimeInterval   // seconds since recording start
    let keyCode: UInt16
    let characters: String        // human-readable key label
    let modifiers: ModifierSet
    let isDown: Bool              // true = key down, false = key up

    init(
        timestamp: TimeInterval,
        keyCode: UInt16,
        characters: String,
        modifiers: ModifierSet,
        isDown: Bool
    ) {
        self.id = UUID()
        self.timestamp = timestamp
        self.keyCode = keyCode
        self.characters = characters
        self.modifiers = modifiers
        self.isDown = isDown
    }

    /// Codable modifier set for keystroke events.
    struct ModifierSet: Codable, Equatable {
        var command: Bool = false
        var shift: Bool = false
        var option: Bool = false
        var control: Bool = false
        var fn: Bool = false

        init() {}

        init(flags: NSEvent.ModifierFlags) {
            self.command = flags.contains(.command)
            self.shift = flags.contains(.shift)
            self.option = flags.contains(.option)
            self.control = flags.contains(.control)
            self.fn = flags.contains(.function)
        }

        /// Returns modifier symbols in standard macOS display order.
        var symbols: [String] {
            var result: [String] = []
            if control { result.append("⌃") }
            if option { result.append("⌥") }
            if shift { result.append("⇧") }
            if command { result.append("⌘") }
            if fn { result.append("fn") }
            return result
        }

        var isEmpty: Bool {
            !command && !shift && !option && !control && !fn
        }
    }
}

/// Records keystrokes during screen recording using global event monitors.
/// Mirrors the CursorRecorder pattern: start/stop/save/load.
@MainActor
final class KeystrokeRecorder: ObservableObject {

    @Published private(set) var events: [KeystrokeEvent] = []

    private var keyDownMonitor: Any?
    private var keyUpMonitor: Any?
    private var flagsMonitor: Any?
    private var startTime: Date?

    // Track currently held modifier state to detect modifier-only taps
    private var lastModifierFlags: NSEvent.ModifierFlags = []

    // MARK: - Recording Control

    func startRecording() {
        events = []
        startTime = Date()
        lastModifierFlags = []

        // Monitor key down events globally
        keyDownMonitor = NSEvent.addGlobalMonitorForEvents(matching: .keyDown) { [weak self] event in
            Task { @MainActor [weak self] in
                self?.handleKeyEvent(event, isDown: true)
            }
        }

        // Monitor key up events globally
        keyUpMonitor = NSEvent.addGlobalMonitorForEvents(matching: .keyUp) { [weak self] event in
            Task { @MainActor [weak self] in
                self?.handleKeyEvent(event, isDown: false)
            }
        }

        // Monitor modifier flag changes (for standalone modifier presses)
        flagsMonitor = NSEvent.addGlobalMonitorForEvents(matching: .flagsChanged) { [weak self] event in
            Task { @MainActor [weak self] in
                self?.handleFlagsChanged(event)
            }
        }

        logger.info("Keystroke recording started")
    }

    /// Stops recording and returns recorded events.
    @discardableResult
    func stopRecording() -> [KeystrokeEvent] {
        if let monitor = keyDownMonitor {
            NSEvent.removeMonitor(monitor)
            keyDownMonitor = nil
        }
        if let monitor = keyUpMonitor {
            NSEvent.removeMonitor(monitor)
            keyUpMonitor = nil
        }
        if let monitor = flagsMonitor {
            NSEvent.removeMonitor(monitor)
            flagsMonitor = nil
        }

        startTime = nil
        logger.info("Keystroke recording stopped: \(self.events.count) events")
        return events
    }

    // MARK: - Event Handling

    private func handleKeyEvent(_ event: NSEvent, isDown: Bool) {
        guard let startTime else { return }

        let timestamp = Date().timeIntervalSince(startTime)
        let label = Self.keyLabel(for: event)
        let modifiers = KeystrokeEvent.ModifierSet(flags: event.modifierFlags)

        let keystroke = KeystrokeEvent(
            timestamp: timestamp,
            keyCode: event.keyCode,
            characters: label,
            modifiers: modifiers,
            isDown: isDown
        )
        events.append(keystroke)
    }

    private func handleFlagsChanged(_ event: NSEvent) {
        guard let startTime else { return }

        let current = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        let timestamp = Date().timeIntervalSince(startTime)

        // Detect which modifier toggled
        let changed = current.symmetricDifference(lastModifierFlags)
        let isDown = current.contains(changed)  // current has the flag = pressed

        let label: String
        if changed.contains(.command) {
            label = "⌘"
        } else if changed.contains(.shift) {
            label = "⇧"
        } else if changed.contains(.option) {
            label = "⌥"
        } else if changed.contains(.control) {
            label = "⌃"
        } else if changed.contains(.function) {
            label = "fn"
        } else {
            lastModifierFlags = current
            return
        }

        let keystroke = KeystrokeEvent(
            timestamp: timestamp,
            keyCode: event.keyCode,
            characters: label,
            modifiers: KeystrokeEvent.ModifierSet(flags: current),
            isDown: isDown
        )
        events.append(keystroke)
        lastModifierFlags = current
    }

    // MARK: - Persistence

    func saveEvents(to url: URL) throws {
        let data = try JSONEncoder().encode(events)
        try data.write(to: url)
        logger.info("Saved \(self.events.count) keystroke events to \(url.lastPathComponent)")
    }

    static func loadEvents(from url: URL) -> [KeystrokeEvent] {
        guard let data = try? Data(contentsOf: url) else { return [] }
        return (try? JSONDecoder().decode([KeystrokeEvent].self, from: data)) ?? []
    }

    // MARK: - Key Label Mapping

    /// Returns a human-readable label for a key event.
    static func keyLabel(for event: NSEvent) -> String {
        // Special keys first
        switch Int(event.keyCode) {
        case kVK_Return:       return "⏎"
        case kVK_Tab:          return "⇥"
        case kVK_Space:        return "Space"
        case kVK_Delete:       return "⌫"
        case kVK_ForwardDelete: return "⌦"
        case kVK_Escape:       return "⎋"
        case kVK_UpArrow:      return "↑"
        case kVK_DownArrow:    return "↓"
        case kVK_LeftArrow:    return "←"
        case kVK_RightArrow:   return "→"
        case kVK_Home:         return "↖"
        case kVK_End:          return "↘"
        case kVK_PageUp:       return "⇞"
        case kVK_PageDown:     return "⇟"
        case kVK_F1:           return "F1"
        case kVK_F2:           return "F2"
        case kVK_F3:           return "F3"
        case kVK_F4:           return "F4"
        case kVK_F5:           return "F5"
        case kVK_F6:           return "F6"
        case kVK_F7:           return "F7"
        case kVK_F8:           return "F8"
        case kVK_F9:           return "F9"
        case kVK_F10:          return "F10"
        case kVK_F11:          return "F11"
        case kVK_F12:          return "F12"
        case kVK_CapsLock:     return "⇪"
        default: break
        }

        // Use the characters ignoring modifiers for normal keys
        if let chars = event.charactersIgnoringModifiers?.uppercased(), !chars.isEmpty {
            return chars
        }

        return "?"
    }
}
