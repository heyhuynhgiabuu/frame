import AppKit
import ScreenCaptureKit
import SwiftUI

// MARK: - RecordingToolbarPanel

/// Floating frosted-glass toolbar at the bottom-center of the screen.
/// Layout inspired by Screen Studio: source picker | camera | mic | system audio | record.
final class RecordingToolbarPanel {
    private var panel: FloatingPanel<RecordingToolbarContent>?

    @MainActor
    func show(
        appState: AppState,
        on screen: NSScreen? = nil
    ) {
        let content = RecordingToolbarContent(appState: appState)
        let panel = FloatingPanel(
            contentRect: NSRect(x: 0, y: 0, width: 720, height: 64)
        ) {
            content
        }
        // Make toolbar draggable
        panel.isMovableByWindowBackground = true

        panel.positionAtBottomCenter(of: screen ?? NSScreen.main ?? NSScreen.screens[0])
        panel.show()
        self.panel = panel
    }

    func dismiss() {
        panel?.dismiss()
        panel = nil
    }

    /// Returns the NSWindow number for SCContentFilter exclusion.
    var windowNumber: Int? {
        panel?.windowNumber
    }

    var nsWindow: NSWindow? {
        panel
    }
}

// MARK: - Toolbar SwiftUI Content

struct RecordingToolbarContent: View {
    var appState: AppState

    var body: some View {
        @Bindable var appState = appState

        VStack(spacing: 0) {
            // Permission banner (shown when screen recording is denied)
            if appState.screenRecordingPermissionDenied {
                permissionBanner
            }

            HStack(spacing: 0) {
                if appState.isRecording {
                    recordingControls
                } else {
                    idleControls
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
        }
        .background(ToolbarBackground())
        .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .strokeBorder(.white.opacity(0.08), lineWidth: 1)
        )
        .shadow(color: .black.opacity(0.3), radius: 16, x: 0, y: 6)
    }

    // MARK: - Permission Banner

    @ViewBuilder
    private var permissionBanner: some View {
        HStack(spacing: 8) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(.yellow)

            Text("Screen recording permission required")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(.white.opacity(0.9))

            Spacer()

            Button("Open Settings") {
                if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture") {
                    NSWorkspace.shared.open(url)
                }
            }
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(.white)
            .padding(.horizontal, 10)
            .padding(.vertical, 4)
            .background(.white.opacity(0.15), in: RoundedRectangle(cornerRadius: 6, style: .continuous))
            .buttonStyle(.plain)

            Button {
                Task {
                    await appState.refreshSources()
                }
            } label: {
                Image(systemName: "arrow.clockwise")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(.white.opacity(0.7))
            }
            .buttonStyle(.plain)
            .help("Re-check permission")
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(.red.opacity(0.15))

        Divider()
            .overlay(Color.white.opacity(0.06))
    }

    // MARK: - Idle State

    @ViewBuilder
    private var idleControls: some View {
        // Section 1: Capture source buttons
        captureSourceButtons

        toolbarDivider

        // Section 2: Camera selector
        cameraSelector

        toolbarDivider

        // Section 3: Microphone selector
        microphoneSelector

        toolbarDivider

        // Section 4: System audio toggle
        systemAudioToggle

        Spacer()
            .frame(width: 12)

        // Section 5: Record button
        startButton
    }

    // MARK: - Recording State

    @ViewBuilder
    private var recordingControls: some View {
        // Pulsing red dot + duration
        HStack(spacing: 8) {
            Circle()
                .fill(.red)
                .frame(width: 10, height: 10)
                .modifier(ToolbarPulseAnimation())

            Text(formattedDuration(appState.recordingDuration))
                .font(.system(.body, design: .monospaced))
                .foregroundStyle(.white)
                .frame(minWidth: 80)
        }
        .padding(.horizontal, 8)

        toolbarDivider

        Spacer()

        // Stop button
        stopButton
    }

    // MARK: - Capture Source Buttons (Display / Window)

    @ViewBuilder
    private var captureSourceButtons: some View {
        HStack(spacing: 2) {
            sourceButton(
                icon: "display",
                label: "Display",
                isSelected: appState.coordinator.config.captureType == .display
            ) {
                appState.coordinator.config.captureType = .display
            }

            sourceButton(
                icon: "macwindow",
                label: "Window",
                isSelected: appState.coordinator.config.captureType == .window
            ) {
                appState.coordinator.config.captureType = .window
            }
        }
        .padding(2)
    }

    // MARK: - Camera Selector

