import AVFoundation
import Accelerate
import OSLog

private let logger = Logger(subsystem: "com.frame.app", category: "AudioWaveformGenerator")

/// Represents audio waveform data extracted from a video file
struct AudioWaveform: Sendable {
    /// Audio amplitude samples normalized to 0.0...1.0 range
    /// One sample per ~100ms of audio
    let samples: [Float]
    
    /// Duration of the audio in seconds
    let duration: TimeInterval
    
    /// Time interval between each sample (in seconds)
    var sampleInterval: TimeInterval {
        guard samples.count > 1 else { return 0 }
        return duration / TimeInterval(samples.count)
    }
    
    /// Get amplitude at a specific time
    func amplitude(at time: TimeInterval) -> Float {
        guard !samples.isEmpty, duration > 0 else { return 0 }
        let index = Int((time / duration) * Double(samples.count))
        guard index >= 0, index < samples.count else { return 0 }
        return samples[index]
    }
    
    /// Get samples for a specific time range
    func samples(in range: ClosedRange<TimeInterval>) -> [Float] {
        guard !samples.isEmpty, duration > 0 else { return [] }
        let startIndex = max(0, Int((range.lowerBound / duration) * Double(samples.count)))
        let endIndex = min(samples.count - 1, Int((range.upperBound / duration) * Double(samples.count)))
        guard startIndex <= endIndex else { return [] }
        return Array(samples[startIndex...endIndex])
    }
}

/// Generates audio waveform data from video files
@MainActor
final class AudioWaveformGenerator: ObservableObject {
    
    @Published private(set) var isGenerating = false
    @Published private(set) var progress: Double = 0
    
    /// Generate waveform data from a video URL
    /// - Parameters:
    ///   - url: Video file URL
    ///   - samplesPerSecond: Number of samples to generate per second of audio (default: 10)
    /// - Returns: AudioWaveform if audio exists, nil if no audio or error
    func generateWaveform(from url: URL, samplesPerSecond: Int = 10) async throws -> AudioWaveform? {
        logger.info("Starting waveform generation for: \(url.lastPathComponent)")
        
        await MainActor.run {
            isGenerating = true
            progress = 0
        }
        defer {
            Task { @MainActor in
                isGenerating = false
                progress = 1
            }
        }
        
        let asset = AVURLAsset(url: url)
        
        // Check if asset has audio tracks
        let audioTracks = try await asset.loadTracks(withMediaType: .audio)
        guard !audioTracks.isEmpty else {
            logger.info("No audio tracks found in video")
            return nil
        }
        
        // Get asset duration
        let duration = try await asset.load(.duration)
        let durationSeconds = duration.seconds
        guard durationSeconds.isFinite && durationSeconds > 0 else {
            logger.warning("Invalid duration")
            return nil
        }
        
        // Configure audio reader
        guard let audioTrack = audioTracks.first,
              let reader = try? AVAssetReader(asset: asset) else {
            logger.error("Failed to create audio reader")
            return nil
        }
        
        let audioOutput = AVAssetReaderTrackOutput(
            track: audioTrack,
            outputSettings: [
                AVFormatIDKey: Int(kAudioFormatLinearPCM),
                AVLinearPCMBitDepthKey: 16,
                AVLinearPCMIsBigEndianKey: false,
                AVLinearPCMIsFloatKey: false,
                AVLinearPCMIsNonInterleaved: false,
                AVSampleRateKey: 44100,
                AVNumberOfChannelsKey: 1
            ]
        )
        
        reader.add(audioOutput)
        reader.startReading()
        
        // Calculate total samples needed
        let totalSamples = Int(durationSeconds * Double(samplesPerSecond))
        var waveformSamples: [Float] = []
        waveformSamples.reserveCapacity(totalSamples)
        
        // Process audio buffers
        let samplesPerBuffer = 44100 / samplesPerSecond // Samples to accumulate per waveform sample
        var accumulatedSamples: [Int16] = []
        accumulatedSamples.reserveCapacity(samplesPerBuffer)
        
        while let sampleBuffer = audioOutput.copyNextSampleBuffer() {
            guard let blockBuffer = CMSampleBufferGetDataBuffer(sampleBuffer) else { continue }
            
            let length = CMBlockBufferGetDataLength(blockBuffer)
            var data = Data(count: length)
            data.withUnsafeMutableBytes { rawBuffer in
                guard let baseAddress = rawBuffer.baseAddress else { return }
                CMBlockBufferCopyDataBytes(blockBuffer, atOffset: 0, dataLength: length, destination: baseAddress)
            }
            
            // Convert to Int16 samples
            data.withUnsafeBytes { rawBuffer in
                guard let samples = rawBuffer.bindMemory(to: Int16.self).baseAddress else { return }
                let sampleCount = length / 2
                
                for i in 0..<sampleCount {
                    accumulatedSamples.append(samples[i])
                    
                    // When we have enough samples, calculate RMS amplitude
                    if accumulatedSamples.count >= samplesPerBuffer {
                        let amplitude = calculateRMS(samples: accumulatedSamples)
                        waveformSamples.append(amplitude)
                        accumulatedSamples.removeAll(keepingCapacity: true)
                        
                        // Update progress
                        let currentProgress = Double(waveformSamples.count) / Double(totalSamples)
                        Task { @MainActor in
                            self.progress = min(currentProgress, 0.99)
                        }
                    }
                }
            }
            
            // Yield periodically to avoid blocking main thread
            if waveformSamples.count % 100 == 0 {
                try await Task.sleep(nanoseconds: 1_000) // 1 microsecond
            }
        }
        
        // Process any remaining samples
        if !accumulatedSamples.isEmpty {
            let amplitude = calculateRMS(samples: accumulatedSamples)
            waveformSamples.append(amplitude)
        }
        
        reader.cancelReading()
        
        logger.info("Generated \(waveformSamples.count) waveform samples for \(durationSeconds)s audio")
        
        return AudioWaveform(samples: waveformSamples, duration: durationSeconds)
    }
    
    /// Calculate Root Mean Square amplitude from audio samples
    private func calculateRMS(samples: [Int16]) -> Float {
        guard !samples.isEmpty else { return 0 }
        
        // Convert Int16 to Float and calculate squares
        var floatSamples = samples.map { Float($0) / Float(Int16.max) }
        var squaredSamples = floatSamples.map { $0 * $0 }
        
        // Calculate mean of squares
        let meanSquare = squaredSamples.reduce(0, +) / Float(squaredSamples.count)
        
        // Square root for RMS
        let rms = sqrt(meanSquare)
        
        // Normalize and apply slight compression for better visualization
        let normalized = min(rms * 2.5, 1.0) // Scale up slightly, clamp to 1.0
        
        return normalized
    }
}

// MARK: - Preview Helpers

extension AudioWaveformGenerator {
    /// Generate a test waveform for previews
    static func testWaveform() -> AudioWaveform {
        let sampleCount = 600 // 60 seconds at 10 samples/sec
        let samples = (0..<sampleCount).map { i -> Float in
            // Generate a synthetic waveform with some variation
            let t = Float(i) / Float(sampleCount)
            let base = sin(t * 10) * 0.3 + 0.3 // Base wave
            let noise = Float.random(in: 0...0.2) // Random noise
            return min(base + noise, 1.0)
        }
        return AudioWaveform(samples: samples, duration: 60)
    }
}
