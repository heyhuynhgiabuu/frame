import SwiftUI

/// Zoom effects inspector panel.
struct ZoomInspector: View {
    @Binding var effects: EffectsConfig

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            // Section: Auto Zoom
            inspectorSection("Auto Zoom") {
                Toggle("Enable auto-zoom on clicks", isOn: $effects.autoZoomEnabled)
                    .font(.system(size: 12))

                Text("Automatically zooms into areas where you click")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }

            if effects.autoZoomEnabled {
                Divider()

                // Section: Scale
                inspectorSection("Scale") {
                    SliderRow(
                        label: "Zoom level",
                        value: $effects.zoomScale,
                        range: 1.2...4.0,
                        format: "%.1fx"
                    )
                }

                Divider()

                // Section: Animation Style
                inspectorSection("Animation") {
                    Picker("Style", selection: $effects.zoomAnimationStyle) {
                        ForEach(ZoomAnimationStyle.allCases, id: \.self) { style in
                            Text(style.displayName).tag(style)
                        }
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()

                    Text(effects.zoomAnimationStyle.description)
                        .font(.caption2)
                        .foregroundStyle(.tertiary)
                }
            }
        }
    }
}

// MARK: - ZoomAnimationStyle Display

extension ZoomAnimationStyle {
    var displayName: String {
        switch self {
        case .slow: return "Slow"
        case .mellow: return "Mellow"
        case .quick: return "Quick"
        case .rapid: return "Rapid"
        }
    }

    var description: String {
        switch self {
        case .slow: return "Gentle, cinematic zoom transitions"
        case .mellow: return "Smooth, natural-feeling zoom"
        case .quick: return "Snappy, responsive zoom"
        case .rapid: return "Instant, punchy zoom"
        }
    }
}
