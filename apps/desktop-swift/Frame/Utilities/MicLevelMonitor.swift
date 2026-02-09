import AVFoundation
import Combine

/// Monitors microphone input level using AVAudioEngine.
/// Publishes a normalised 0…1 level suitable for driving a live meter.
final class MicLevelMonitor {

    /// Normalised mic level (0 = silence, 1 = loud).
    @Published private(set) var level: Float = 0

    private var audioEngine: AVAudioEngine?
    private let smoothingFactor: Float = 0.3
    private var previousLevel: Float = 0

    /// Start monitoring the given audio device (by uniqueID).
    /// Pass `nil` to use the system default mic.
    func start(deviceID: String? = nil) {
        stop()

        let engine = AVAudioEngine()

        // Select the requested device if provided
        if let deviceID,
           let device = AVCaptureDevice.DiscoverySession(
               deviceTypes: [.microphone, .external],
               mediaType: .audio,
               position: .unspecified
           ).devices.first(where: { $0.uniqueID == deviceID }) {
            setAudioDevice(device, on: engine)
        }

        let inputNode = engine.inputNode
        let format = inputNode.outputFormat(forBus: 0)

        guard format.sampleRate > 0, format.channelCount > 0 else { return }

        inputNode.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            self?.processBuffer(buffer)
        }

        do {
            try engine.start()
            audioEngine = engine
        } catch {
            print("[MicLevelMonitor] Failed to start: \(error.localizedDescription)")
        }
    }

    func stop() {
        audioEngine?.inputNode.removeTap(onBus: 0)
        audioEngine?.stop()
        audioEngine = nil
        previousLevel = 0
        DispatchQueue.main.async { [weak self] in
            self?.level = 0
        }
    }

    // MARK: - Private

    private func processBuffer(_ buffer: AVAudioPCMBuffer) {
        guard let channelData = buffer.floatChannelData else { return }

        let channelCount = Int(buffer.format.channelCount)
        let frameLength = Int(buffer.frameLength)
        guard frameLength > 0 else { return }

        // RMS across all channels
        var rms: Float = 0
        for ch in 0..<channelCount {
            let samples = channelData[ch]
            var sumOfSquares: Float = 0
            for i in 0..<frameLength {
                let s = samples[i]
                sumOfSquares += s * s
            }
            rms += sqrtf(sumOfSquares / Float(frameLength))
        }
        rms /= Float(channelCount)

        // Convert to dB, then normalise to 0…1  (−60 dB → 0, 0 dB → 1)
        let db = 20 * log10f(max(rms, 1e-6))
        let minDb: Float = -60
        let normalised = max(0, min(1, (db - minDb) / (0 - minDb)))

        // EMA smoothing
        let smoothed = smoothingFactor * normalised + (1 - smoothingFactor) * previousLevel
        previousLevel = smoothed

        DispatchQueue.main.async { [weak self] in
            self?.level = smoothed
        }
    }

    private func setAudioDevice(_ device: AVCaptureDevice, on engine: AVAudioEngine) {
        var deviceID = AudioDeviceID(0)
        var address = AudioObjectPropertyAddress(
            mSelector: kAudioHardwarePropertyDevices,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain
        )

        var size: UInt32 = 0
        AudioObjectGetPropertyDataSize(
            AudioObjectID(kAudioObjectSystemObject),
            &address,
            0, nil,
            &size
        )

        let count = Int(size) / MemoryLayout<AudioDeviceID>.size
        var devices = [AudioDeviceID](repeating: 0, count: count)
        AudioObjectGetPropertyData(
            AudioObjectID(kAudioObjectSystemObject),
            &address,
            0, nil,
            &size,
            &devices
        )

        // Match by UID
        for dev in devices {
            var uid: CFString = "" as CFString
            var uidSize = UInt32(MemoryLayout<CFString>.size)
            var uidAddress = AudioObjectPropertyAddress(
                mSelector: kAudioDevicePropertyDeviceUID,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain
            )
            AudioObjectGetPropertyData(dev, &uidAddress, 0, nil, &uidSize, &uid)
            if (uid as String) == device.uniqueID {
                deviceID = dev
                break
            }
        }

        guard deviceID != 0 else { return }

        // Set input device on the audio unit behind the engine's input node
        let audioUnit = engine.inputNode.audioUnit
        var devID = deviceID
        AudioUnitSetProperty(
            audioUnit!,
            kAudioOutputUnitProperty_CurrentDevice,
            kAudioUnitScope_Global,
            0,
            &devID,
            UInt32(MemoryLayout<AudioDeviceID>.size)
        )
    }

    deinit {
        stop()
    }
}
