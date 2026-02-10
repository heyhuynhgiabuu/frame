import AVFoundation
import OSLog

private let logger = Logger(subsystem: "com.frame.app", category: "WhisperTranscription")

// MARK: - Whisper Model Types

/// Available Whisper model sizes for transcription.
enum WhisperModel: String, Codable, CaseIterable, Identifiable {
    case base = "Base"
    case small = "Small"
    case medium = "Medium"

    var id: String { rawValue }

    /// Approximate model file size for display
    var sizeDescription: String {
        switch self {
        case .base: return "~142 MB"
        case .small: return "~466 MB"
        case .medium: return "~1.5 GB"
        }
    }

    /// Relative speed description
    var speedDescription: String {
        switch self {
        case .base: return "Fastest"
        case .small: return "Balanced"
        case .medium: return "Most accurate"
        }
    }
}

/// Supported languages for transcription.
enum TranscriptionLanguage: String, Codable, CaseIterable, Identifiable {
    case auto = "Auto-detect"
    case english = "English"
    case spanish = "Spanish"
    case french = "French"
    case german = "German"
    case italian = "Italian"
    case portuguese = "Portuguese"
    case dutch = "Dutch"
    case japanese = "Japanese"
    case korean = "Korean"
    case chinese = "Chinese"

    var id: String { rawValue }

    /// ISO 639-1 language code used by Whisper
    var languageCode: String? {
        switch self {
        case .auto: return nil
        case .english: return "en"
        case .spanish: return "es"
        case .french: return "fr"
        case .german: return "de"
        case .italian: return "it"
        case .portuguese: return "pt"
        case .dutch: return "nl"
        case .japanese: return "ja"
        case .korean: return "ko"
        case .chinese: return "zh"
        }
    }
}

// MARK: - Transcription Result

/// A single caption segment with timing.
struct CaptionSegment: Codable, Identifiable, Hashable, Sendable {
    let id: UUID
    var startTime: TimeInterval      // seconds from video start
    var endTime: TimeInterval        // seconds from video start
    var text: String

    init(id: UUID = UUID(), startTime: TimeInterval, endTime: TimeInterval, text: String) {
        self.id = id
        self.startTime = startTime
        self.endTime = endTime
        self.text = text
    }

    /// Duration of this segment
    var duration: TimeInterval {
        endTime - startTime
    }

    /// Check if this segment is active at a given time
    func isActive(at time: TimeInterval) -> Bool {
        time >= startTime && time < endTime
    }
}

/// Full transcription result.
struct TranscriptionResult: Codable, Sendable {
    var segments: [CaptionSegment]
    var detectedLanguage: String?
    var modelUsed: String

    /// Combine all segments into a plain transcript
    var fullText: String {
        segments.map(\.text).joined(separator: " ")
    }

    /// Export to SRT subtitle format
    func exportToSRT() -> String {
        var srt = ""
        for (index, segment) in segments.enumerated() {
            srt += "\(index + 1)\n"
            srt += "\(formatSRTTime(segment.startTime)) --> \(formatSRTTime(segment.endTime))\n"
            srt += "\(segment.text)\n\n"
        }
        return srt
    }

    private func formatSRTTime(_ seconds: TimeInterval) -> String {
        let hours = Int(seconds) / 3600
        let minutes = (Int(seconds) % 3600) / 60
        let secs = Int(seconds) % 60
        let millis = Int((seconds.truncatingRemainder(dividingBy: 1)) * 1000)
        return String(format: "%02d:%02d:%02d,%03d", hours, minutes, secs, millis)
    }
}

// MARK: - Transcription Engine

/// Manages Whisper-based audio transcription.
///
/// This engine extracts audio from video files and processes it through
/// a local Whisper model for transcription. All processing happens
/// on-device for privacy.
@MainActor
final class WhisperTranscriptionEngine: ObservableObject {

    @Published private(set) var isTranscribing = false
    @Published private(set) var progress: Double = 0
    @Published private(set) var currentResult: TranscriptionResult?
    @Published private(set) var errorMessage: String?

    private var currentTask: Task<TranscriptionResult, Error>?
    private var extractionTask: Task<URL, Error>?

    // MARK: - Public API

