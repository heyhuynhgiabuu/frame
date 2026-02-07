import SwiftUI
import ScreenCaptureKit

struct RecordingView: View {
    @Environment(AppState.self) private var appState

    var body: some View {
        ZStack {
            // Background with vibrancy
            VisualEffectBackground(material: .hudWindow, blendingMode: .behindWindow)

            VStack(spacing: 0) {
                Spacer()

                // Preview area or recording indicator
                if appState.isRecording {
                    recordingIndicator
                } else {
                    previewArea
                }

                Spacer()

                // Duration display (when recording)
                if appState.isRecording {
                    durationDisplay
                        .padding(.bottom, 12)
                }

                // Record button
                recordButton
                    .padding(.bottom, 24)

                // Source options — wired to coordinator
                SourcePicker(
                    config: Binding(
                        get: { appState.coordinator.config },
                        set: { appState.coordinator.config = $0 }
                    ),
                    displays: appState.availableDisplays,
                    windows: appState.availableWindows,
                    appState: appState
                )
                .padding(.bottom, 32)
                .disabled(appState.isRecording)
            }
        }
        .task {
            // Refresh available sources when view appears
            await appState.refreshSources()
        }
        // NOTE: .alert is on ContentView only — avoid duplicate alerts across
        // multiple windows/panels which causes CFRunLoop crashes on macOS.
    }

    // MARK: - Preview Area (idle state)

    private var previewArea: some View {
        VStack(spacing: 16) {
            // Permission warning banner
            if appState.coordinator.screenRecorder.permissionDenied {
                permissionBanner
            }

            RoundedRectangle(cornerRadius: 12)
                .fill(.quaternary)
                .overlay {
                    VStack(spacing: 16) {
                        Image(systemName: "film.stack")
                            .font(.system(size: 48))
                            .foregroundStyle(.secondary)

                        Text("Click Record to start capturing")
                            .font(.title3)
                            .foregroundStyle(.secondary)
                    }
                }
                .frame(maxWidth: 640, maxHeight: 400)
        }
        .padding(.horizontal, 48)
    }

    // MARK: - Permission Banner

    private var permissionBanner: some View {
        HStack(spacing: 12) {
            Image(systemName: "exclamationmark.triangle.fill")
                .foregroundStyle(.yellow)
                .font(.title3)

            VStack(alignment: .leading, spacing: 2) {
                Text("Screen Recording Permission Required")
                    .font(.callout.weight(.semibold))
                Text("Enable in System Settings, then restart Frame.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer()

            Button("Open Settings") {
                openScreenRecordingSettings()
            }
            .controlSize(.small)

            Button("Retry") {
                Task {
                    await appState.refreshSources()
                }
            }
            .controlSize(.small)
        }
        .padding(12)
        .background {
            RoundedRectangle(cornerRadius: 8)
                .fill(.yellow.opacity(0.12))
                .strokeBorder(.yellow.opacity(0.3), lineWidth: 1)
        }
        .frame(maxWidth: 640)
    }

    // MARK: - Recording Indicator (active state)

    private var recordingIndicator: some View {
        RoundedRectangle(cornerRadius: 12)
            .fill(.quaternary)
            .overlay {
                VStack(spacing: 16) {
                    // Pulsing red dot
                    Circle()
                        .fill(.red)
                        .frame(width: 16, height: 16)
                        .shadow(color: .red.opacity(0.5), radius: 8)
                        .modifier(PulseAnimation())

                    Text("Recording in progress…")
                        .font(.title3)
                        .foregroundStyle(.primary)

                    Text("Your screen is being captured")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }
            .frame(maxWidth: 640, maxHeight: 400)
            .padding(.horizontal, 48)
    }

    // MARK: - Duration Display

    private var durationDisplay: some View {
        Text(formattedDuration(appState.recordingDuration))
            .font(.system(size: 28, weight: .medium, design: .monospaced))
            .foregroundStyle(.primary)
            .contentTransition(.numericText())
            .animation(.easeInOut(duration: 0.1), value: appState.recordingDuration)
    }

    // MARK: - Record Button

    private var recordButton: some View {
        Button(action: {
            if appState.isRecording {
                appState.stopRecording()
            } else {
                appState.startRecording()
            }
        }) {
            HStack(spacing: 8) {
                Circle()
                    .fill(appState.isRecording ? .white : .red)
                    .frame(width: 10, height: 10)

                Text(appState.isRecording ? "Stop Recording" : "Start Recording")
                    .fontWeight(.semibold)
            }
            .padding(.horizontal, 24)
            .padding(.vertical, 10)
        }
        .buttonStyle(.borderedProminent)
        .tint(appState.isRecording ? .red : .blue)
        .controlSize(.large)
    }

    // MARK: - Helpers

    private func formattedDuration(_ duration: TimeInterval) -> String {
        let hours = Int(duration) / 3600
        let minutes = (Int(duration) % 3600) / 60
        let seconds = Int(duration) % 60
        let centiseconds = Int((duration.truncatingRemainder(dividingBy: 1)) * 100)

        if hours > 0 {
            return String(format: "%d:%02d:%02d.%02d", hours, minutes, seconds, centiseconds)
        }
        return String(format: "%02d:%02d.%02d", minutes, seconds, centiseconds)
    }

    /// Opens macOS System Settings to the Screen Recording privacy pane.
    private func openScreenRecordingSettings() {
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture") {
            NSWorkspace.shared.open(url)
        }
    }
}

// MARK: - Pulse Animation

private struct PulseAnimation: ViewModifier {
    @State private var isPulsing = false

    func body(content: Content) -> some View {
        content
            .opacity(isPulsing ? 0.4 : 1.0)
            .scaleEffect(isPulsing ? 0.85 : 1.0)
            .animation(
                .easeInOut(duration: 0.8)
                .repeatForever(autoreverses: true),
                value: isPulsing
            )
            .onAppear { isPulsing = true }
    }
}

// MARK: - Visual Effect Background (AppKit Bridge)

struct VisualEffectBackground: NSViewRepresentable {
    let material: NSVisualEffectView.Material
    let blendingMode: NSVisualEffectView.BlendingMode

    func makeNSView(context: Context) -> NSVisualEffectView {
        let view = NSVisualEffectView()
        view.material = material
        view.blendingMode = blendingMode
        view.state = .active
        return view
    }

    func updateNSView(_ nsView: NSVisualEffectView, context: Context) {
        nsView.material = material
        nsView.blendingMode = blendingMode
    }
}

#Preview {
    RecordingView()
        .environment(AppState())
        .frame(width: 1280, height: 800)
}
