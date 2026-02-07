import SwiftUI
import AVFoundation

/// Timeline view with transport controls, scrubbing track, trim handles, and playhead.
struct TimelineView: View {
    @ObservedObject var engine: PlaybackEngine
    @Binding var effects: EffectsConfig
    let duration: TimeInterval

    @State private var isScrubbing = false
    @State private var scrubProgress: Double = 0
    @State private var isDraggingTrimIn = false
    @State private var isDraggingTrimOut = false

    var body: some View {
        VStack(spacing: 0) {
            // Transport controls bar
            transportBar
                .padding(.horizontal, 16)
                .padding(.vertical, 8)

            Divider()

            // Timeline track area
            timelineTrack
                .padding(.horizontal, 16)
                .padding(.vertical, 12)
        }
        .background(.ultraThinMaterial)
    }

    // MARK: - Trim Helpers

    private var trimInProgress: Double {
        guard duration > 0, let t = effects.trimInTime else { return 0 }
        return t / duration
    }

    private var trimOutProgress: Double {
        guard duration > 0, let t = effects.trimOutTime else { return 1 }
        return t / duration
    }

    private var trimmedDuration: TimeInterval {
        let inTime = effects.trimInTime ?? 0
        let outTime = effects.trimOutTime ?? duration
        return max(0, outTime - inTime)
    }

    // MARK: - Transport Bar

    private var transportBar: some View {
        HStack(spacing: 0) {
            // Left: time display
            timeDisplay
                .frame(minWidth: 100, alignment: .leading)

            Spacer()

            // Center: playback controls
            playbackControls

            Spacer()

            // Right: trim info + frame step + duration
            HStack(spacing: 12) {
                // Trim indicator
                if effects.trimInTime != nil || effects.trimOutTime != nil {
                    trimBadge
                }

                frameStepControls

                durationDisplay
                    .frame(minWidth: 100, alignment: .trailing)
            }
        }
    }

    private var trimBadge: some View {
        Button(action: clearTrim) {
            HStack(spacing: 3) {
                Image(systemName: "scissors")
                    .font(.system(size: 9))
                Text(formatTime(trimmedDuration))
                    .font(.system(size: 10, weight: .medium, design: .monospaced))
            }
            .padding(.horizontal, 6)
            .padding(.vertical, 3)
            .background(
                Capsule().fill(.blue.opacity(0.15))
            )
            .foregroundStyle(.blue)
        }
        .buttonStyle(.borderless)
        .help("Click to clear trim points")
    }

    private func clearTrim() {
        effects.trimInTime = nil
        effects.trimOutTime = nil
    }

    private var playbackControls: some View {
        HStack(spacing: 16) {
            // Skip to start (or trim in)
            Button(action: { engine.seekToStart() }) {
                Image(systemName: "backward.end.fill")
                    .font(.system(size: 12))
            }
            .buttonStyle(.borderless)
            .help("Go to start")

            // Step back 1 frame
            Button(action: { engine.stepByFrames(-1) }) {
                Image(systemName: "backward.frame.fill")
                    .font(.system(size: 12))
            }
            .buttonStyle(.borderless)
            .help("Previous frame")

            // Play/Pause
            Button(action: { engine.togglePlayPause() }) {
                Image(systemName: engine.isPlaying ? "pause.fill" : "play.fill")
                    .font(.system(size: 16))
                    .frame(width: 24, height: 24)
            }
            .buttonStyle(.borderless)
            .keyboardShortcut(.space, modifiers: [])
            .help(engine.isPlaying ? "Pause" : "Play")

            // Step forward 1 frame
            Button(action: { engine.stepByFrames(1) }) {
                Image(systemName: "forward.frame.fill")
                    .font(.system(size: 12))
            }
            .buttonStyle(.borderless)
            .help("Next frame")

            // Skip to end (or trim out)
            Button(action: { engine.seekToEnd() }) {
                Image(systemName: "forward.end.fill")
                    .font(.system(size: 12))
            }
            .buttonStyle(.borderless)
            .help("Go to end")

            Divider()
                .frame(height: 16)

            // Set trim in
            Button(action: setTrimIn) {
                Image(systemName: "bracket.square.left.fill")
                    .font(.system(size: 12))
            }
            .buttonStyle(.borderless)
            .help("Set trim in point (I)")
            .keyboardShortcut("i", modifiers: [])

            // Set trim out
            Button(action: setTrimOut) {
                Image(systemName: "bracket.square.right.fill")
                    .font(.system(size: 12))
            }
            .buttonStyle(.borderless)
            .help("Set trim out point (O)")
            .keyboardShortcut("o", modifiers: [])
        }
    }

    private func setTrimIn() {
        let currentTime = engine.currentTime
        // Ensure in < out
        if let outTime = effects.trimOutTime, currentTime >= outTime {
            return
        }
        effects.trimInTime = currentTime > 0.01 ? currentTime : nil
    }

    private func setTrimOut() {
        let currentTime = engine.currentTime
        // Ensure out > in
        if let inTime = effects.trimInTime, currentTime <= inTime {
            return
        }
        effects.trimOutTime = currentTime < (duration - 0.01) ? currentTime : nil
    }

    private var frameStepControls: some View {
        HStack(spacing: 8) {
            Button(action: { engine.stepByFrames(-10) }) {
                Image(systemName: "gobackward.5")
                    .font(.system(size: 11))
            }
            .buttonStyle(.borderless)
            .help("Back 10 frames")

            Button(action: { engine.stepByFrames(10) }) {
                Image(systemName: "goforward.5")
                    .font(.system(size: 11))
            }
            .buttonStyle(.borderless)
            .help("Forward 10 frames")
        }
    }

