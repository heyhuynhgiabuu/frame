import AVFoundation
import CoreImage
import CoreImage.CIFilterBuiltins
import AppKit
import OSLog

private let logger = Logger(subsystem: "com.frame.app", category: "ExportEngine")

/// Renders effects onto a recorded video and exports to a new file.
@MainActor
final class ExportEngine: ObservableObject {

    // MARK: - Published State

    @Published private(set) var isExporting = false
    @Published private(set) var progress: Double = 0      // 0...1
    @Published private(set) var currentPhase: ExportPhase = .idle
    @Published private(set) var exportError: String?

    enum ExportPhase: String {
        case idle = "Ready"
        case preparing = "Preparing…"
        case rendering = "Rendering frames…"
        case encoding = "Encoding…"
        case finalizing = "Finalizing…"
        case complete = "Export complete"
        case failed = "Export failed"
    }

    // MARK: - Private

    private var exportTask: Task<Void, Never>?
    private let ciContext = CIContext(options: [
        .useSoftwareRenderer: false,     // Use GPU
        .highQualityDownsample: true
    ])
    private let gifExporter = GIFExporter()

    // MARK: - Cancel

    func cancel() {
        exportTask?.cancel()
        gifExporter.cancel()
        exportTask = nil
        isExporting = false
        progress = 0
        currentPhase = .idle
    }

    // MARK: - Export

    func export(
        project: Project,
        config: ExportConfig,
        outputURL: URL
    ) {
        guard !isExporting else {
            logger.warning("Export already in progress")
            return
        }
        guard let sourceURL = project.recordingURL else {
            exportError = "No recording file found"
            currentPhase = .failed
            return
        }

        isExporting = true
        progress = 0
        currentPhase = .preparing
        exportError = nil

        exportTask = Task {
            do {
                // Branch based on export format
                if config.format == .gif {
                    try await performGIFExport(
                        sourceURL: sourceURL,
                        outputURL: outputURL,
                        config: config,
                        effects: project.effects
                    )
                } else {
                    try await performVideoExport(
                        sourceURL: sourceURL,
                        outputURL: outputURL,
                        config: config,
                        effects: project.effects
                    )
                }
                self.currentPhase = .complete
                self.progress = 1.0
                logger.info("Export complete: \(outputURL.lastPathComponent)")
            } catch is CancellationError {
                self.currentPhase = .idle
                logger.info("Export cancelled")
            } catch {
                self.exportError = error.localizedDescription
                self.currentPhase = .failed
                logger.error("Export failed: \(error.localizedDescription)")
            }
            self.isExporting = false
        }
    }

    // MARK: - GIF Export

    private func performGIFExport(
        sourceURL: URL,
        outputURL: URL,
        config: ExportConfig,
        effects: EffectsConfig
    ) async throws {
        await MainActor.run {
            self.currentPhase = .rendering
        }

        try await gifExporter.export(
            sourceURL: sourceURL,
            outputURL: outputURL,
            config: config,
            effects: effects
        ) { [weak self] (progress: GIFExporter.ExportProgress) in
            Task { @MainActor in
                self?.progress = progress.percentage
            }
        }

        await MainActor.run {
            self.currentPhase = .finalizing
            self.progress = 0.95
        }
    }

    // MARK: - Core Pipeline

