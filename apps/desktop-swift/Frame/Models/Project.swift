import Foundation
import CoreGraphics

/// Represents a Frame project (recording + edit state).
struct Project: Identifiable {
    let id: UUID
    var name: String
    var createdAt: Date
    var modifiedAt: Date

    /// Path to the raw screen recording file (.mov)
    var recordingURL: URL?

    /// Path to the separate webcam recording file (.mov), if webcam was active
    var webcamRecordingURL: URL?

    /// Path to the project directory (.frame/)
    var projectDirectoryURL: URL?

    /// Recording metadata
    var duration: TimeInterval = 0
    var resolutionWidth: Double = 0
    var resolutionHeight: Double = 0
    var frameRate: Double = 30

    /// Effects configuration
    var effects: EffectsConfig = .default

    init(name: String = "Untitled") {
        self.id = UUID()
        self.name = name
        self.createdAt = Date()
        self.modifiedAt = Date()
    }
}

/// All effect parameters for a project.
struct EffectsConfig: Codable {
    // Background
    var backgroundType: BackgroundType = .gradient
    var gradientPresetID: String? = "sunset"
    var backgroundColor: CodableColor = CodableColor(red: 0.1, green: 0.1, blue: 0.1)
    var backgroundImageURL: URL?
    var padding: Double = 32
    var cornerRadius: Double = 12
    var shadowBlur: Double = 40
    var shadowOpacity: Double = 0.5
    var shadowOffsetY: Double = 10

    // Cursor
    var cursorEnabled: Bool = true
    var cursorSmoothing: Double = 0.5       // 0 = off, 1 = max
    var cursorScale: Double = 1.0           // 0.5 - 3.0
    var cursorAutoHide: Bool = true
    var cursorAutoHideDelay: Double = 2.0   // seconds
    var cursorHighlight: Bool = true
    var clickEffectEnabled: Bool = true
    var cursorClickColor: CodableColor = CodableColor(red: 0.3, green: 0.5, blue: 1.0)

    // Zoom
    var autoZoomEnabled: Bool = true
    var zoomScale: Double = 2.0
    var zoomAnimationStyle: ZoomAnimationStyle = .mellow

    // Webcam
    var webcamEnabled: Bool = false
    var webcamPosition: WebcamPosition = .bottomLeft
    var webcamSize: Double = 0.2            // 0.1 - 0.4 (relative to video)
    var webcamShape: WebcamShape = .circle
    var webcamOffsetX: Double = 0           // Offset from corner position (-1 to 1)
    var webcamOffsetY: Double = 0           // Offset from corner position (-1 to 1)

    // Audio
    var systemAudioEnabled: Bool = true
    var microphoneEnabled: Bool = false
    var volume: Double = 1.0

    // Trim (nil = use full duration)
    var trimInTime: Double?         // seconds from start
    var trimOutTime: Double?        // seconds from start

    // Keyboard overlay
    var keystrokesEnabled: Bool = true
    var keystrokeFontSize: Double = 16
    var keystrokeDisplayDuration: Double = 1.5
    var keystrokesOnlyShortcuts: Bool = false

    static let `default` = EffectsConfig()
}

// MARK: - Supporting Types

enum BackgroundType: String, Codable, CaseIterable {
    case wallpaper
    case gradient
    case solid
    case image
}

enum ZoomAnimationStyle: String, Codable, CaseIterable {
    case slow
    case mellow
    case quick
    case rapid

    var tension: Double {
        switch self {
        case .slow: return 80
        case .mellow: return 120
        case .quick: return 200
        case .rapid: return 400
        }
    }

    var friction: Double {
        switch self {
        case .slow: return 20
        case .mellow: return 22
        case .quick: return 28
        case .rapid: return 35
        }
    }
}

enum WebcamPosition: String, Codable, CaseIterable {
    case topLeft
    case topRight
    case bottomLeft
    case bottomRight
}

enum WebcamShape: String, Codable, CaseIterable {
    case circle
    case roundedRectangle
    case rectangle
}

/// Codable color representation (since SwiftUI Color isn't directly Codable).
struct CodableColor: Codable, Equatable {
    var red: Double
    var green: Double
    var blue: Double
    var alpha: Double = 1.0
}
