import SwiftUI

/// The main editor view — video preview + timeline + inspector.
struct EditorView: View {
    @Environment(AppState.self) private var appState

    var body: some View {
        HSplitView {
            // Left: Video preview + Timeline
            VStack(spacing: 0) {
                // Video preview area
                videoPreview
                    .frame(maxWidth: .infinity, maxHeight: .infinity)

                Divider()

                // Timeline
                if var project = appState.currentProject {
                    let effectsBinding = Binding<EffectsConfig>(
                        get: { project.effects },
                        set: { newEffects in
                            project.effects = newEffects
                            appState.currentProject = project
                        }
                    )
                    TimelineView(
                        engine: appState.playbackEngine,
                        effects: effectsBinding,
                        duration: project.duration
                    )
                    .frame(height: 120)
                } else {
                    TimelineView(
                        engine: appState.playbackEngine,
                        effects: .constant(EffectsConfig()),
                        duration: 0
                    )
                    .frame(height: 120)
                }
            }

            // Right: Inspector panel
            inspectorPanel
                .frame(width: 280)
        }
        .background {
            VisualEffectBackground(material: .hudWindow, blendingMode: .behindWindow)
        }
    }

    // MARK: - Video Preview

    private var videoPreview: some View {
        ZStack {
            Color.black

            if let project = appState.currentProject, project.recordingURL != nil {
                // Live preview canvas with effects
                PreviewCanvas(
                    player: appState.playbackEngine.player,
                    effects: project.effects,
                    isReady: appState.playbackEngine.isReady,
                    loadError: appState.playbackEngine.loadError,
                    cursorEvents: appState.cursorEvents,
                    currentTime: appState.playbackEngine.currentTime,
                    videoSize: CGSize(
                        width: CGFloat(project.resolutionWidth),
                        height: CGFloat(project.resolutionHeight)
                    ),
                    webcamImage: appState.webcamImage,
                    webcamPlayer: appState.webcamPlayer,
                    zoomState: appState.zoomEngine.currentZoom,
                    keystrokeEvents: appState.keystrokeEvents
                )
                .onTapGesture {
                    appState.playbackEngine.togglePlayPause()
                }
            } else {
                // Empty state
                VStack(spacing: 12) {
                    Image(systemName: "play.rectangle")
                        .font(.system(size: 40))
                        .foregroundStyle(.tertiary)
                    Text("No recording loaded")
                        .font(.headline)
                        .foregroundStyle(.tertiary)
                    Text("Record something first, then come back to edit")
                        .font(.caption)
                        .foregroundStyle(.quaternary)
                }
            }
        }
    }

    // MARK: - Inspector Panel

    private var inspectorPanel: some View {
        VStack(spacing: 0) {
            // Tab bar
            inspectorTabBar
                .padding(.horizontal, 8)
                .padding(.vertical, 8)

            Divider()

            // Tab content
            ScrollView {
                inspectorContent
                    .padding(16)
            }
        }
        .background(.ultraThinMaterial)
    }

    private var inspectorTabBar: some View {
        HStack(spacing: 2) {
            ForEach(AppState.InspectorTab.allCases) { tab in
                Button(action: {
                    appState.selectedInspectorTab = tab
                }) {
                    Image(systemName: tab.icon)
                        .font(.system(size: 14))
                        .frame(width: 32, height: 28)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.borderless)
                .foregroundStyle(
                    appState.selectedInspectorTab == tab
                        ? .primary
                        : .secondary
                )
                .background(
                    appState.selectedInspectorTab == tab
                        ? RoundedRectangle(cornerRadius: 6)
                            .fill(.quaternary)
                        : nil
                )
                .help(tab.rawValue)
            }
        }
    }

    @ViewBuilder
    private var inspectorContent: some View {
        if var project = appState.currentProject {
            let effectsBinding = Binding<EffectsConfig>(
                get: { project.effects },
                set: { newEffects in
                    project.effects = newEffects
                    appState.currentProject = project
                }
            )

            switch appState.selectedInspectorTab {
            case .background:
                BackgroundInspector(effects: effectsBinding)
            case .cursor:
                CursorInspector(effects: effectsBinding)
            case .keyboard:
                KeyboardInspector(effects: effectsBinding)
            case .camera:
                WebcamInspector(effects: effectsBinding)
            case .zoom:
                ZoomInspector(effects: effectsBinding)
            case .audio:
                AudioInspector(effects: effectsBinding)
            }
        } else {
            VStack(spacing: 12) {
                Image(systemName: "slider.horizontal.3")
                    .font(.system(size: 28))
                    .foregroundStyle(.tertiary)
                Text("No project loaded")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(.top, 40)
        }
    }
}

#Preview {
    EditorView()
        .environment(AppState())
        .frame(width: 1280, height: 800)
}