    private func performVideoExport(
        sourceURL: URL,
        outputURL: URL,
        config: ExportConfig,
        effects: EffectsConfig
    ) async throws {
        // 1. Open source asset
        let asset = AVURLAsset(url: sourceURL)
        let fullDuration = try await asset.load(.duration)
        let fullDurationSeconds = CMTimeGetSeconds(fullDuration)

        guard let videoTrack = try await asset.loadTracks(withMediaType: .video).first else {
            throw ExportError.noVideoTrack
        }
        let sourceSize = try await videoTrack.load(.naturalSize)
        let audioTracks = try await asset.loadTracks(withMediaType: .audio)

        // Apply trim range
        let trimIn = effects.trimInTime ?? 0
        let trimOut = effects.trimOutTime ?? fullDurationSeconds
        let trimRange = CMTimeRange(
            start: CMTime(seconds: trimIn, preferredTimescale: 600),
            end: CMTime(seconds: trimOut, preferredTimescale: 600)
        )
        let trimmedDuration = trimOut - trimIn

        // 2. Compute output dimensions (video + padding + background)
        let padding = effects.padding
        let canvasWidth = Int(sourceSize.width + padding * 2)
        let canvasHeight = Int(sourceSize.height + padding * 2)

        // Apply resolution scaling
        let finalSize = config.outputSize(sourceWidth: canvasWidth, sourceHeight: canvasHeight)

        logger.info("Export: \(Int(sourceSize.width))x\(Int(sourceSize.height)) → \(Int(finalSize.width))x\(Int(finalSize.height))")

        // 3. Set up reader
        let reader = try AVAssetReader(asset: asset)
        reader.timeRange = trimRange

        let videoOutputSettings: [String: Any] = [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA
        ]
        let videoOutput = AVAssetReaderTrackOutput(track: videoTrack, outputSettings: videoOutputSettings)
        videoOutput.alwaysCopiesSampleData = false
        reader.add(videoOutput)

        var audioOutput: AVAssetReaderTrackOutput?
        if let audioTrack = audioTracks.first, config.format != .gif {
            let aOutput = AVAssetReaderTrackOutput(track: audioTrack, outputSettings: [
                AVFormatIDKey: kAudioFormatLinearPCM,
                AVLinearPCMBitDepthKey: 16,
                AVLinearPCMIsFloatKey: false,
                AVLinearPCMIsBigEndianKey: false
            ])
            reader.add(aOutput)
            audioOutput = aOutput
        }

        // 4. Set up writer
        // Remove existing file
        try? FileManager.default.removeItem(at: outputURL)

        let fileType: AVFileType = config.format == .mov ? .mov : .mp4
        let writer = try AVAssetWriter(outputURL: outputURL, fileType: fileType)

        let videoSettings = makeVideoWriterSettings(
            size: finalSize,
            config: config
        )
        let videoInput = AVAssetWriterInput(
            mediaType: .video,
            outputSettings: videoSettings
        )
        videoInput.expectsMediaDataInRealTime = false

        let pixelBufferAdaptor = AVAssetWriterInputPixelBufferAdaptor(
            assetWriterInput: videoInput,
            sourcePixelBufferAttributes: [
                kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
                kCVPixelBufferWidthKey as String: Int(finalSize.width),
                kCVPixelBufferHeightKey as String: Int(finalSize.height)
            ]
        )
        writer.add(videoInput)

        var audioInput: AVAssetWriterInput?
        if audioOutput != nil && config.format != .gif {
            let aInput = AVAssetWriterInput(
                mediaType: .audio,
                outputSettings: [
                    AVFormatIDKey: kAudioFormatMPEG4AAC,
                    AVSampleRateKey: 44100,
                    AVNumberOfChannelsKey: 2,
                    AVEncoderBitRateKey: 128_000
                ]
            )
            aInput.expectsMediaDataInRealTime = false
            writer.add(aInput)
            audioInput = aInput
        }

        // 5. Start reading + writing
        reader.startReading()
        writer.startWriting()
        writer.startSession(atSourceTime: .zero)

        await MainActor.run {
            self.currentPhase = .rendering
        }

        // 6. Process video frames
        var frameCount = 0
        let estimatedFrames = max(1, Int(trimmedDuration * Double(config.frameRate.rawValue)))

        while let sampleBuffer = videoOutput.copyNextSampleBuffer() {
            try Task.checkCancellation()

            autoreleasepool {
                let presentationTime = CMSampleBufferGetPresentationTimeStamp(sampleBuffer)

                // Get source pixel buffer
                guard let pixelBuffer = CMSampleBufferGetImageBuffer(sampleBuffer) else { return }

                // Create CIImage from source frame
                let sourceImage = CIImage(cvPixelBuffer: pixelBuffer)

                // Apply effects compositing
                let compositedImage = applyEffects(
                    to: sourceImage,
                    effects: effects,
                    canvasSize: finalSize
                )

                // Render to output pixel buffer
                if let outputBuffer = createPixelBuffer(size: finalSize, pool: pixelBufferAdaptor.pixelBufferPool) {
                    ciContext.render(compositedImage, to: outputBuffer)

                    while !videoInput.isReadyForMoreMediaData {
                        Thread.sleep(forTimeInterval: 0.01)
                    }
                    pixelBufferAdaptor.append(outputBuffer, withPresentationTime: presentationTime)
                }

                frameCount += 1
            }

            // Update progress on main thread periodically
            if frameCount % 5 == 0 {
                let p = min(0.9, Double(frameCount) / Double(estimatedFrames))
                await MainActor.run {
                    self.progress = p
                }
            }
        }

        videoInput.markAsFinished()

        // 7. Process audio
        if let audioOutput, let audioInput {
            await MainActor.run {
                self.currentPhase = .encoding
            }

            while let sampleBuffer = audioOutput.copyNextSampleBuffer() {
                try Task.checkCancellation()
                while !audioInput.isReadyForMoreMediaData {
                    Thread.sleep(forTimeInterval: 0.01)
                }
                audioInput.append(sampleBuffer)
            }
            audioInput.markAsFinished()
        }

        // 8. Finalize
        await MainActor.run {
            self.currentPhase = .finalizing
            self.progress = 0.95
        }

        await writer.finishWriting()

        if writer.status == .failed {
            throw ExportError.writerFailed(writer.error?.localizedDescription ?? "Unknown error")
        }

        reader.cancelReading()

        logger.info("Export rendered \(frameCount) frames")
    }

    // MARK: - Effects Compositing