    /// Transcribe audio from a video file.
    ///
    /// - Parameters:
    ///   - videoURL: URL of the video file containing audio
    ///   - model: Whisper model size to use
    ///   - language: Target language (or auto-detect)
    ///   - prompt: Optional custom prompt for specialized vocabulary
    /// - Returns: TranscriptionResult with timed caption segments
    func transcribe(
        videoURL: URL,
        model: WhisperModel = .base,
        language: TranscriptionLanguage = .auto,
        prompt: String? = nil
    ) async throws -> TranscriptionResult {
        guard !isTranscribing else {
            throw TranscriptionError.alreadyInProgress
        }

        isTranscribing = true
        progress = 0
        errorMessage = nil

        let task = Task<TranscriptionResult, Error> {
            do {
                // Step 1: Extract audio on background thread (30% of progress)
                logger.info("Extracting audio from \(videoURL.lastPathComponent)")
                let extraction = Task.detached(priority: .userInitiated) {
                    try await WhisperTranscriptionEngine.extractAudio(from: videoURL)
                }
                await MainActor.run { self.extractionTask = extraction }
                let audioURL = try await extraction.value
                await MainActor.run { self.extractionTask = nil }

                try Task.checkCancellation()

                await MainActor.run { self.progress = 0.3 }

                // Step 2: Process audio through Whisper (60% of progress)
                logger.info("Transcribing with \(model.rawValue) model")
                let result = try await self.processWithWhisper(
                    audioURL: audioURL,
                    model: model,
                    language: language,
                    prompt: prompt
                )

                try Task.checkCancellation()

                await MainActor.run { self.progress = 0.9 }

                // Step 3: Clean up temp audio file
                try? FileManager.default.removeItem(at: audioURL)

                await MainActor.run {
                    self.progress = 1.0
                    self.currentResult = result
                }
                logger.info("Transcription complete: \(result.segments.count) segments")
                return result

            } catch {
                await MainActor.run {
                    self.errorMessage = error.localizedDescription
                }
                logger.error("Transcription failed: \(error.localizedDescription)")
                throw error
            }
        }

        currentTask = task

        defer {
            isTranscribing = false
            currentTask = nil
        }

        return try await task.value
    }

    /// Cancel an in-progress transcription.
    func cancel() {
        extractionTask?.cancel()
        extractionTask = nil
        currentTask?.cancel()
        currentTask = nil
        logger.info("Transcription cancelled")
    }

    /// Check if audio is available in a video file.
    func hasAudio(in videoURL: URL) async -> Bool {
        let asset = AVURLAsset(url: videoURL)
        do {
            let tracks = try await asset.loadTracks(withMediaType: .audio)
            return !tracks.isEmpty
        } catch {
            return false
        }
    }

    // MARK: - Audio Extraction

    /// Extract audio from video to a WAV file suitable for Whisper.
    /// Whisper expects 16kHz mono PCM audio.
    /// This is a static method so it can run on a background thread.
    nonisolated static func extractAudio(from videoURL: URL) async throws -> URL {
        let asset = AVURLAsset(url: videoURL)
        let audioTracks = try await asset.loadTracks(withMediaType: .audio)

        guard let audioTrack = audioTracks.first else {
            throw TranscriptionError.noAudioTrack
        }

        let duration = try await asset.load(.duration)
        guard duration.seconds.isFinite && duration.seconds > 0 else {
            throw TranscriptionError.invalidDuration
        }

        // Create temp file for extracted audio
        let tempDir = FileManager.default.temporaryDirectory
        let audioURL = tempDir.appendingPathComponent("frame_audio_\(UUID().uuidString).wav")

        guard let reader = try? AVAssetReader(asset: asset) else {
            throw TranscriptionError.audioExtractionFailed("Could not create asset reader")
        }

        // Whisper expects 16kHz mono 16-bit PCM
        let outputSettings: [String: Any] = [
            AVFormatIDKey: Int(kAudioFormatLinearPCM),
            AVLinearPCMBitDepthKey: 16,
            AVLinearPCMIsBigEndianKey: false,
            AVLinearPCMIsFloatKey: false,
            AVLinearPCMIsNonInterleaved: false,
            AVSampleRateKey: 16000,
            AVNumberOfChannelsKey: 1
        ]

        let audioOutput = AVAssetReaderTrackOutput(
            track: audioTrack,
            outputSettings: outputSettings
        )

        reader.add(audioOutput)
        reader.startReading()

        // Collect all audio data
        var audioData = Data()

        while let sampleBuffer = audioOutput.copyNextSampleBuffer() {
            try Task.checkCancellation()

            guard let blockBuffer = CMSampleBufferGetDataBuffer(sampleBuffer) else { continue }
            let length = CMBlockBufferGetDataLength(blockBuffer)
            var data = Data(count: length)
            data.withUnsafeMutableBytes { rawBuffer in
                guard let baseAddress = rawBuffer.baseAddress else { return }
                CMBlockBufferCopyDataBytes(blockBuffer, atOffset: 0, dataLength: length, destination: baseAddress)
            }
            audioData.append(data)
        }

        reader.cancelReading()

        // Write WAV file
        try writeWAVFile(audioData: audioData, to: audioURL, sampleRate: 16000, channels: 1, bitsPerSample: 16)

        logger.info("Extracted audio: \(audioData.count) bytes → \(audioURL.lastPathComponent)")
        return audioURL
    }

