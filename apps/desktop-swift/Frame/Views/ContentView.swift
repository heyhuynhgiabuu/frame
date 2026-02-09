import SwiftUI

struct ContentView: View {
    @Environment(AppState.self) private var appState

    var body: some View {
        @Bindable var appState = appState

        Group {
            switch appState.mode {
            case .recorder:
                RecordingView()
            case .editor:
                EditorView()
            }
        }
        .toolbar {
            ToolbarItems()
        }
        .animation(.easeInOut(duration: 0.25), value: appState.mode)
        .sheet(isPresented: $appState.showExportSheet) {
            ExportView()
                .environment(appState)
        }
        .sheet(isPresented: $appState.showQuickExportSettings, onDismiss: {
            appState.handleQuickExportSettingsDismissed()
        }) {
            QuickExportSettingsSheet()
                .environment(appState)
        }
        .alert(
            appState.recordingError?.title ?? "Error",
            isPresented: $appState.showErrorAlert,
            presenting: appState.recordingError
        ) { error in
            if error.showOpenSettings {
                Button("Open System Settings") {
                    appState.showErrorAlert = false
                    if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture") {
                        NSWorkspace.shared.open(url)
                    }
                }
                Button("OK", role: .cancel) {
                    appState.showErrorAlert = false
                }
            } else {
                Button("OK", role: .cancel) {
                    appState.showErrorAlert = false
                }
            }
        } message: { error in
            Text(error.message)
        }
    }
}

private struct QuickExportSettingsSheet: View {
    @Environment(AppState.self) private var appState

    var body: some View {
        @Bindable var appState = appState

        VStack(alignment: .leading, spacing: 16) {
            Text("Quick export settings")
                .font(.title3.weight(.semibold))

            Form {
                Picker("Format", selection: $appState.quickExportFormat) {
                    ForEach(ExportConfig.ExportFormat.allCases, id: \.self) { format in
                        Text(format.rawValue.uppercased())
                            .tag(format)
                    }
                }

                Picker("Quality", selection: $appState.quickExportQuality) {
                    ForEach(ExportConfig.ExportQuality.allCases, id: \.self) { quality in
                        Text(quality.rawValue.capitalized)
                            .tag(quality)
                    }
                }

                Picker("Resolution", selection: $appState.quickExportResolution) {
                    ForEach(ExportConfig.ExportResolution.allCases, id: \.self) { resolution in
                        Text(resolution.rawValue)
                            .tag(resolution)
                    }
                }

                Picker("Frame Rate", selection: $appState.quickExportFrameRate) {
                    ForEach(ExportConfig.ExportFrameRate.allCases, id: \.self) { frameRate in
                        Text(frameRate.displayName)
                            .tag(frameRate)
                    }
                }
            }

            HStack {
                Spacer()
                Button("Done") {
                    appState.showQuickExportSettings = false
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(20)
        .frame(width: 380)
    }
}

#Preview {
    ContentView()
        .environment(AppState())
        .frame(width: 1280, height: 800)
}