    private var timeDisplay: some View {
        Text(formatTime(engine.currentTime))
            .font(.system(size: 12, weight: .medium, design: .monospaced))
            .foregroundStyle(.primary)
    }

    private var durationDisplay: some View {
        Text(formatTime(engine.duration))
            .font(.system(size: 12, weight: .regular, design: .monospaced))
            .foregroundStyle(.secondary)
    }

    // MARK: - Timeline Track

    private var timelineTrack: some View {
        GeometryReader { geometry in
            let width = geometry.size.width
            let height = geometry.size.height
            let displayProgress = isScrubbing ? scrubProgress : engine.progress

            ZStack(alignment: .leading) {
                // Background track
                RoundedRectangle(cornerRadius: 4)
                    .fill(.quaternary)
                    .frame(height: height)

                // Trimmed-out region (left)
                if trimInProgress > 0 {
                    Rectangle()
                        .fill(.black.opacity(0.4))
                        .frame(width: width * trimInProgress, height: height)
                        .allowsHitTesting(false)
                }

                // Trimmed-out region (right)
                if trimOutProgress < 1 {
                    Rectangle()
                        .fill(.black.opacity(0.4))
                        .frame(width: width * (1 - trimOutProgress), height: height)
                        .offset(x: width * trimOutProgress)
                        .allowsHitTesting(false)
                }

                // Active region (between trim points)
                RoundedRectangle(cornerRadius: 4)
                    .fill(
                        LinearGradient(
                            colors: [
                                .blue.opacity(0.3),
                                .purple.opacity(0.3),
                                .blue.opacity(0.3)
                            ],
                            startPoint: .leading,
                            endPoint: .trailing
                        )
                    )
                    .frame(
                        width: max(0, width * (trimOutProgress - trimInProgress)),
                        height: height
                    )
                    .offset(x: width * trimInProgress)

                // Progress fill
                RoundedRectangle(cornerRadius: 4)
                    .fill(.blue.opacity(0.15))
                    .frame(width: max(0, width * displayProgress), height: height)

                // Trim In handle
                if effects.trimInTime != nil {
                    trimHandle(at: trimInProgress * width, color: .yellow, isIn: true)
                        .gesture(
                            DragGesture(minimumDistance: 0)
                                .onChanged { value in
                                    isDraggingTrimIn = true
                                    let progress = max(0, min(trimOutProgress - 0.01, value.location.x / width))
                                    effects.trimInTime = progress * duration
                                }
                                .onEnded { _ in
                                    isDraggingTrimIn = false
                                }
                        )
                }

                // Trim Out handle
                if effects.trimOutTime != nil {
                    trimHandle(at: trimOutProgress * width, color: .yellow, isIn: false)
                        .gesture(
                            DragGesture(minimumDistance: 0)
                                .onChanged { value in
                                    isDraggingTrimOut = true
                                    let progress = max(trimInProgress + 0.01, min(1, value.location.x / width))
                                    effects.trimOutTime = progress * duration
                                }
                                .onEnded { _ in
                                    isDraggingTrimOut = false
                                }
                        )
                }

                // Playhead line
                Rectangle()
                    .fill(.white)
                    .frame(width: 2, height: height + 8)
                    .offset(x: max(0, min(width - 2, width * displayProgress - 1)))
                    .shadow(color: .black.opacity(0.3), radius: 2)

                // Playhead handle (top triangle)
                PlayheadHandle()
                    .fill(.white)
                    .frame(width: 12, height: 8)
                    .offset(
                        x: max(0, min(width - 12, width * displayProgress - 6)),
                        y: -(height / 2 + 4)
                    )
                    .shadow(color: .black.opacity(0.2), radius: 1)
            }
            .frame(height: height)
            .contentShape(Rectangle())
            .gesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { value in
                        guard !isDraggingTrimIn && !isDraggingTrimOut else { return }
                        isScrubbing = true
                        let progress = max(0, min(1, value.location.x / width))
                        scrubProgress = progress
                        engine.seekToProgress(progress)
                    }
                    .onEnded { value in
                        guard !isDraggingTrimIn && !isDraggingTrimOut else { return }
                        let progress = max(0, min(1, value.location.x / width))
                        engine.seekToProgress(progress)
                        isScrubbing = false
                    }
            )
        }
        .frame(height: 48)
    }

    // MARK: - Trim Handle

    private func trimHandle(at xOffset: CGFloat, color: Color, isIn: Bool) -> some View {
        ZStack {
            // Vertical line
            Rectangle()
                .fill(color)
                .frame(width: 2, height: 56)

            // Handle tab
            RoundedRectangle(cornerRadius: 2)
                .fill(color)
                .frame(width: 8, height: 24)
                .offset(y: 0)
        }
        .offset(x: xOffset - 1)
        .zIndex(10)
    }

    // MARK: - Helpers

    private func formatTime(_ time: TimeInterval) -> String {
        guard time.isFinite && time >= 0 else { return "00:00.00" }
        let minutes = Int(time) / 60
        let seconds = Int(time) % 60
        let centiseconds = Int((time.truncatingRemainder(dividingBy: 1)) * 100)
        return String(format: "%02d:%02d.%02d", minutes, seconds, centiseconds)
    }
}

// MARK: - Playhead Handle Shape

private struct PlayheadHandle: Shape {
    func path(in rect: CGRect) -> Path {
        var path = Path()
        path.move(to: CGPoint(x: rect.midX, y: rect.maxY))
        path.addLine(to: CGPoint(x: rect.minX + 2, y: rect.minY))
        path.addLine(to: CGPoint(x: rect.maxX - 2, y: rect.minY))
        path.closeSubpath()
        return path
    }
}
