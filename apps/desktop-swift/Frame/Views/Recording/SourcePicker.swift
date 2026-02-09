import SwiftUI
import ScreenCaptureKit

/// Picker for selecting what to record: display or window.
struct SourcePicker: View {
    @Binding var config: RecordingConfig
    let displays: [SCDisplay]
    let windows: [SCWindow]
    var appState: AppState?  // Optional — for webcam toggle

    var body: some View {
        HStack(spacing: 24) {
            // Capture type
            captureTypePicker

            Divider()
                .frame(height: 20)

            // Audio options
            audioOptions

            Divider()
                .frame(height: 20)

            // Webcam toggle
            webcamToggle
        }
        .font(.callout)
        .foregroundStyle(.secondary)
    }

    // MARK: - Capture Type

    private var captureTypePicker: some View {
        Menu {
            Section("Display") {
                ForEach(Array(displays.enumerated()), id: \.offset) { index, display in
                    Button(action: {
                        if let appState {
                            appState.setCaptureMode(RecorderToolbarSettings.CaptureMode.display)
                            appState.setSelectedDisplay(display)
                        } else {
                            config.captureType = .display
                            config.selectedDisplay = display
                        }
                    }) {
                        Label(
                            displays.count > 1 ? "Display \(index + 1)" : "Full Screen",
                            systemImage: "macwindow"
                        )
                    }
                }
            }

            Section("Window") {
                ForEach(Array(windows.prefix(10).enumerated()), id: \.offset) { _, window in
                    Button(action: {
                        if let appState {
                            appState.setCaptureMode(RecorderToolbarSettings.CaptureMode.window)
                            appState.setSelectedWindow(window)
                        } else {
                            config.captureType = .window
                            config.selectedWindow = window
                        }
                    }) {
                        Label(
                            window.title ?? window.owningApplication?.applicationName ?? "Unknown",
                            systemImage: "macwindow.on.rectangle"
                        )
                    }
                }
            }

            Section("Other") {
                Button(action: {
                    config.captureType = .area
                }) {
                    Label("Area", systemImage: "rectangle.dashed")
                }

                Button(action: {
                    config.captureType = .device
                }) {
                    Label("Device", systemImage: "iphone")
                }
            }
        } label: {
            HStack(spacing: 6) {
                Image(systemName: captureTypeIcon)
                    .font(.caption)
                Text(captureTypeLabel)
            }
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
    }

    private var captureTypeLabel: String {
        switch config.captureType {
        case .display:
            return "Full Screen"
        case .window:
            return config.selectedWindow?.title ?? "Select Window"
        case .area:
            return "Area"
        case .device:
            return "Device"
        }
    }

    private var captureTypeIcon: String {
        switch config.captureType {
        case .display: return "macwindow"
        case .window: return "macwindow.on.rectangle"
        case .area: return "rectangle.dashed"
        case .device: return "iphone"
        }
    }

    // MARK: - Audio Options

    private var audioOptions: some View {
        HStack(spacing: 16) {
            Toggle(isOn: $config.captureSystemAudio) {
                HStack(spacing: 6) {
                    Image(systemName: config.captureSystemAudio ? "speaker.wave.2.fill" : "speaker.slash")
                        .font(.caption)
                    Text("System Audio")
                }
            }
            .toggleStyle(.button)
            .buttonStyle(.borderless)

            Toggle(isOn: $config.captureMicrophone) {
                HStack(spacing: 6) {
                    Image(systemName: config.captureMicrophone ? "mic.fill" : "mic.slash")
                        .font(.caption)
                    Text("Mic")
                }
            }
            .toggleStyle(.button)
            .buttonStyle(.borderless)
        }
    }

    // MARK: - Webcam

    private var webcamToggle: some View {
        Button {
            appState?.toggleWebcam()
        } label: {
            HStack(spacing: 6) {
                Image(systemName: appState?.webcamEngine.isRunning == true
                    ? "camera.fill" : "camera")
                    .font(.caption)
                Text(appState?.webcamEngine.isRunning == true
                    ? "Webcam On" : "Webcam Off")
            }
            .opacity(appState?.webcamEngine.isRunning == true ? 1.0 : 0.5)
        }
        .buttonStyle(.borderless)
    }
}
