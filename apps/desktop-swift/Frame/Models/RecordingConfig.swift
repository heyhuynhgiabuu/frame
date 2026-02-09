import Foundation
import CoreGraphics
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

    /// Area rect for area capture (in display-relative points, top-left origin)
    var areaRect: CGRect?

    /// Preferred output resolution for window capture (nil = native window size)
    var windowOutputSize: CGSize?

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
        // Load persisted values, defaulting to system audio off / mic off
        let defaults = UserDefaults.standard
        if defaults.object(forKey: "captureSystemAudio") != nil {
            self.captureSystemAudio = defaults.bool(forKey: "captureSystemAudio")
        } else {
            self.captureSystemAudio = false
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
        case area = "Area"
        case device = "Device"

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

        if captureType == .area, let area = areaRect {
            // Area capture: use sourceRect to crop and size output to area dimensions
            // Clamp sourceRect to display bounds to avoid "invalid parameter" errors
            let displayW = CGFloat(display.width)
            let displayH = CGFloat(display.height)
            let clampedX = max(0, min(area.origin.x, displayW - 1))
            let clampedY = max(0, min(area.origin.y, displayH - 1))
            let clampedW = max(2, min(area.width, displayW - clampedX))
            let clampedH = max(2, min(area.height, displayH - clampedY))
            let safeRect = CGRect(x: clampedX, y: clampedY, width: clampedW, height: clampedH)

            config.sourceRect = safeRect
            let areaWidth = Int(safeRect.width * scaleFactor)
            let areaHeight = Int(safeRect.height * scaleFactor)
            let clampedSize = RecordingConfig.clampedVideoSize(width: areaWidth, height: areaHeight)
            config.width = clampedSize.width
            config.height = clampedSize.height
        } else {
            // Full display: capture entire screen
            let baseWidth = Int(CGFloat(display.width) * scaleFactor)
            let baseHeight = Int(CGFloat(display.height) * scaleFactor)
            let clampedSize = RecordingConfig.clampedVideoSize(width: baseWidth, height: baseHeight)
            config.width = clampedSize.width
            config.height = clampedSize.height
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

    static func clampedVideoSize(width: Int, height: Int) -> (width: Int, height: Int) {
        let safeWidth = max(64, width)
        let safeHeight = max(64, height)
        let maxWidth = 4096
        let maxHeight = 2304

        if safeWidth <= maxWidth, safeHeight <= maxHeight {
            return (safeWidth, safeHeight)
        }

        let scale = min(Double(maxWidth) / Double(safeWidth), Double(maxHeight) / Double(safeHeight))
        let scaledWidth = max(64, Int(Double(safeWidth) * scale))
        let scaledHeight = max(64, Int(Double(safeHeight) * scale))
        return (scaledWidth, scaledHeight)
    }
}
