import Foundation
import AVFoundation
import CoreImage
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers
import AppKit
import OSLog

private let logger = Logger(subsystem: "com.frame.app", category: "GIFExporter")

/// Exports video frames to animated GIF format.
/// Uses ImageIO for GIF encoding with customizable frame rate, quality, and color palette.
@MainActor
final class GIFExporter {

    // MARK: - Types

    struct ExportProgress {
        let completedFrames: Int
        let totalFrames: Int
        let percentage: Double
    }

    // MARK: - Properties

    private let ciContext = CIContext(options: [
        .useSoftwareRenderer: false,
        .highQualityDownsample: true
    ])

    private var isCancelled = false

    // MARK: - Export

    /// Exports a video to GIF format.
    /// - Parameters:
    ///   - sourceURL: URL of the source video
    ///   - outputURL: Destination URL for the GIF
    ///   - config: Export configuration
    ///   - effects: Effects to apply to each frame
    ///   - progressHandler: Called periodically with progress updates
    /// - Throws: ExportError if export fails
    func export(
        sourceURL: URL,
        outputURL: URL,
        config: ExportConfig,
        effects: EffectsConfig,
        progressHandler: ((ExportProgress) -> Void)? = nil
    ) async throws {
        isCancelled = false

        // Remove existing file
        try? FileManager.default.removeItem(at: outputURL)

        // 1. Open source asset
        let asset = AVURLAsset(url: sourceURL)
        let fullDuration = try await asset.load(.duration)
        let fullDurationSeconds = CMTimeGetSeconds(fullDuration)

        guard let videoTrack = try await asset.loadTracks(withMediaType: .video).first else {
            throw ExportError.noVideoTrack
        }
        let sourceSize = try await videoTrack.load(.naturalSize)

        // Apply trim range
        let trimIn = effects.trimInTime ?? 0
        let trimOut = effects.trimOutTime ?? fullDurationSeconds
        let trimmedDuration = trimOut - trimIn

        // 2. Compute output dimensions
        let padding = effects.padding
        let canvasWidth = Int(sourceSize.width + padding * 2)
        let canvasHeight = Int(sourceSize.height + padding * 2)
        let finalSize = config.outputSize(sourceWidth: canvasWidth, sourceHeight: canvasHeight)

        // Scale down for GIF if too large (GIFs become unwieldy at high resolutions)
        let maxDimension: CGFloat = 720
        var outputSize = finalSize
        if finalSize.width > maxDimension || finalSize.height > maxDimension {
            let scale = min(maxDimension / finalSize.width, maxDimension / finalSize.height)
            outputSize = CGSize(
                width: floor(finalSize.width * scale),
                height: floor(finalSize.height * scale)
            )
        }

        logger.info("GIF Export: \(Int(sourceSize.width))x\(Int(sourceSize.height)) → \(Int(outputSize.width))x\(Int(outputSize.height))")

        // 3. Set up GIF destination
        guard let destination = CGImageDestinationCreateWithURL(
            outputURL as CFURL,
            UTType.gif.identifier as CFString,
            0,  // Frame count will be determined dynamically
            nil
        ) else {
            throw ExportError.writerFailed("Failed to create GIF destination")
        }

        // Set GIF properties
        let gifProperties: [String: Any] = [
            kCGImagePropertyGIFDictionary as String: [
                kCGImagePropertyGIFLoopCount as String: config.gifLoopCount
            ]
        ]
        CGImageDestinationSetProperties(destination, gifProperties as CFDictionary)

        // 4. Set up asset reader
        let reader = try AVAssetReader(asset: asset)
        let trimRange = CMTimeRange(
            start: CMTime(seconds: trimIn, preferredTimescale: 600),
            end: CMTime(seconds: trimOut, preferredTimescale: 600)
        )
        reader.timeRange = trimRange

        let videoOutputSettings: [String: Any] = [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA
        ]
        let videoOutput = AVAssetReaderTrackOutput(track: videoTrack, outputSettings: videoOutputSettings)
        videoOutput.alwaysCopiesSampleData = false
        reader.add(videoOutput)

        reader.startReading()

        // 5. Process frames
        let gifFrameInterval = 1.0 / Double(config.gifFPS)
        var lastFrameTime: Double = -1
        var frameCount = 0
        var processedFrames = 0
        let estimatedFrames = max(1, Int(trimmedDuration * Double(config.gifFPS)))

        while let sampleBuffer = videoOutput.copyNextSampleBuffer() {
            guard !isCancelled else {
                reader.cancelReading()
                throw CancellationError()
            }

            let presentationTime = CMSampleBufferGetPresentationTimeStamp(sampleBuffer)
            let presentationSeconds = CMTimeGetSeconds(presentationTime)

            // Skip frames to match target GIF frame rate
            if presentationSeconds - lastFrameTime < gifFrameInterval && lastFrameTime >= 0 {
                continue
            }
            lastFrameTime = presentationSeconds

            autoreleasepool {
                guard let pixelBuffer = CMSampleBufferGetImageBuffer(sampleBuffer) else { return }

                // Create CIImage and apply effects
                let sourceImage = CIImage(cvPixelBuffer: pixelBuffer)
                let compositedImage = applyEffects(
                    to: sourceImage,
                    effects: effects,
                    canvasSize: outputSize,
                    originalSize: finalSize
                )

                // Render to CGImage with color quantization
                guard let cgImage = renderCGImage(
                    from: compositedImage,
                    size: outputSize,
                    maxColors: config.gifMaxColors
                ) else { return }

                // Calculate frame duration
                let frameDuration = gifFrameInterval

                // Set frame properties
                let frameProperties: [String: Any] = [
                    kCGImagePropertyGIFDictionary as String: [
                        kCGImagePropertyGIFDelayTime as String: frameDuration
                    ]
                ]

                // Add frame to GIF
                CGImageDestinationAddImage(destination, cgImage, frameProperties as CFDictionary)

                frameCount += 1
            }

            processedFrames += 1

            // Report progress
            if processedFrames % 5 == 0 {
                let progress = ExportProgress(
                    completedFrames: frameCount,
                    totalFrames: estimatedFrames,
                    percentage: min(0.95, Double(frameCount) / Double(estimatedFrames))
                )
                await MainActor.run {
                    progressHandler?(progress)
                }
            }
        }

        // 6. Finalize GIF
        guard CGImageDestinationFinalize(destination) else {
            throw ExportError.writerFailed("Failed to finalize GIF")
        }

        reader.cancelReading()

        logger.info("GIF Export complete: \(frameCount) frames")
    }