    /// Composites the video frame onto a background with padding, rounded corners, and shadow.
    private func applyEffects(
        to sourceImage: CIImage,
        effects: EffectsConfig,
        canvasSize: CGSize
    ) -> CIImage {
        let canvasRect = CGRect(origin: .zero, size: canvasSize)
        let padding = effects.padding
        let sourceRect = sourceImage.extent

        // 1. Create background
        var background = createBackground(effects: effects, size: canvasSize)

        // 2. Apply corner radius to source
        var processedSource = sourceImage
        if effects.cornerRadius > 0 {
            processedSource = applyCornerRadius(
                to: processedSource,
                radius: effects.cornerRadius,
                size: sourceRect.size
            )
        }

        // 3. Apply shadow
        if effects.shadowBlur > 0 && effects.shadowOpacity > 0 {
            let shadowImage = createShadow(
                for: processedSource,
                blur: effects.shadowBlur,
                opacity: effects.shadowOpacity,
                offsetY: effects.shadowOffsetY
            )
            // Position shadow
            let shadowOffset = CGAffineTransform(
                translationX: padding,
                y: padding - effects.shadowOffsetY
            )
            let positionedShadow = shadowImage.transformed(by: shadowOffset)
            background = positionedShadow.composited(over: background)
        }

        // 4. Position source on background with padding
        let sourceOffset = CGAffineTransform(translationX: padding, y: padding)
        let positionedSource = processedSource.transformed(by: sourceOffset)

        // 5. Composite source over background
        let final = positionedSource.composited(over: background)

        return final.cropped(to: canvasRect)
    }

    /// Creates a solid or gradient background.
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
            let preset = GradientPreset.allPresets.first { $0.id == effects.gradientPresetID }
            let colors = preset?.colors ?? [.blue, .purple]

            // Use a two-color linear gradient via CIFilter
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
            // Fallback to dark background for unsupported types
            return CIImage(color: CIColor(red: 0.08, green: 0.08, blue: 0.12)).cropped(to: rect)
        }
    }

    /// Applies rounded corners using a mask.
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

    /// Creates a shadow image from the source shape.
    private func createShadow(
        for image: CIImage,
        blur: Double,
        opacity: Double,
        offsetY: Double
    ) -> CIImage {
        // Create black silhouette
        let colorMatrix = CIFilter.colorMatrix()
        colorMatrix.inputImage = image
        colorMatrix.rVector = CIVector(x: 0, y: 0, z: 0, w: 0)
        colorMatrix.gVector = CIVector(x: 0, y: 0, z: 0, w: 0)
        colorMatrix.bVector = CIVector(x: 0, y: 0, z: 0, w: 0)
        colorMatrix.aVector = CIVector(x: 0, y: 0, z: 0, w: CGFloat(opacity))

        guard let silhouette = colorMatrix.outputImage else { return CIImage.empty() }

        // Apply blur
        let blurred = silhouette.clampedToExtent()
            .applyingGaussianBlur(sigma: blur)
            .cropped(to: silhouette.extent.insetBy(dx: -blur * 3, dy: -blur * 3))

        return blurred
    }

    // MARK: - Video Writer Settings

    private func makeVideoWriterSettings(size: CGSize, config: ExportConfig) -> [String: Any] {
        var settings: [String: Any] = [
            AVVideoWidthKey: Int(size.width),
            AVVideoHeightKey: Int(size.height),
        ]

        if config.format == .mov {
            settings[AVVideoCodecKey] = AVVideoCodecType.proRes422
        } else {
            settings[AVVideoCodecKey] = AVVideoCodecType.h264
            settings[AVVideoCompressionPropertiesKey] = [
                AVVideoAverageBitRateKey: config.quality.baseBitrate,
                AVVideoProfileLevelKey: AVVideoProfileLevelH264HighAutoLevel,
                AVVideoMaxKeyFrameIntervalKey: config.frameRate.rawValue * 2,
            ] as [String: Any]
        }

        return settings
    }

    // MARK: - Pixel Buffer

    private func createPixelBuffer(size: CGSize, pool: CVPixelBufferPool?) -> CVPixelBuffer? {
        var pixelBuffer: CVPixelBuffer?

        if let pool {
            let status = CVPixelBufferPoolCreatePixelBuffer(nil, pool, &pixelBuffer)
            if status == kCVReturnSuccess {
                return pixelBuffer
            }
        }

        // Fallback: create without pool
        let attrs: [String: Any] = [
            kCVPixelBufferCGImageCompatibilityKey as String: true,
            kCVPixelBufferCGBitmapContextCompatibilityKey as String: true,
            kCVPixelBufferIOSurfacePropertiesKey as String: [:] as [String: Any]
        ]

        CVPixelBufferCreate(
            nil,
            Int(size.width),
            Int(size.height),
            kCVPixelFormatType_32BGRA,
            attrs as CFDictionary,
            &pixelBuffer
        )

        return pixelBuffer
    }
}

// MARK: - Export Errors

enum ExportError: LocalizedError {
    case noVideoTrack
    case writerFailed(String)
    case readerFailed(String)

    var errorDescription: String? {
        switch self {
        case .noVideoTrack:
            return "No video track found in the recording"
        case .writerFailed(let detail):
            return "Video writer failed: \(detail)"
        case .readerFailed(let detail):
            return "Video reader failed: \(detail)"
        }
    }
}
