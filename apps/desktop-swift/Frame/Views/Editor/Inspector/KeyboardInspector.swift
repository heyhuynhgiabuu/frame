import SwiftUI

/// Keyboard overlay inspector panel — controls for keystroke visualization.
struct KeyboardInspector: View {
    @Binding var effects: EffectsConfig

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            // Section: Enable
            inspectorSection("Keystrokes") {
                Toggle("Show keystroke overlay", isOn: $effects.keystrokesEnabled)
                    .font(.system(size: 12))

                Text("Displays keyboard shortcuts and key presses during playback")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }

            if effects.keystrokesEnabled {
                Divider()

                // Section: Appearance
                inspectorSection("Appearance") {
                    SliderRow(
                        label: "Font size",
                        value: $effects.keystrokeFontSize,
                        range: 10...28,
                        format: "%.0fpt"
                    )

                    SliderRow(
                        label: "Display duration",
                        value: $effects.keystrokeDisplayDuration,
                        range: 0.5...5.0,
                        format: "%.1fs"
                    )
                }

                Divider()

                // Section: Filter
                inspectorSection("Filter") {
                    Toggle("Only show shortcuts", isOn: $effects.keystrokesOnlyShortcuts)
                        .font(.system(size: 12))

                    Text("When enabled, only key presses with modifiers (⌘, ⌃, ⌥) are shown")
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
            }
        }
    }
}