    func cancel() {
        isCancelled = true
    }

    // MARK: - Effects Compositing

    private func applyEffects(
        to sourceImage: CIImage,
        effects: EffectsConfig,
        canvasSize: CGSize,
        originalSize: CGSize
    ) -> CIImage {
        let canvasRect = CGRect(origin: .zero, size: canvasSize)
        let padding = effects.padding * (canvasSize.width / originalSize.width)
        let sourceRect = sourceImage.extent

        // Scale source to fit canvas maintaining aspect ratio
        let scaleX = (canvasSize.width - padding * 2) / sourceRect.width
        let scaleY = (canvasSize.height - padding * 2) / sourceRect.height
        let scale = min(scaleX, scaleY)

        let scaledWidth = sourceRect.width * scale
        let scaledHeight = sourceRect.height * scale
        let xOffset = (canvasSize.width - scaledWidth) / 2
        let yOffset = (canvasSize.height - scaledHeight) / 2

        // Create background
        var background = createBackground(effects: effects, size: canvasSize)

        // Apply corner radius and shadow to source
        var processedSource = sourceImage
        if effects.cornerRadius > 0 {
            processedSource = applyCornerRadius(
                to: processedSource,
                radius: effects.cornerRadius * scale,
                size: sourceRect.size
            )
        }

        // Scale source
        let scaleTransform = CGAffineTransform(scaleX: scale, y: scale)
        processedSource = processedSource.transformed(by: scaleTransform)

        // Apply shadow
        if effects.shadowBlur > 0 && effects.shadowOpacity > 0 {
            let shadowImage = createShadow(
                for: processedSource,
                blur: effects.shadowBlur * scale,
                opacity: effects.shadowOpacity
            )
            let shadowOffset = CGAffineTransform(
                translationX: xOffset,
                y: yOffset - effects.shadowOffsetY * scale
            )
            let positionedShadow = shadowImage.transformed(by: shadowOffset)
            background = positionedShadow.composited(over: background)
        }

        // Position source on background
        let sourceOffset = CGAffineTransform(translationX: xOffset, y: yOffset)
        let positionedSource = processedSource.transformed(by: sourceOffset)

        // Composite
        let final = positionedSource.composited(over: background)

        return final.cropped(to: canvasRect)
    }

