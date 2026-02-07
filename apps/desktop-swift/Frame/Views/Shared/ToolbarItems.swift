import SwiftUI

struct ToolbarItems: ToolbarContent {
    @Environment(AppState.self) private var appState

    var body: some ToolbarContent {
        // Leading: App icon + title
        ToolbarItem(placement: .navigation) {
            HStack(spacing: 8) {
                Image(systemName: "record.circle")
                    .font(.title2)
                    .foregroundStyle(.blue)

                Text("Frame")
                    .font(.headline)
            }
        }

        // Center: Mode toggle
        ToolbarItem(placement: .principal) {
            modePicker
        }

        // Trailing: Export button
        ToolbarItemGroup(placement: .primaryAction) {
            if appState.mode == .editor {
                Button(action: { appState.showExportSheet = true }) {
                    Label("Export", systemImage: "square.and.arrow.up")
                }
                .help("Export video (⇧⌘E)")
            }

            if appState.mode == .recorder {
                recordToolbarButton
            }
        }
    }

    // MARK: - Mode Picker

    private var modePicker: some View {
        Picker("Mode", selection: Binding(
            get: { appState.mode },
            set: { newMode in
                switch newMode {
                case .recorder:
                    appState.switchToRecorder()
                case .editor:
                    appState.switchToEditor()
                }
            }
        )) {
            Text("Record").tag(AppState.Mode.recorder)
            Text("Edit").tag(AppState.Mode.editor)
        }
        .pickerStyle(.segmented)
        .fixedSize()
        .disabled(appState.isRecording) // Can't switch modes while recording
    }

    // MARK: - Record Button

    private var recordToolbarButton: some View {
        Button(action: {
            if appState.isRecording {
                appState.stopRecording()
            } else {
                appState.startRecording()
            }
        }) {
            HStack(spacing: 4) {
                Circle()
                    .fill(appState.isRecording ? .white : .red)
                    .frame(width: 8, height: 8)
                Text(appState.isRecording ? "Stop" : "Record")
            }
        }
        .buttonStyle(.borderedProminent)
        .tint(appState.isRecording ? .red : .blue)
    }
}