    @ViewBuilder
    private var cameraSelector: some View {
        Button {
            appState.toggleWebcam()
        } label: {
            HStack(spacing: 6) {
                Image(systemName: appState.isWebcamRunning ? "video.fill" : "video.slash.fill")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(appState.isWebcamRunning ? .white : .white.opacity(0.45))

                Text(appState.isWebcamRunning ? "FaceTime HD" : "No Camera")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(appState.isWebcamRunning ? .white : .white.opacity(0.45))
                    .lineLimit(1)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(
                appState.isWebcamRunning ? .white.opacity(0.1) : .clear,
                in: RoundedRectangle(cornerRadius: 7, style: .continuous)
            )
        }
        .buttonStyle(.plain)
        .help("Toggle webcam")
    }

    // MARK: - Microphone Selector

    @ViewBuilder
    private var microphoneSelector: some View {
        Button {
            appState.captureMicrophone.toggle()
        } label: {
            HStack(spacing: 6) {
                Image(systemName: appState.captureMicrophone ? "mic.fill" : "mic.slash.fill")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(appState.captureMicrophone ? .white : .white.opacity(0.45))

                Text(appState.captureMicrophone ? "MacBook Pro" : "No Mic")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(appState.captureMicrophone ? .white : .white.opacity(0.45))
                    .lineLimit(1)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(
                appState.captureMicrophone ? .white.opacity(0.1) : .clear,
                in: RoundedRectangle(cornerRadius: 7, style: .continuous)
            )
        }
        .buttonStyle(.plain)
        .help("Toggle microphone")
    }

    // MARK: - System Audio Toggle

    @ViewBuilder
    private var systemAudioToggle: some View {
        Button {
            appState.captureSystemAudio.toggle()
        } label: {
            HStack(spacing: 6) {
                Image(systemName: appState.captureSystemAudio
                    ? "speaker.wave.2.fill" : "speaker.slash.fill")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(appState.captureSystemAudio ? .white : .white.opacity(0.45))

                Text(appState.captureSystemAudio ? "System Audio" : "No Audio")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(appState.captureSystemAudio ? .white : .white.opacity(0.45))
                    .lineLimit(1)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(
                appState.captureSystemAudio ? .white.opacity(0.1) : .clear,
                in: RoundedRectangle(cornerRadius: 7, style: .continuous)
            )
        }
        .buttonStyle(.plain)
        .help("Toggle system audio capture")
    }

    // MARK: - Start / Stop Buttons

    @ViewBuilder
    private var startButton: some View {
        Button {
            appState.startRecording()
        } label: {
            HStack(spacing: 6) {
                if appState.isStartingRecording {
                    ProgressView()
                        .controlSize(.small)
                        .tint(.white)
                } else {
                    Circle()
                        .fill(.white)
                        .frame(width: 8, height: 8)
                }
                Text(appState.isStartingRecording ? "Starting…" : "Record")
                    .font(.system(size: 13, weight: .semibold))
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .background(
                appState.isStartingRecording ? .red.opacity(0.5) : .red,
                in: RoundedRectangle(cornerRadius: 8, style: .continuous)
            )
        }
        .buttonStyle(.plain)
        .disabled(appState.isStartingRecording || appState.screenRecordingPermissionDenied)
    }

    @ViewBuilder
    private var stopButton: some View {
        Button {
            appState.stopRecording()
        } label: {
            HStack(spacing: 6) {
                RoundedRectangle(cornerRadius: 2, style: .continuous)
                    .fill(.white)
                    .frame(width: 10, height: 10)
                Text("Stop")
                    .font(.system(size: 13, weight: .semibold))
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
            .background(.white.opacity(0.15), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        }
        .buttonStyle(.plain)
    }

    // MARK: - Reusable Components

    @ViewBuilder
    private func sourceButton(
        icon: String,
        label: String,
        isSelected: Bool,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            VStack(spacing: 3) {
                Image(systemName: icon)
                    .font(.system(size: 14, weight: .medium))
                Text(label)
                    .font(.system(size: 10, weight: .medium))
            }
            .foregroundStyle(isSelected ? .white : .white.opacity(0.45))
            .frame(width: 60, height: 40)
            .background(
                isSelected ? .white.opacity(0.15) : .clear,
                in: RoundedRectangle(cornerRadius: 8, style: .continuous)
            )
        }
        .buttonStyle(.plain)
    }

    private var toolbarDivider: some View {
        Divider()
            .frame(height: 28)
            .overlay(Color.white.opacity(0.12))
            .padding(.horizontal, 8)
    }

    // MARK: - Helpers

    private func formattedDuration(_ duration: TimeInterval) -> String {
        let totalSeconds = Int(duration)
        let hours = totalSeconds / 3600
        let minutes = (totalSeconds % 3600) / 60
        let seconds = totalSeconds % 60
        let centiseconds = Int((duration - Double(totalSeconds)) * 100)

        if hours > 0 {
            return String(format: "%d:%02d:%02d.%02d", hours, minutes, seconds, centiseconds)
        }
        return String(format: "%02d:%02d.%02d", minutes, seconds, centiseconds)
    }
}

// MARK: - Toolbar Background (Frosted Glass)

struct ToolbarBackground: NSViewRepresentable {
    func makeNSView(context: Context) -> NSVisualEffectView {
        let view = NSVisualEffectView()
        view.material = .hudWindow
        view.blendingMode = .behindWindow
        view.state = .active
        view.wantsLayer = true
        view.layer?.cornerRadius = 14
        return view
    }

    func updateNSView(_ nsView: NSVisualEffectView, context: Context) {}
}

// MARK: - Pulse Animation

struct ToolbarPulseAnimation: ViewModifier {
    @State private var isPulsing = false

    func body(content: Content) -> some View {
        content
            .opacity(isPulsing ? 0.4 : 1.0)
            .scaleEffect(isPulsing ? 0.85 : 1.0)
            .animation(
                .easeInOut(duration: 0.8).repeatForever(autoreverses: true),
                value: isPulsing
            )
            .onAppear { isPulsing = true }
    }
}
