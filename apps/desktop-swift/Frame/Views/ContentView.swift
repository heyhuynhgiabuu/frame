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

#Preview {
    ContentView()
        .environment(AppState())
        .frame(width: 1280, height: 800)
}
