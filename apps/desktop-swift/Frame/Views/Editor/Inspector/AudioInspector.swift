import SwiftUI

/// Audio settings inspector panel.
struct AudioInspector: View {
    @Binding var effects: EffectsConfig

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            // Section: Audio Sources
            inspectorSection("Sources") {
                Toggle("System audio", isOn: $effects.systemAudioEnabled)
                    .font(.system(size: 12))

                Toggle("Microphone", isOn: $effects.microphoneEnabled)
                    .font(.system(size: 12))
            }

            Divider()

            // Section: Volume
            inspectorSection("Volume") {
                SliderRow(
                    label: "Master",
                    value: $effects.volume,
                    range: 0...1,
                    format: "%.0f%%",
                    multiplier: 100
                )
            }
        }
    }
}
