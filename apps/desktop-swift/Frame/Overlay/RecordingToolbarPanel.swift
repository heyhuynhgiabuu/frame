import AppKit
import ScreenCaptureKit
import SwiftUI

// MARK: - RecordingToolbarPanel

/// Floating frosted-glass toolbar at the bottom-center of the screen.
/// Shows recording controls: source picker, audio toggles, webcam toggle, start/stop.
final class RecordingToolbarPanel {
    private var panel: FloatingPanel<RecordingToolbarContent>?

    @MainActor
    func show(
        appState: AppState,
        on screen: NSScreen? = nil
    ) {
        let content = RecordingToolbarContent(appState: appState)
        let panel = FloatingPanel(
            contentRect: NSRect(x: 0, y: 0, width: 520, height: 56)
        ) {
            content
        }
        // Make toolbar draggable like the webcam panel
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

        HStack(spacing: 12) {
            if appState.isRecording {
                recordingControls
            } else {
                idleControls
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .background(ToolbarBackground())
        .clipShape(RoundedRectangle(cornerRadius: 16, style: .continuous))
        .shadow(color: .black.opacity(0.25), radius: 12, x: 0, y: 4)
        // NOTE: .alert is on ContentView only — avoid duplicate alerts across
        // multiple windows/panels which causes CFRunLoop crashes on macOS.
    }

    // MARK: - Idle State

    @ViewBuilder
    private var idleControls: some View {
        // Capture mode selector
        captureModePicker

        Divider()
            .frame(height: 24)
            .opacity(0.3)

        // Audio toggles
        audioToggles

        Divider()
            .frame(height: 24)
            .opacity(0.3)

        // Webcam toggle
        webcamToggle

        Divider()
            .frame(height: 24)
            .opacity(0.3)

        // Start button
        startButton
    }

    // MARK: - Recording State

    @ViewBuilder
    private var recordingControls: some View {
        // Pulsing red dot
        Circle()
            .fill(.red)
            .frame(width: 10, height: 10)
            .modifier(ToolbarPulseAnimation())

        // Duration
        Text(formattedDuration(appState.recordingDuration))
            .font(.system(.body, design: .monospaced))
            .foregroundStyle(.white)
            .frame(minWidth: 80)

        Divider()
            .frame(height: 24)
            .opacity(0.3)

        // Stop button
        stopButton
    }

    // MARK: - Components

    @ViewBuilder
    private var captureModePicker: some View {
        Menu {
            Section("Display") {
                ForEach(appState.coordinator.screenRecorder.availableDisplays, id: \.displayID) { display in
                    Button {
                        appState.coordinator.config.captureType = .display
                        appState.coordinator.config.selectedDisplay = display
                    } label: {
                        Text("Display \(display.displayID)")
                    }
                }
            }
            Section("Window") {
                ForEach(
                    Array(appState.coordinator.screenRecorder.availableWindows.prefix(10)),
                    id: \.windowID
                ) { window in
                    Button {
                        appState.coordinator.config.captureType = .window
                        appState.coordinator.config.selectedWindow = window
                    } label: {
                        Text(window.title ?? "Window \(window.windowID)")
                    }
                }
            }
        } label: {
            HStack(spacing: 4) {
                Image(systemName: appState.coordinator.config.captureType == .display
                    ? "display"
                    : "macwindow")
                Image(systemName: "chevron.down")
                    .font(.caption2)
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
    }

    @ViewBuilder
    private var audioToggles: some View {
        // System audio
        toolbarToggle(
            icon: appState.captureSystemAudio
                ? "speaker.wave.2.fill" : "speaker.slash.fill",
            isOn: appState.captureSystemAudio,
            tooltip: "System Audio"
        ) {
            appState.captureSystemAudio.toggle()
        }

        // Microphone
        toolbarToggle(
            icon: appState.captureMicrophone
                ? "mic.fill" : "mic.slash.fill",
            isOn: appState.captureMicrophone,
            tooltip: "Microphone"
        ) {
            appState.captureMicrophone.toggle()
        }
    }

    @ViewBuilder
    private var webcamToggle: some View {
        toolbarToggle(
            icon: appState.isWebcamRunning
                ? "video.fill" : "video.slash.fill",
            isOn: appState.isWebcamRunning,
            tooltip: "Webcam"
        ) {
            appState.toggleWebcam()
        }
    }

    @ViewBuilder
    private var startButton: some View {
        Button {
            appState.startRecording()
        } label: {
            HStack(spacing: 6) {
                Circle()
                    .fill(.red)
                    .frame(width: 8, height: 8)
                Text("Record")
                    .font(.system(.callout, weight: .semibold))
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .background(.red.opacity(0.8), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        }
        .buttonStyle(.plain)
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
                    .font(.system(.callout, weight: .semibold))
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .background(.white.opacity(0.2), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        }
        .buttonStyle(.plain)
    }

    // MARK: - Helpers

    @ViewBuilder
    private func toolbarToggle(
        icon: String,
        isOn: Bool,
        tooltip: String,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.system(size: 14))
                .foregroundStyle(isOn ? .white : .white.opacity(0.5))
                .frame(width: 28, height: 28)
                .background(
                    isOn ? .white.opacity(0.15) : .clear,
                    in: RoundedRectangle(cornerRadius: 6, style: .continuous)
                )
        }
        .buttonStyle(.plain)
        .help(tooltip)
    }

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
        view.layer?.cornerRadius = 16
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