    private func createBackground(effects: EffectsConfig, size: CGSize) -> CIImage {
        let rect = CGRect(origin: .zero, size: size)

        switch effects.backgroundType {
        case .solid:
            let color = CIColor(
                red: effects.backgroundColor.red,
                green: effects.backgroundColor.green,
                blue: effects.backgroundColor.blue,
                alpha: effects.backgroundColor.alpha
            )
            return CIImage(color: color).cropped(to: rect)

        case .gradient:
            // Match ExportEngine's gradient logic using GradientPreset
            let preset = GradientPreset.allPresets.first { $0.id == effects.gradientPresetID }
            let colors = preset?.colors ?? [.blue, .purple]

            let nsColor0 = NSColor(colors.first ?? .blue)
            let nsColor1 = NSColor(colors.last ?? .purple)

            let color0 = CIColor(color: nsColor0) ?? CIColor(red: 0.2, green: 0.3, blue: 0.9)
            let color1 = CIColor(color: nsColor1) ?? CIColor(red: 0.5, green: 0.2, blue: 0.7)

            let gradient = CIFilter.linearGradient()
            gradient.point0 = CGPoint(x: 0, y: 0)
            gradient.point1 = CGPoint(x: size.width, y: size.height)
            gradient.color0 = color0
            gradient.color1 = color1

            return gradient.outputImage?.cropped(to: rect)
                ?? CIImage(color: CIColor(red: 0.1, green: 0.1, blue: 0.1)).cropped(to: rect)

        case .wallpaper, .image:
            return CIImage(color: CIColor(red: 0.08, green: 0.08, blue: 0.12)).cropped(to: rect)
        }
    }

    private func applyCornerRadius(to image: CIImage, radius: Double, size: CGSize) -> CIImage {
        let roundedRect = CIFilter.roundedRectangleGenerator()
        roundedRect.extent = CGRect(origin: .zero, size: size)
        roundedRect.radius = Float(radius)
        roundedRect.color = .white

        guard let mask = roundedRect.outputImage else { return image }

        let blended = CIFilter.blendWithMask()
        blended.inputImage = image
        blended.backgroundImage = CIImage.empty()
        blended.maskImage = mask

        return blended.outputImage ?? image
    }

    private func createShadow(
        for image: CIImage,
        blur: Double,
        opacity: Double
    ) -> CIImage {
        let colorMatrix = CIFilter.colorMatrix()
        colorMatrix.inputImage = image
        colorMatrix.rVector = CIVector(x: 0, y: 0, z: 0, w: 0)
        colorMatrix.gVector = CIVector(x: 0, y: 0, z: 0, w: 0)
        colorMatrix.bVector = CIVector(x: 0, y: 0, z: 0, w: 0)
        colorMatrix.aVector = CIVector(x: 0, y: 0, z: 0, w: CGFloat(opacity))

        guard let silhouette = colorMatrix.outputImage else { return CIImage.empty() }

        return silhouette.clampedToExtent()
            .applyingGaussianBlur(sigma: blur)
            .cropped(to: silhouette.extent.insetBy(dx: -blur * 3, dy: -blur * 3))
    }

    private func renderCGImage(from ciImage: CIImage, size: CGSize, maxColors: Int = 256) -> CGImage? {
        // Create a color space
        guard let colorSpace = CGColorSpace(name: CGColorSpace.sRGB) else { return nil }

        // Render to a CGImage directly using CIContext
        guard let cgImage = ciContext.createCGImage(ciImage, from: ciImage.extent, format: .RGBA8, colorSpace: colorSpace) else {
            return nil
        }

        // Apply color quantization for smaller GIF file sizes
        // Use indexed color space when maxColors < 256
        if maxColors < 256 {
            // Reduce color depth by rendering through a smaller color table
            let bitmapInfo = CGBitmapInfo(rawValue: CGImageAlphaInfo.noneSkipLast.rawValue)
            guard let context = CGContext(
                data: nil,
                width: Int(size.width),
                height: Int(size.height),
                bitsPerComponent: 8,
                bytesPerRow: Int(size.width) * 4,
                space: colorSpace,
                bitmapInfo: bitmapInfo.rawValue
            ) else { return cgImage }

            // Draw with lower interpolation quality to reduce colors
            context.interpolationQuality = maxColors <= 64 ? .low : .medium
            context.draw(cgImage, in: CGRect(origin: .zero, size: size))
            return context.makeImage() ?? cgImage
        }

        return cgImage
    }
}

// ExportError is defined in ExportEngine.swift and shared across the module