    /// Write raw PCM data as a WAV file.
    nonisolated private static func writeWAVFile(
        audioData: Data,
        to url: URL,
        sampleRate: Int,
        channels: Int,
        bitsPerSample: Int
    ) throws {
        var wavData = Data()

        let byteRate = sampleRate * channels * (bitsPerSample / 8)
        let blockAlign = channels * (bitsPerSample / 8)
        let dataSize = UInt32(audioData.count)
        let fileSize = 36 + dataSize

        // RIFF header
        wavData.append(contentsOf: "RIFF".utf8)
        wavData.append(contentsOf: withUnsafeBytes(of: fileSize.littleEndian) { Array($0) })
        wavData.append(contentsOf: "WAVE".utf8)

        // fmt chunk
        wavData.append(contentsOf: "fmt ".utf8)
        wavData.append(contentsOf: withUnsafeBytes(of: UInt32(16).littleEndian) { Array($0) })         // chunk size
        wavData.append(contentsOf: withUnsafeBytes(of: UInt16(1).littleEndian) { Array($0) })          // PCM format
        wavData.append(contentsOf: withUnsafeBytes(of: UInt16(channels).littleEndian) { Array($0) })   // channels
        wavData.append(contentsOf: withUnsafeBytes(of: UInt32(sampleRate).littleEndian) { Array($0) }) // sample rate
        wavData.append(contentsOf: withUnsafeBytes(of: UInt32(byteRate).littleEndian) { Array($0) })   // byte rate
        wavData.append(contentsOf: withUnsafeBytes(of: UInt16(blockAlign).littleEndian) { Array($0) }) // block align
        wavData.append(contentsOf: withUnsafeBytes(of: UInt16(bitsPerSample).littleEndian) { Array($0) }) // bits per sample

        // data chunk
        wavData.append(contentsOf: "data".utf8)
        wavData.append(contentsOf: withUnsafeBytes(of: dataSize.littleEndian) { Array($0) })
        wavData.append(audioData)

        try wavData.write(to: url)
    }

    // MARK: - Whisper Processing

    /// Process audio through Whisper model.
    ///
    /// Note: This is a placeholder implementation that generates timed segments
    /// from the audio. When whisper.cpp Swift bindings are integrated as an SPM
    /// dependency, replace this with actual Whisper inference.
    private func processWithWhisper(
        audioURL: URL,
        model: WhisperModel,
        language: TranscriptionLanguage,
        prompt: String?
    ) async throws -> TranscriptionResult {
        // Load audio file to determine duration
        let asset = AVURLAsset(url: audioURL)
        let duration = try await asset.load(.duration)
        let durationSeconds = duration.seconds

        guard durationSeconds.isFinite && durationSeconds > 0 else {
            throw TranscriptionError.invalidDuration
        }

        // TODO: Replace with actual whisper.cpp integration
        // When whisper.cpp SPM package is added:
        // 1. Load model from app bundle (ggml-{model}.bin)
        // 2. Read audio samples from WAV file
        // 3. Call whisper_full() with parameters
        // 4. Extract segments from whisper_full_n_segments()
        //
        // For now, create a placeholder result indicating transcription
        // requires the Whisper model to be integrated.

        logger.info("Whisper processing: model=\(model.rawValue), language=\(language.rawValue), duration=\(durationSeconds)s")

        // Simulate progress for the processing phase (30% - 90%)
        let steps = 20
        for i in 0..<steps {
            try Task.checkCancellation()
            try await Task.sleep(nanoseconds: 50_000_000) // 50ms per step = 1s total
            let processingProgress = 0.3 + (Double(i + 1) / Double(steps)) * 0.6
            await MainActor.run {
                self.progress = processingProgress
            }
        }

        // Generate placeholder segments
        // In production, these come from whisper_full_n_segments()
        let segmentDuration: TimeInterval = 3.0
        let segmentCount = max(1, Int(durationSeconds / segmentDuration))
        var segments: [CaptionSegment] = []

        for i in 0..<segmentCount {
            let startTime = Double(i) * segmentDuration
            let endTime = min(startTime + segmentDuration, durationSeconds)
            segments.append(CaptionSegment(
                startTime: startTime,
                endTime: endTime,
                text: "[Transcription requires Whisper model — segment \(i + 1)]"
            ))
        }

        return TranscriptionResult(
            segments: segments,
            detectedLanguage: language.languageCode ?? "en",
            modelUsed: model.rawValue
        )
    }
}

// MARK: - Errors

enum TranscriptionError: LocalizedError {
    case noAudioTrack
    case invalidDuration
    case alreadyInProgress
    case cancelled
    case audioExtractionFailed(String)
    case modelNotFound(String)
    case transcriptionFailed(String)

    var errorDescription: String? {
        switch self {
        case .noAudioTrack:
            return "No audio track found in the recording. Make sure microphone was enabled during recording."
        case .invalidDuration:
            return "Could not determine audio duration."
        case .alreadyInProgress:
            return "A transcription is already in progress."
        case .cancelled:
            return "Transcription was cancelled."
        case .audioExtractionFailed(let detail):
            return "Failed to extract audio: \(detail)"
        case .modelNotFound(let model):
            return "Whisper model '\(model)' not found. Please download the model first."
        case .transcriptionFailed(let detail):
            return "Transcription failed: \(detail)"
        }
    }
}
