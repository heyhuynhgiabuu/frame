import SwiftUI

/// Cursor effects inspector panel.
struct CursorInspector: View {
    @Binding var effects: EffectsConfig

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Section: Enable
            inspectorSection("Cursor") {
                Toggle("Show cursor overlay", isOn: $effects.cursorEnabled)
                    .font(.system(size: 12))
            }

            if effects.cursorEnabled {
                Divider()

                // Section: Smoothing
                inspectorSection("Smoothing") {
                    SliderRow(
                        label: "Amount",
                        value: $effects.cursorSmoothing,
                        range: 0...1,
                        defaultValue: EffectsConfig.default.cursorSmoothing,
                        format: "%.0f",
                        multiplier: 100,
                        unit: "%"
                    )

                    Text("Smooths cursor movement to reduce jitter")
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }

                Divider()

                // Section: Appearance
                inspectorSection("Appearance") {
                    SliderRow(
                        label: "Scale",
                        value: $effects.cursorScale,
                        range: 0.5...3.0,
                        defaultValue: EffectsConfig.default.cursorScale,
                        format: "%.1f",
                        unit: "x"
                    )

                    Toggle("Highlight circle", isOn: $effects.cursorHighlight)
                        .font(.system(size: 12))
                }

                Divider()

                // Section: Auto-Hide
                inspectorSection("Auto-Hide") {
                    Toggle("Hide cursor when idle", isOn: $effects.cursorAutoHide)
                        .font(.system(size: 12))

                    if effects.cursorAutoHide {
                        SliderRow(
                            label: "Delay",
                            value: $effects.cursorAutoHideDelay,
                            range: 0.5...5.0,
                            defaultValue: EffectsConfig.default.cursorAutoHideDelay,
                            format: "%.1f",
                            unit: "s"
                        )
                    }
                }

                Divider()

                // Section: Click Effect
                inspectorSection("Click Effect") {
                    Toggle("Show click ripple", isOn: $effects.clickEffectEnabled)
                        .font(.system(size: 12))

                    if effects.clickEffectEnabled {
                        ColorPicker(
                            "Click color",
                            selection: Binding(
                                get: { effects.cursorClickColor.color },
                                set: { effects.cursorClickColor = CodableColor(from: $0) }
                            )
                        )
                        .font(.system(size: 12))
                    }
                }
            }
        }
    }
}
