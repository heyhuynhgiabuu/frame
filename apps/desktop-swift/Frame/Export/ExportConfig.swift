import Foundation
import AVFoundation

/// Configuration for video export.
struct ExportConfig: Codable, Equatable {
    static let defaultConfig = ExportConfig()

    // MARK: - Format

    enum ExportFormat: String, Codable, CaseIterable, Identifiable {
        case mp4 = "MP4"
        case mov = "MOV"
        case gif = "GIF"

        var id: String { rawValue }

        var fileExtension: String {
            switch self {
            case .mp4: return "mp4"
            case .mov: return "mov"
            case .gif: return "gif"
            }
        }

        var icon: String {
            switch self {
            case .mp4: return "film"
            case .mov: return "film.stack"
            case .gif: return "photo.on.rectangle.angled"
            }
        }

        var description: String {
            switch self {
            case .mp4: return "H.264 video, widely compatible"
            case .mov: return "Apple ProRes, highest quality"
            case .gif: return "Animated GIF for sharing"
            }
        }
    }

    var format: ExportFormat = .mp4

    // MARK: - Quality

    enum ExportQuality: String, Codable, CaseIterable, Identifiable {
        case low = "Low"
        case medium = "Medium"
        case high = "High"
        case maximum = "Maximum"

        var id: String { rawValue }

        /// Approximate bitrate multiplier relative to base
        var bitrateMultiplier: Double {
            switch self {
            case .low: return 0.5
            case .medium: return 1.0
            case .high: return 2.0
            case .maximum: return 4.0
            }
        }

        /// Compression quality for H.264 (0.0 - 1.0)
        var compressionQuality: Float {
            switch self {
            case .low: return 0.4
            case .medium: return 0.65
            case .high: return 0.85
            case .maximum: return 1.0
            }
        }

        /// Average bitrate in bits per second for 1080p
        var baseBitrate: Int {
            switch self {
            case .low: return 2_500_000
            case .medium: return 5_000_000
            case .high: return 10_000_000
            case .maximum: return 20_000_000
            }
        }
    }

    var quality: ExportQuality = .high

    // MARK: - Resolution

    enum ExportResolution: String, Codable, CaseIterable, Identifiable {
        case original = "Original"
        case res4K = "4K"
        case res1080p = "1080p"
        case res720p = "720p"
        case res480p = "480p"

        var id: String { rawValue }

        /// Target height in pixels (nil = use source resolution)
        var targetHeight: Int? {
            switch self {
            case .original: return nil
            case .res4K: return 2160
            case .res1080p: return 1080
            case .res720p: return 720
            case .res480p: return 480
            }
        }
    }

    var resolution: ExportResolution = .original

    // MARK: - Frame Rate

    enum ExportFrameRate: Int, Codable, CaseIterable, Identifiable {
        case fps24 = 24
        case fps30 = 30
        case fps60 = 60

        var id: Int { rawValue }

        var displayName: String { "\(rawValue) FPS" }
    }

    var frameRate: ExportFrameRate = .fps30

    // MARK: - GIF Options

    var gifFPS: Int = 15                    // Lower FPS for GIF
    var gifMaxColors: Int = 256             // Color palette size
    var gifLoopCount: Int = 0               // 0 = infinite loop

    // MARK: - Helpers

    /// Compute output size given source dimensions
    func outputSize(sourceWidth: Int, sourceHeight: Int) -> CGSize {
        guard let targetHeight = resolution.targetHeight else {
            return CGSize(width: sourceWidth, height: sourceHeight)
        }

        let aspect = Double(sourceWidth) / Double(sourceHeight)
        let width = Int(Double(targetHeight) * aspect)
        // Ensure even dimensions for H.264
        let evenWidth = width % 2 == 0 ? width : width + 1
        let evenHeight = targetHeight % 2 == 0 ? targetHeight : targetHeight + 1
        return CGSize(width: evenWidth, height: evenHeight)
    }

    /// Generate a default output filename
    func defaultFilename(projectName: String) -> String {
        let timestamp = ISO8601DateFormatter().string(from: Date())
            .replacingOccurrences(of: ":", with: "-")
        return "\(projectName)_\(timestamp).\(format.fileExtension)"
    }
}
