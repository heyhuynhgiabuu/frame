import AVFoundation
import Combine
import OSLog

private let logger = Logger(subsystem: "com.frame.app", category: "PlaybackEngine")

/// Manages AVPlayer state for video playback in the editor.
@MainActor
final class PlaybackEngine: ObservableObject {

    // MARK: - Published State

    @Published private(set) var isPlaying = false
    @Published private(set) var currentTime: TimeInterval = 0
    @Published private(set) var duration: TimeInterval = 0
    @Published private(set) var isReady = false

    /// Normalized progress 0...1 for timeline binding
    var progress: Double {
        guard duration > 0 else { return 0 }
        return currentTime / duration
    }

    // MARK: - Player

    let player = AVPlayer()
    private var playerItem: AVPlayerItem?
    private var timeObserver: Any?
    private var statusObserver: NSKeyValueObservation?
    private var durationObserver: NSKeyValueObservation?
    private var rateObserver: NSKeyValueObservation?

    // MARK: - Lifecycle

    nonisolated deinit {
        // Clean up time observer — must access player directly since
        // deinit is nonisolated and can't call @MainActor methods
        if let timeObserver {
            player.removeTimeObserver(timeObserver)
        }
    }

    // MARK: - Load Media

    func loadVideo(url: URL) {
        logger.info("Loading video: \(url.lastPathComponent)")

        // Clean up previous
        removeObservers()
        player.pause()
        isPlaying = false
        currentTime = 0
        duration = 0
        isReady = false

        // Create new player item
        let asset = AVURLAsset(url: url)
        let item = AVPlayerItem(asset: asset)
        playerItem = item
        player.replaceCurrentItem(with: item)

        // Observe status
        statusObserver = item.observe(\.status, options: [.new]) { [weak self] item, _ in
            Task { @MainActor in
                guard let self else { return }
                switch item.status {
                case .readyToPlay:
                    self.isReady = true
                    self.duration = item.duration.seconds.isFinite ? item.duration.seconds : 0
                    logger.info("Video ready — duration: \(self.duration)s")
                case .failed:
                    logger.error("Player item failed: \(item.error?.localizedDescription ?? "unknown")")
                    self.isReady = false
                default:
                    break
                }
            }
        }

        // Observe rate (play/pause state)
        rateObserver = player.observe(\.rate, options: [.new]) { [weak self] player, _ in
            Task { @MainActor in
                self?.isPlaying = player.rate > 0
            }
        }

        // Periodic time observer — 30fps for smooth timeline
        let interval = CMTime(value: 1, timescale: 30)
        timeObserver = player.addPeriodicTimeObserver(forInterval: interval, queue: .main) { [weak self] time in
            Task { @MainActor in
                guard let self else { return }
                let seconds = time.seconds
                if seconds.isFinite {
                    self.currentTime = seconds
                }
            }
        }

        // Observe end of playback
        NotificationCenter.default.addObserver(
            forName: .AVPlayerItemDidPlayToEndTime,
            object: item,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor in
                self?.handlePlaybackEnded()
            }
        }
    }

    // MARK: - Playback Controls

    func play() {
        guard isReady else { return }
        player.play()
    }

    func pause() {
        player.pause()
    }

    func togglePlayPause() {
        if isPlaying {
            pause()
        } else {
            play()
        }
    }

    func seek(to time: TimeInterval) {
        let cmTime = CMTime(seconds: time, preferredTimescale: 600)
        player.seek(to: cmTime, toleranceBefore: .zero, toleranceAfter: .zero)
    }

    func seekToProgress(_ progress: Double) {
        let time = progress * duration
        seek(to: time)
    }

    func seekToStart() {
        seek(to: 0)
    }

    func seekToEnd() {
        seek(to: max(0, duration - 0.01))
    }

    /// Step forward/backward by a number of frames
    func stepByFrames(_ count: Int, fps: Double = 30) {
        let frameDuration = 1.0 / fps
        let newTime = max(0, min(duration, currentTime + Double(count) * frameDuration))
        seek(to: newTime)
    }

    // MARK: - Private

    private func handlePlaybackEnded() {
        isPlaying = false
        // Loop back to start
        seek(to: 0)
    }

    private func removeObservers() {
        if let timeObserver {
            player.removeTimeObserver(timeObserver)
            self.timeObserver = nil
        }
        statusObserver?.invalidate()
        statusObserver = nil
        durationObserver?.invalidate()
        durationObserver = nil
        rateObserver?.invalidate()
        rateObserver = nil
        NotificationCenter.default.removeObserver(self, name: .AVPlayerItemDidPlayToEndTime, object: playerItem)
    }
}
