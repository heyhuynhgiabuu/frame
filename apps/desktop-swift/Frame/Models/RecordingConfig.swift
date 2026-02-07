import Foundation
import ScreenCaptureKit
import CoreGraphics

/// Configuration for a recording session.
struct RecordingConfig {
    /// What to capture
    var captureType: CaptureType = .display

    /// Selected display (nil = primary)
    var selectedDisplay: SCDisplay?

    /// Selected window (for window capture mode)
    var selectedWindow: SCWindow?

    /// Video settings
    var frameRate: Int = 30
    var showsCursor: Bool = true

    /// Audio settings (persisted via UserDefaults)
    var captureSystemAudio: Bool {
        didSet { UserDefaults.standard.set(captureSystemAudio, forKey: "captureSystemAudio") }
    }
    var captureMicrophone: Bool {
        didSet { UserDefaults.standard.set(captureMicrophone, forKey: "captureMicrophone") }
    }

    /// Quality
    var scaleFactor: CGFloat = 2.0  // Retina
    var quality: VideoQuality = .high

    init() {
        // Load persisted values, defaulting to system audio on / mic off
        let defaults = UserDefaults.standard
        if defaults.object(forKey: "captureSystemAudio") != nil {
            self.captureSystemAudio = defaults.bool(forKey: "captureSystemAudio")
        } else {
            self.captureSystemAudio = true
        }
        if defaults.object(forKey: "captureMicrophone") != nil {
            self.captureMicrophone = defaults.bool(forKey: "captureMicrophone")
        } else {
            self.captureMicrophone = false
        }
    }

    enum CaptureType: String, CaseIterable, Identifiable {
        case display = "Full Screen"
        case window = "Window"

        var id: String { rawValue }
    }

    enum VideoQuality: String, CaseIterable {
        case low
        case medium
        case high

        /// Compression quality for H.264 (0.0 - 1.0)
        var compressionQuality: Double {
            switch self {
            case .low: return 0.5
            case .medium: return 0.75
            case .high: return 1.0
            }
        }
    }

    /// Build SCStreamConfiguration from this config
    func makeStreamConfiguration(for display: SCDisplay) -> SCStreamConfiguration {
        let config = SCStreamConfiguration()

        // Resolution: account for retina
        let width = Int(CGFloat(display.width) * scaleFactor)
        let height = Int(CGFloat(display.height) * scaleFactor)

        // H.264 max is 4096x2304 — downscale if needed
        let maxWidth = 4096
        let maxHeight = 2304

        if width > maxWidth || height > maxHeight {
            let scale = min(Double(maxWidth) / Double(width), Double(maxHeight) / Double(height))
            config.width = Int(Double(width) * scale)
            config.height = Int(Double(height) * scale)
        } else {
            config.width = width
            config.height = height
        }

        config.minimumFrameInterval = CMTime(value: 1, timescale: CMTimeScale(frameRate))
        config.showsCursor = showsCursor
        config.pixelFormat = kCVPixelFormatType_32BGRA

        // Audio
        config.capturesAudio = captureSystemAudio
        config.excludesCurrentProcessAudio = true  // Don't capture our own app sounds

        // Microphone capture (macOS 15.0+)
        if #available(macOS 15.0, *) {
            config.captureMicrophone = captureMicrophone
        }

        return config
    }
}
