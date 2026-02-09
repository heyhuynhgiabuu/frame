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
            contentRect: NSRect(x: 0, y: 0, width: 860, height: 64)
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
        .fixedSize()
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
        closeButton

        toolbarDivider

        // Section 1: Capture mode buttons
        captureSourceButtons

        if appState.recorderToolbarSettings.captureMode == RecorderToolbarSettings.CaptureMode.display,
           appState.hasUserExplicitlySelectedCaptureModeForCard {
            displaySelector
        }

        if appState.recorderToolbarSettings.captureMode == RecorderToolbarSettings.CaptureMode.window,
           appState.hasUserExplicitlySelectedCaptureModeForCard {
            windowSelector
        }

        toolbarDivider

        // Section 2: Camera menu
        cameraSelector

        toolbarDivider

        // Section 3: Microphone + System Audio
        VStack(spacing: 3) {
            microphoneSelector

            // Live mic level bar — only below mic button
            if appState.recorderToolbarSettings.recordMicrophone {
                MicLevelBar(level: CGFloat(appState.micLevel))
                    .frame(height: 3)
                    .padding(.horizontal, 8)
            }
        }

        systemAudioMenu

        toolbarDivider

        settingsMenu
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

        pauseButton

        resetButton

        deleteButton

        Spacer()

        // Stop button
        stopButton
    }

    // MARK: - Capture Source Buttons (Display / Window / Area / Device)

    @ViewBuilder
    private var captureSourceButtons: some View {
        HStack(spacing: 2) {
            sourceButton(
                icon: "display",
                label: "Display",
                isSelected: appState.hasUserExplicitlySelectedCaptureModeForCard &&
                    appState.recorderToolbarSettings.captureMode == RecorderToolbarSettings.CaptureMode.display
            ) {
                appState.setCaptureMode(RecorderToolbarSettings.CaptureMode.display)
            }

            sourceButton(
                icon: "macwindow",
                label: "Window",
                isSelected: appState.hasUserExplicitlySelectedCaptureModeForCard &&
                    appState.recorderToolbarSettings.captureMode == RecorderToolbarSettings.CaptureMode.window,
                onHover: { isHovering in
                    guard isHovering, !appState.isRecording else { return }
                    guard appState.recorderToolbarSettings.captureMode != RecorderToolbarSettings.CaptureMode.window ||
                        !appState.hasUserExplicitlySelectedCaptureModeForCard else {
                        return
                    }

                    appState.setCaptureMode(RecorderToolbarSettings.CaptureMode.window)
                }
            ) {
                appState.setCaptureMode(RecorderToolbarSettings.CaptureMode.window)
            }

            sourceButton(
                icon: "rectangle.dashed",
                label: "Area",
                isSelected: appState.hasUserExplicitlySelectedCaptureModeForCard &&
                    appState.recorderToolbarSettings.captureMode == RecorderToolbarSettings.CaptureMode.area
            ) {
                appState.setCaptureMode(RecorderToolbarSettings.CaptureMode.area)
            }

            sourceButton(
                icon: "iphone",
                label: "Device",
                isSelected: appState.hasUserExplicitlySelectedCaptureModeForCard &&
                    appState.recorderToolbarSettings.captureMode == RecorderToolbarSettings.CaptureMode.device
            ) {
                appState.setCaptureMode(RecorderToolbarSettings.CaptureMode.device)
            }
        }
        .padding(2)
    }

    @ViewBuilder
    private var displaySelector: some View {
        Menu {
            if appState.availableDisplays.isEmpty {
                Text("No displays available")
            } else {
                ForEach(appState.availableDisplays, id: \.displayID) { display in
                    Button {
                        appState.setSelectedDisplay(display)
                    } label: {
                        dropdownLabel(
                            title: appState.displayName(for: display),
                            checked: appState.isSelectedDisplay(display)
                        )
                    }
                }
            }
        } label: {
            toolbarMenuLabel(
                icon: "display",
                title: appState.selectedDisplayNameForToolbar,
                isActive: appState.isSelectedDisplayActive
            )
        }
        .menuStyle(.borderlessButton)
        .buttonStyle(.plain)
        .help("Select display to record")
    }

    @ViewBuilder
    private var windowSelector: some View {
        Menu {
            if appState.availableWindows.isEmpty {
                Text("No windows available")
            } else {
                ForEach(appState.availableWindows.prefix(20), id: \.windowID) { window in
                    Button {
                        appState.setSelectedWindow(window)
                    } label: {
                        dropdownLabel(
                            title: appState.windowName(for: window),
                            checked: appState.isSelectedWindow(window)
                        )
                    }
                }
            }
        } label: {
            toolbarMenuLabel(
                icon: "macwindow",
                title: appState.selectedWindowNameForToolbar,
                isActive: appState.isSelectedWindowActive
            )
        }
        .menuStyle(.borderlessButton)
        .buttonStyle(.plain)
        .help("Select window to record")
    }

    @ViewBuilder
    private var closeButton: some View {
        Button {
            appState.hideRecorderOverlays()
        } label: {
            Image(systemName: "xmark")
                .font(.system(size: 14, weight: .semibold))
                .foregroundStyle(.white)
                .frame(width: 34, height: 34)
                .background(.white.opacity(0.1), in: Circle())
        }
        .buttonStyle(.plain)
        .help("Hide toolbar")
    }

    // MARK: - Camera Selector

    @ViewBuilder
    private var cameraSelector: some View {
        Menu {
            ForEach(appState.availableCameraDevices, id: \.uniqueID) { device in
                Button {
                    appState.setCameraDevice(device.uniqueID)
                    if !appState.isWebcamRunning {
                        appState.toggleWebcam()
                    }
                } label: {
                    dropdownLabel(
                        title: cameraDeviceLabel(device.localizedName),
                        checked: appState.recorderToolbarSettings.cameraDeviceID == device.uniqueID
                    )
                }
            }

            Divider()

            Menu("Max camera resolution") {
                ForEach(RecorderToolbarSettings.CameraResolution.allCases) { resolution in
                    Toggle(
                        resolution.label,
                        isOn: Binding(
                            get: { appState.recorderToolbarSettings.cameraResolution == resolution },
                            set: { if $0 { appState.setCameraResolution(resolution) } }
                        )
                    )
                }
            }

            Divider()

            Toggle(
                isOn: Binding(
                    get: { appState.recorderToolbarSettings.hideCameraPreview },
                    set: { appState.setHideCameraPreview($0) }
                )
            ) {
                Text("Hide camera preview")
            }

            Toggle(
                isOn: Binding(
                    get: { !appState.recorderToolbarSettings.recordCamera },
                    set: { appState.setRecordCamera(!$0) }
                )
            ) {
                Text("Don't record camera")
            }
        } label: {
            toolbarMenuLabel(
                icon: appState.recorderToolbarSettings.recordCamera ? "video" : "video.slash",
                title: appState.recorderToolbarSettings.recordCamera ? appState.selectedCameraName : "No Camera",
                isActive: appState.recorderToolbarSettings.recordCamera
            )
        }
        .menuStyle(.borderlessButton)
        .buttonStyle(.plain)
        .help("Camera options")
    }

    // MARK: - Microphone Selector

    @ViewBuilder
    private var microphoneSelector: some View {
        Menu {
            ForEach(appState.availableMicrophoneDevices) { device in
                Button {
                    appState.setMicrophoneDevice(device.id)
                    appState.setRecordMicrophone(true)
                } label: {
                    dropdownLabel(
                        title: device.name,
                        checked: appState.recorderToolbarSettings.microphoneDeviceID == device.id
                    )
                }
            }

            Divider()

            Toggle(
                isOn: Binding(
                    get: { appState.recorderToolbarSettings.reduceNoiseAndNormalizeVolume },
                    set: { appState.setReduceNoiseAndNormalize($0) }
                )
            ) {
                Text("Reduce noise and normalize volume")
            }

            Toggle(
                isOn: Binding(
                    get: { appState.recorderToolbarSettings.disableAutoGainControl },
                    set: { appState.setDisableAutoGainControl($0) }
                )
            ) {
                Text("Disable auto gain control")
            }

            Toggle(
                isOn: Binding(
                    get: { !appState.recorderToolbarSettings.recordMicrophone },
                    set: { appState.setRecordMicrophone(!$0) }
                )
            ) {
                Text("Don't record microphone")
            }
        } label: {
            toolbarMenuLabel(
                icon: appState.recorderToolbarSettings.recordMicrophone ? "mic" : "mic.slash",
                title: appState.selectedMicrophoneName,
                isActive: appState.recorderToolbarSettings.recordMicrophone
            )
        }
        .menuStyle(.borderlessButton)
        .buttonStyle(.plain)
        .help("Microphone options")
    }

    // MARK: - System Audio Menu

    @ViewBuilder
    private var systemAudioMenu: some View {
        Menu {
            Button {
                appState.setRecordSystemAudio(true)
            } label: {
                dropdownLabel(title: "Record system audio", checked: appState.recorderToolbarSettings.recordSystemAudio)
            }

            Button {
                appState.setRecordSystemAudio(false)
            } label: {
                dropdownLabel(title: "No system audio", checked: !appState.recorderToolbarSettings.recordSystemAudio)
            }
        } label: {
            toolbarMenuLabel(
                icon: appState.recorderToolbarSettings.recordSystemAudio ? "speaker.wave.2.fill" : "speaker.slash.fill",
                title: appState.recorderToolbarSettings.recordSystemAudio ? "System audio" : "No system audio",
                isActive: appState.recorderToolbarSettings.recordSystemAudio
            )
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
        .buttonStyle(.plain)
        .help("System audio options")
    }

    @ViewBuilder
    private var settingsMenu: some View {
        Menu {
            Toggle(
                isOn: Binding(
                    get: { appState.recorderToolbarSettings.hideDesktopIcons },
                    set: { appState.recorderToolbarSettings.hideDesktopIcons = $0 }
                )
            ) {
                Text("Hide desktop icons in recorded video")
            }

            Toggle(
                isOn: Binding(
                    get: { appState.recorderToolbarSettings.hideDockIconWhileRecording },
                    set: { appState.recorderToolbarSettings.hideDockIconWhileRecording = $0 }
                )
            ) {
                Text("Hide Frame dock icon while recording")
            }

            Toggle(
                isOn: Binding(
                    get: { appState.recorderToolbarSettings.highlightRecordedArea },
                    set: { appState.recorderToolbarSettings.highlightRecordedArea = $0 }
                )
            ) {
                Text("Highlight recorded area during recording")
            }

            Toggle(
                isOn: Binding(
                    get: { appState.recorderToolbarSettings.openQuickShareWidgetAfterRecording },
                    set: { appState.recorderToolbarSettings.openQuickShareWidgetAfterRecording = $0 }
                )
            ) {
                Text("Open quick share widget after recording")
            }

            Toggle(
                isOn: Binding(
                    get: { appState.recorderToolbarSettings.showSpeakerNotes },
                    set: { appState.recorderToolbarSettings.showSpeakerNotes = $0 }
                )
            ) {
                Text("Show speaker notes")
            }

            Divider()

            Menu("Recording countdown") {
                ForEach(RecorderToolbarSettings.Countdown.allCases) { countdown in
                    Button {
                        appState.recorderToolbarSettings.recordingCountdown = countdown
                    } label: {
                        dropdownLabel(
                            title: countdown.label,
                            checked: appState.recorderToolbarSettings.recordingCountdown == countdown
                        )
                    }
                }
            }

            Menu("Advanced") {
                Menu("Recording engine") {
                    ForEach(RecorderToolbarSettings.RecordingEngineMode.allCases) { engine in
                        Button {
                            appState.recorderToolbarSettings.recordingEngineMode = engine
                        } label: {
                            dropdownLabel(
                                title: engine.label,
                                checked: appState.recorderToolbarSettings.recordingEngineMode == engine
                            )
                        }
                    }
                }
            }

            Divider()

            Button("Settings...") {
                appState.showMainWindow()
                appState.showSettings = true
            }
        } label: {
            Image(systemName: "gearshape")
                .font(.system(size: 14, weight: .semibold))
                .foregroundStyle(.white.opacity(0.85))
                .frame(width: 30, height: 30)
                .background(.white.opacity(0.08), in: RoundedRectangle(cornerRadius: 7, style: .continuous))
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
        .buttonStyle(.plain)
        .help("Recording settings")
    }

    @ViewBuilder
    private func toolbarMenuLabel(icon: String, title: String, isActive: Bool) -> some View {
        HStack(spacing: 6) {
            Image(systemName: icon)
                .font(.system(size: 12, weight: .medium))
            Text(title)
                .font(.system(size: 12, weight: .medium))
                .lineLimit(1)
            Image(systemName: "chevron.down")
                .font(.system(size: 9, weight: .semibold))
                .foregroundStyle(.white.opacity(0.6))
        }
        .foregroundStyle(isActive ? .white : .white.opacity(0.55))
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(
            isActive ? .white.opacity(0.1) : .clear,
            in: RoundedRectangle(cornerRadius: 7, style: .continuous)
        )
    }

    private func cameraDeviceLabel(_ name: String) -> String {
        if name.contains("FaceTime") {
            return "\(name) (default)"
        }
        if name.contains("iPhone") {
            return name
        }
        return name
    }

    // MARK: - Start / Stop Buttons

    @ViewBuilder
    private var startButton: some View {
        RecordingStartSplitButton(appState: appState)
    }

    @ViewBuilder
    private func dropdownLabel(title: String, checked: Bool) -> some View {
        HStack(spacing: 8) {
            if checked {
                Image(systemName: "checkmark")
                    .frame(width: 10)
            } else {
                Spacer()
                    .frame(width: 10)
            }
            Text(title)
        }
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
        .help("Stop recording")
    }

    @ViewBuilder
    private var pauseButton: some View {
        Button {
            appState.togglePause()
        } label: {
            Image(systemName: appState.isPaused ? "play.fill" : "pause.fill")
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(.white)
                .frame(width: 30, height: 30)
                .background(.white.opacity(0.12), in: RoundedRectangle(cornerRadius: 7, style: .continuous))
        }
        .buttonStyle(.plain)
        .help(appState.isPaused ? "Resume recording" : "Pause recording")
    }

    @ViewBuilder
    private var resetButton: some View {
        Button {
            appState.resetRecording()
        } label: {
            Image(systemName: "arrow.counterclockwise")
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(.white)
                .frame(width: 30, height: 30)
                .background(.white.opacity(0.12), in: RoundedRectangle(cornerRadius: 7, style: .continuous))
        }
        .buttonStyle(.plain)
        .help("Reset recording")
    }

    @ViewBuilder
    private var deleteButton: some View {
        Button {
            appState.deleteRecording()
        } label: {
            Image(systemName: "trash")
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(.white)
                .frame(width: 30, height: 30)
                .background(.red.opacity(0.25), in: RoundedRectangle(cornerRadius: 7, style: .continuous))
        }
        .buttonStyle(.plain)
        .help("Delete recording")
    }

    // MARK: - Reusable Components

    @ViewBuilder
    private func sourceButton(
        icon: String,
        label: String,
        isSelected: Bool,
        onHover: ((Bool) -> Void)? = nil,
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
        .onHover { isHovering in
            onHover?(isHovering)
        }
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

struct RecordingStartSplitButton: View {
    var appState: AppState

    var body: some View {
        @Bindable var appState = appState
        let awaitingDisplaySelection = appState.recorderToolbarSettings.captureMode == .display &&
            appState.isAwaitingDisplayClickSelection
        let awaitingWindowSelection = appState.recorderToolbarSettings.captureMode == .window &&
            appState.isAwaitingWindowClickSelection

        let idleLabel: String = if awaitingDisplaySelection {
            "Select display first"
        } else if awaitingWindowSelection {
            "Select window first"
        } else {
            "Start recording"
        }

        HStack(spacing: 0) {
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
                    Text(
                        appState.isStartingRecording
                            ? "Starting..."
                            : idleLabel
                    )
                        .font(.system(size: 13, weight: .semibold))
                }
                .foregroundStyle(.white)
                .padding(.horizontal, 16)
                .padding(.vertical, 8)
            }

            Divider()
                .frame(height: 18)
                .overlay(Color.white.opacity(0.25))

            Menu {
                Toggle(
                    isOn: Binding(
                        get: { appState.postCreateProject },
                        set: { appState.postCreateProject = $0 }
                    )
                ) {
                    Text("Create project")
                }

                Button {
                    appState.postExportMode = appState.postExportMode == .clipboard ? .none : .clipboard
                } label: {
                    menuLabel(title: "Export and copy to clipboard", checked: appState.postExportMode == .clipboard)
                }

                Button {
                    appState.postExportMode = appState.postExportMode == .shareableLink ? .none : .shareableLink
                } label: {
                    menuLabel(title: "Export and create shareable link", checked: appState.postExportMode == .shareableLink)
                }

                Button {
                    appState.postExportMode = appState.postExportMode == .saveToFile ? .none : .saveToFile
                } label: {
                    menuLabel(title: "Export and save to file", checked: appState.postExportMode == .saveToFile)
                }

                Divider()

                Toggle(
                    isOn: Binding(
                        get: { appState.postAutoCreateZooms },
                        set: { appState.postAutoCreateZooms = $0 }
                    )
                ) {
                    Text("Automatically create zooms")
                }

                Divider()

                Button("Quick export settings...") {
                    appState.openQuickExportSettings()
                }
            } label: {
                Image(systemName: "chevron.down")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(.white.opacity(0.9))
                    .frame(width: 28, height: 32)
                    .contentShape(Rectangle())
            }
            .menuStyle(.borderlessButton)
            .buttonStyle(.plain)
        }
        .background(
            appState.isStartingRecording ? .red.opacity(0.5) : .red,
            in: RoundedRectangle(cornerRadius: 8, style: .continuous)
        )
        .buttonStyle(.plain)
        .disabled(!appState.canStartRecordingFromCard)
    }

    @ViewBuilder
    private func menuLabel(title: String, checked: Bool) -> some View {
        HStack(spacing: 8) {
            if checked {
                Image(systemName: "checkmark")
                    .frame(width: 10)
            } else {
                Spacer()
                    .frame(width: 10)
            }
            Text(title)
        }
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
