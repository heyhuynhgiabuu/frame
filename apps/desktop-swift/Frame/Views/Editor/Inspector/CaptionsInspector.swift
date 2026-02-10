import SwiftUI
import OSLog

private let logger = Logger(subsystem: "com.frame.app", category: "CaptionsInspector")

/// Inspector panel for AI captions/transcription settings.
struct CaptionsInspector: View {
    @Binding var effects: EffectsConfig
    @Environment(AppState.self) private var appState
    @StateObject private var transcriptionEngine = WhisperTranscriptionEngine()
    @State private var showTranscriptEditor = false
    @State private var hasAudioTrack = false

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Enable toggle
            inspectorSection("Captions") {
                Toggle("Show captions", isOn: $effects.captionsEnabled)
                    .toggleStyle(.switch)
                    .controlSize(.small)
            }

            if effects.captionsEnabled {
                Divider()

                // Model selection
                inspectorSection("AI Model") {
                    Picker("Model", selection: $effects.captionModel) {
                        ForEach(WhisperModel.allCases) { model in
                            VStack(alignment: .leading) {
                                Text(model.rawValue)
                            }
                            .tag(model)
                        }
                    }
                    .pickerStyle(.segmented)

                    Text(effects.captionModel.speedDescription + " · " + effects.captionModel.sizeDescription)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }

                Divider()

                // Language selection
                inspectorSection("Language") {
                    Picker("Language", selection: $effects.captionLanguage) {
                        ForEach(TranscriptionLanguage.allCases) { lang in
                            Text(lang.rawValue).tag(lang)
                        }
                    }
                    .labelsHidden()
                    .onChange(of: effects.captionLanguage) { _, _ in
                        guard !effects.captionSegments.isEmpty, !transcriptionEngine.isTranscribing else { return }
                        Task {
                            await generateTranscript()
                        }
                    }
                }

                Divider()

                // Custom prompt
                inspectorSection("Custom Prompt") {
                    TextField("Product names, technical terms…", text: $effects.captionPrompt, axis: .vertical)
                        .textFieldStyle(.roundedBorder)
                        .lineLimit(2...4)
                        .font(.caption)

                    Text("Helps recognize specialized vocabulary")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }

                Divider()

                // Caption size
                inspectorSection("Display") {
                    SliderRow(
                        label: "Size",
                        value: $effects.captionFontSize,
                        range: 12...48,
                        defaultValue: 24,
                        format: "%.0f",
                        unit: "pt"
                    )
                }

                Divider()

                // Generate / Actions
                inspectorSection("Transcript") {
                    generateButton

                    if transcriptionEngine.isTranscribing {
                        progressIndicator
                    }

                    if let errorMessage = transcriptionEngine.errorMessage {
                        Label(errorMessage, systemImage: "exclamationmark.triangle.fill")
                            .font(.caption)
                            .foregroundStyle(.red)
                    }

                    if !effects.captionSegments.isEmpty {
                        transcriptActions
                    }
                }
            }
        }
    }

    // MARK: - Generate Button

    @ViewBuilder
    private var generateButton: some View {
        Button(action: {
            Task {
                await generateTranscript()
            }
        }) {
            HStack {
                if transcriptionEngine.isTranscribing {
                    ProgressView()
                        .controlSize(.small)
                        .scaleEffect(0.7)
                } else {
                    Image(systemName: "waveform.and.mic")
                }
                Text(effects.captionSegments.isEmpty ? "Generate Transcript" : "Regenerate")
            }
            .frame(maxWidth: .infinity)
        }
        .buttonStyle(.borderedProminent)
        .controlSize(.small)
        .disabled(!hasAudioTrack || transcriptionEngine.isTranscribing)
        .task(id: appState.currentProject?.recordingURL) {
            await checkForAudioTrack()
        }

        if !hasAudioTrack {
            Text("No audio recorded. Enable microphone during recording.")
                .font(.caption2)
                .foregroundStyle(.secondary)
        }
    }

    // MARK: - Progress

    private var progressIndicator: some View {
        VStack(alignment: .leading, spacing: 4) {
            ProgressView(value: transcriptionEngine.progress, total: 1.0)
                .progressViewStyle(.linear)

            Text(progressLabel)
                .font(.caption2)
                .foregroundStyle(.secondary)

            Button("Cancel") {
                transcriptionEngine.cancel()
            }
            .buttonStyle(.borderless)
            .controlSize(.small)
            .foregroundStyle(.red)
        }
    }

    private var progressLabel: String {
        let percent = Int(transcriptionEngine.progress * 100)
        if transcriptionEngine.progress < 0.3 {
            return "Extracting audio… \(percent)%"
        } else if transcriptionEngine.progress < 0.9 {
            return "Transcribing… \(percent)%"
        } else {
            return "Finishing… \(percent)%"
        }
    }

    // MARK: - Transcript Actions

    private var transcriptActions: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("\(effects.captionSegments.count) segments")
                .font(.caption)
                .foregroundStyle(.secondary)

            HStack(spacing: 8) {
                Button(action: {
                    showTranscriptEditor = true
                }) {
                    Label("Edit", systemImage: "pencil")
                }
                .buttonStyle(.borderless)
                .controlSize(.small)
                .sheet(isPresented: $showTranscriptEditor) {
                    TranscriptEditor(
                        segments: $effects.captionSegments,
                        duration: appState.currentProject?.duration ?? 0
                    )
                }

                Button(action: exportTranscript) {
                    Label("Export .srt", systemImage: "square.and.arrow.up")
                }
                .buttonStyle(.borderless)
                .controlSize(.small)

                Spacer()

                Button(action: {
                    effects.captionSegments = []
                }) {
                    Label("Clear", systemImage: "trash")
                }
                .buttonStyle(.borderless)
                .controlSize(.small)
                .foregroundStyle(.red)
            }
        }
    }

    // MARK: - Actions

    private func checkForAudioTrack() async {
        guard let videoURL = appState.currentProject?.recordingURL else {
            hasAudioTrack = false
            return
        }
        hasAudioTrack = await transcriptionEngine.hasAudio(in: videoURL)
    }

    private func generateTranscript() async {
        guard let videoURL = appState.currentProject?.recordingURL else { return }

        do {
            let result = try await transcriptionEngine.transcribe(
                videoURL: videoURL,
                model: effects.captionModel,
                language: effects.captionLanguage,
                prompt: effects.captionPrompt.isEmpty ? nil : effects.captionPrompt
            )
            effects.captionSegments = result.segments
            logger.info("Generated \(result.segments.count) caption segments")
        } catch is CancellationError {
            logger.info("Transcription cancelled by user")
        } catch {
            logger.error("Transcription failed: \(error.localizedDescription)")
        }
    }

    private func exportTranscript() {
        let result = TranscriptionResult(
            segments: effects.captionSegments,
            detectedLanguage: effects.captionLanguage.languageCode,
            modelUsed: effects.captionModel.rawValue
        )

        let srtContent = result.exportToSRT()

        let panel = NSSavePanel()
        panel.nameFieldStringValue = "\(appState.currentProject?.name ?? "transcript").srt"
        panel.allowedContentTypes = [.init(filenameExtension: "srt") ?? .plainText]

        panel.begin { response in
            guard response == .OK, let url = panel.url else { return }
            do {
                try srtContent.write(to: url, atomically: true, encoding: .utf8)
                logger.info("Exported transcript to \(url.lastPathComponent)")
            } catch {
                logger.error("Failed to export transcript: \(error.localizedDescription)")
            }
        }
    }
}
