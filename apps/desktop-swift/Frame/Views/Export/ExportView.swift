import SwiftUI

/// Export sheet — format picker, quality settings, progress bar, export/cancel.
struct ExportView: View {
    @Environment(AppState.self) private var appState
    @Environment(\.dismiss) private var dismiss

    @State private var config = ExportConfig()
    @State private var showingSavePanel = false

    var body: some View {
        VStack(spacing: 0) {
            // Header
            header
                .padding(.horizontal, 20)
                .padding(.top, 20)
                .padding(.bottom, 12)

            Divider()

            if appState.exportEngine.isExporting {
                // Export in-progress view
                exportProgressView
                    .padding(20)
            } else {
                // Settings
                ScrollView {
                    settingsContent
                        .padding(20)
                }
            }

            Divider()

            // Footer buttons
            footer
                .padding(.horizontal, 20)
                .padding(.vertical, 16)
        }
        .frame(width: 420, height: appState.exportEngine.isExporting ? 280 : 520)
        .background(.ultraThickMaterial)
    }

    // MARK: - Header

    private var header: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text("Export Video")
                    .font(.headline)
                if let project = appState.currentProject {
                    Text(project.name)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            Spacer()

            if !appState.exportEngine.isExporting {
                // Format quick-select
                Picker("Format", selection: $config.format) {
                    ForEach(ExportConfig.ExportFormat.allCases) { format in
                        Label(format.rawValue, systemImage: format.icon)
                            .tag(format)
                    }
                }
                .pickerStyle(.segmented)
                .fixedSize()
            }
        }
    }

    // MARK: - Settings Content

    private var settingsContent: some View {
        VStack(alignment: .leading, spacing: 20) {
            // Format description
            formatInfo

            // Quality
            qualitySection

            // Resolution
            resolutionSection

            // Frame Rate
            frameRateSection

            // GIF options (only for GIF format)
            if config.format == .gif {
                gifSection
            }
        }
    }

    // MARK: - Format Info

    private var formatInfo: some View {
        HStack(spacing: 12) {
            Image(systemName: config.format.icon)
                .font(.title2)
                .foregroundStyle(.blue)
                .frame(width: 36, height: 36)
                .background(.blue.opacity(0.1))
                .clipShape(RoundedRectangle(cornerRadius: 8))

            VStack(alignment: .leading, spacing: 2) {
                Text(config.format.rawValue)
                    .font(.subheadline.weight(.medium))
                Text(config.format.description)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(RoundedRectangle(cornerRadius: 8).fill(.quaternary.opacity(0.5)))
    }

    // MARK: - Quality Section

    private var qualitySection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Quality")
                .font(.subheadline.weight(.medium))

            Picker("Quality", selection: $config.quality) {
                ForEach(ExportConfig.ExportQuality.allCases) { q in
                    Text(q.rawValue).tag(q)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()

            // Bitrate estimate
            Text("~\(config.quality.baseBitrate / 1_000_000) Mbps")
                .font(.caption2)
                .foregroundStyle(.tertiary)
        }
    }

    // MARK: - Resolution Section

    private var resolutionSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Resolution")
                .font(.subheadline.weight(.medium))

            Picker("Resolution", selection: $config.resolution) {
                ForEach(ExportConfig.ExportResolution.allCases) { res in
                    Text(res.rawValue).tag(res)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
        }
    }

    // MARK: - Frame Rate Section

    private var frameRateSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Frame Rate")
                .font(.subheadline.weight(.medium))

            Picker("Frame Rate", selection: $config.frameRate) {
                ForEach(ExportConfig.ExportFrameRate.allCases) { fps in
                    Text(fps.displayName).tag(fps)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
        }
    }

    // MARK: - GIF Section

    private var gifSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Divider()

            Text("GIF Options")
                .font(.subheadline.weight(.medium))

            HStack {
                Text("FPS")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Spacer()
                Picker("GIF FPS", selection: $config.gifFPS) {
                    Text("10").tag(10)
                    Text("15").tag(15)
                    Text("20").tag(20)
                    Text("25").tag(25)
                }
                .pickerStyle(.segmented)
                .fixedSize()
                .labelsHidden()
            }

            HStack {
                Text("Colors")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Spacer()
                Picker("Colors", selection: $config.gifMaxColors) {
                    Text("64").tag(64)
                    Text("128").tag(128)
                    Text("256").tag(256)
                }
                .pickerStyle(.segmented)
                .fixedSize()
                .labelsHidden()
            }
        }
    }

    // MARK: - Export Progress

    private var exportProgressView: some View {
        VStack(spacing: 20) {
            Spacer()

            // Phase icon
            Image(systemName: phaseIcon)
                .font(.system(size: 36))
                .foregroundStyle(.blue)
                .symbolEffect(.pulse, isActive: appState.exportEngine.isExporting)

            // Phase text
            Text(appState.exportEngine.currentPhase.rawValue)
                .font(.headline)

            // Progress bar
            VStack(spacing: 6) {
                ProgressView(value: appState.exportEngine.progress)
                    .progressViewStyle(.linear)
                    .tint(.blue)

                Text("\(Int(appState.exportEngine.progress * 100))%")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(.secondary)
            }

            // Error message
            if let error = appState.exportEngine.exportError {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.red)
                    .multilineTextAlignment(.center)
            }

            Spacer()
        }
        .frame(maxWidth: .infinity)
    }

    private var phaseIcon: String {
        switch appState.exportEngine.currentPhase {
        case .idle: return "square.and.arrow.up"
        case .preparing: return "gearshape.2"
        case .rendering: return "paintbrush"
        case .encoding: return "film"
        case .finalizing: return "checkmark.circle"
        case .complete: return "checkmark.circle.fill"
        case .failed: return "xmark.circle.fill"
        }
    }

    // MARK: - Footer

    private var footer: some View {
        HStack {
            if appState.exportEngine.isExporting {
                Button("Cancel") {
                    appState.exportEngine.cancel()
                }
                .keyboardShortcut(.cancelAction)

                Spacer()

                if appState.exportEngine.currentPhase == .complete {
                    Button("Done") {
                        appState.exportEngine.cancel()    // Reset state
                        dismiss()
                    }
                    .keyboardShortcut(.defaultAction)
                    .buttonStyle(.borderedProminent)
                }
            } else {
                Button("Cancel") {
                    dismiss()
                }
                .keyboardShortcut(.cancelAction)

                Spacer()

                // Estimated file size hint
                estimatedSize

                Button("Export") {
                    startExport()
                }
                .keyboardShortcut(.defaultAction)
                .buttonStyle(.borderedProminent)
                .disabled(appState.currentProject == nil)
            }
        }
    }

    private var estimatedSize: some View {
        Group {
            if let project = appState.currentProject {
                let duration = max(1, project.duration)
                let bitrate = Double(config.quality.baseBitrate)
                let sizeMB = (bitrate * duration) / 8 / 1_000_000
                Text("~\(String(format: "%.0f", sizeMB)) MB")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }
        }
    }

    // MARK: - Actions

    private func startExport() {
        guard let project = appState.currentProject else { return }

        let panel = NSSavePanel()
        panel.title = "Export Video"
        panel.nameFieldStringValue = config.defaultFilename(projectName: project.name)
        panel.allowedContentTypes = [
            config.format == .mp4 ? .mpeg4Movie :
            config.format == .mov ? .quickTimeMovie :
                .gif
        ]
        panel.canCreateDirectories = true

        panel.begin { response in
            guard response == .OK, let url = panel.url else { return }
            appState.exportEngine.export(
                project: project,
                config: config,
                outputURL: url
            )
        }
    }
}

#Preview {
    ExportView()
        .environment(AppState())
}
