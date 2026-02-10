import SwiftUI

/// Keyboard overlay inspector panel — controls for keystroke visualization.
struct KeyboardInspector: View {
    @Binding var effects: EffectsConfig

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
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
                        defaultValue: EffectsConfig.default.keystrokeFontSize,
                        format: "%.0f",
                        unit: "pt"
                    )

                    SliderRow(
                        label: "Display duration",
                        value: $effects.keystrokeDisplayDuration,
                        range: 0.5...5.0,
                        defaultValue: EffectsConfig.default.keystrokeDisplayDuration,
                        format: "%.1f",
                        unit: "s"
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
