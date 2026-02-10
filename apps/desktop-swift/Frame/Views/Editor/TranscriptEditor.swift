import SwiftUI
import OSLog
import UniformTypeIdentifiers

private let logger = Logger(subsystem: "com.frame.app", category: "TranscriptEditor")

/// A sheet-based editor for correcting transcription text and adjusting segment timing.
struct TranscriptEditor: View {
    @Binding var segments: [CaptionSegment]
    let duration: TimeInterval
    @Environment(\.dismiss) private var dismiss
    @State private var editableSegments: [CaptionSegment] = []

    var body: some View {
        VStack(spacing: 0) {
            // Header
            header

            Divider()

            // Segment list
            if editableSegments.isEmpty {
                emptyState
            } else {
                segmentList
            }

            Divider()

            // Footer
            footer
        }
        .frame(width: 500, height: 450)
        .onAppear {
            editableSegments = segments
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack {
            Text("Edit Transcript")
                .font(.headline)

            Spacer()

            Text("\(editableSegments.count) segments")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding()
    }

    // MARK: - Empty State

    private var emptyState: some View {
        VStack(spacing: 8) {
            Spacer()
            Image(systemName: "text.bubble")
                .font(.system(size: 36))
                .foregroundStyle(.secondary)
            Text("No transcript segments")
                .font(.headline)
                .foregroundStyle(.secondary)
            Text("Generate a transcript first from the Captions inspector.")
                .font(.caption)
                .foregroundStyle(.tertiary)
            Spacer()
        }
        .frame(maxWidth: .infinity)
    }

    // MARK: - Segment List

    private var segmentList: some View {
        List {
            ForEach(Array(editableSegments.enumerated()), id: \.element.id) { index, _ in
                segmentRow(index: index)
            }
            .onDelete { offsets in
                editableSegments.remove(atOffsets: offsets)
            }
        }
        .listStyle(.inset)
    }

    private func segmentRow(index: Int) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            // Timing row
            HStack(spacing: 8) {
                Text(formatTime(editableSegments[index].startTime))
                    .font(.caption.monospaced())
                    .foregroundStyle(.secondary)

                Image(systemName: "arrow.right")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)

                Text(formatTime(editableSegments[index].endTime))
                    .font(.caption.monospaced())
                    .foregroundStyle(.secondary)

                Spacer()

                Text("#\(index + 1)")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }

            // Text editor
            TextField("Caption text", text: $editableSegments[index].text, axis: .vertical)
                .textFieldStyle(.roundedBorder)
                .lineLimit(1...4)
                .font(.body)
        }
        .padding(.vertical, 4)
    }

    // MARK: - Footer

    private var footer: some View {
        HStack {
            // Export button
            Button(action: exportSRT) {
                Label("Export .srt", systemImage: "square.and.arrow.up")
            }
            .buttonStyle(.borderless)
            .controlSize(.small)

            Spacer()

            Button("Cancel") {
                dismiss()
            }
            .keyboardShortcut(.cancelAction)

            Button("Save") {
                segments = editableSegments
                dismiss()
            }
            .buttonStyle(.borderedProminent)
            .keyboardShortcut(.defaultAction)
        }
        .padding()
    }

    // MARK: - Actions

    private func exportSRT() {
        let result = TranscriptionResult(
            segments: editableSegments,
            detectedLanguage: nil,
            modelUsed: "edited"
        )

        let srtContent = result.exportToSRT()

        let panel = NSSavePanel()
        panel.nameFieldStringValue = "transcript.srt"
        panel.allowedContentTypes = [UTType(filenameExtension: "srt") ?? .plainText]

        panel.begin { response in
            guard response == .OK, let url = panel.url else { return }
            do {
                try srtContent.write(to: url, atomically: true, encoding: .utf8)
                logger.info("Exported transcript to \(url.lastPathComponent)")
            } catch {
                logger.error("Export failed: \(error.localizedDescription)")
            }
        }
    }

    // MARK: - Helpers

    private func formatTime(_ seconds: TimeInterval) -> String {
        let minutes = Int(seconds) / 60
        let secs = Int(seconds) % 60
        let millis = Int((seconds.truncatingRemainder(dividingBy: 1)) * 100)
        return String(format: "%02d:%02d.%02d", minutes, secs, millis)
    }
}
