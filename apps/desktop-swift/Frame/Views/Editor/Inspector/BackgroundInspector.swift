import SwiftUI

/// Background effects inspector panel.
struct BackgroundInspector: View {
    @Binding var effects: EffectsConfig

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Section: Background Type
            inspectorSection("Type") {
                Picker("Background", selection: $effects.backgroundType) {
                    ForEach(BackgroundType.allCases, id: \.self) { type in
                        Text(type.displayName).tag(type)
                    }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
            }

            // Section: Gradient Presets (when gradient selected)
            if effects.backgroundType == .gradient {
                inspectorSection("Preset") {
                    LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 6), count: 4), spacing: 6) {
                        ForEach(GradientPreset.allPresets) { preset in
                            Button(action: { effects.gradientPresetID = preset.id }) {
                                RoundedRectangle(cornerRadius: 6)
                                    .fill(
                                        LinearGradient(
                                            colors: preset.colors,
                                            startPoint: .topLeading,
                                            endPoint: .bottomTrailing
                                        )
                                    )
                                    .frame(height: 36)
                                    .overlay(
                                        RoundedRectangle(cornerRadius: 6)
                                            .strokeBorder(
                                                effects.gradientPresetID == preset.id
                                                    ? Color.white
                                                    : Color.clear,
                                                lineWidth: 2
                                            )
                                    )
                            }
                            .buttonStyle(.plain)
                            .help(preset.name)
                        }
                    }
                }
            }

            // Section: Solid Color (when solid selected)
            if effects.backgroundType == .solid {
                inspectorSection("Color") {
                    ColorPicker(
                        "Background Color",
                        selection: Binding(
                            get: { effects.backgroundColor.color },
                            set: { effects.backgroundColor = CodableColor(from: $0) }
                        )
                    )
                    .labelsHidden()
                }
            }

            Divider()

            // Section: Padding
            inspectorSection("Padding") {
                SliderRow(
                    label: "Size",
                    value: $effects.padding,
                    range: 0...128,
                    defaultValue: EffectsConfig.default.padding,
                    format: "%.0f",
                    unit: "px"
                )
            }

            // Section: Corner Radius
            inspectorSection("Corners") {
                SliderRow(
                    label: "Radius",
                    value: $effects.cornerRadius,
                    range: 0...48,
                    defaultValue: EffectsConfig.default.cornerRadius,
                    format: "%.0f",
                    unit: "px"
                )
            }

            Divider()

            // Section: Shadow
            inspectorSection("Shadow") {
                SliderRow(
                    label: "Blur",
                    value: $effects.shadowBlur,
                    range: 0...100,
                    defaultValue: EffectsConfig.default.shadowBlur,
                    format: "%.0f"
                )
                SliderRow(
                    label: "Opacity",
                    value: $effects.shadowOpacity,
                    range: 0...1,
                    defaultValue: EffectsConfig.default.shadowOpacity,
                    format: "%.0f",
                    multiplier: 100,
                    unit: "%"
                )
                SliderRow(
                    label: "Offset Y",
                    value: $effects.shadowOffsetY,
                    range: 0...40,
                    defaultValue: EffectsConfig.default.shadowOffsetY,
                    format: "%.0f"
                )
            }
        }
    }
}

// MARK: - Gradient Presets

struct GradientPreset: Identifiable {
    let id: String
    let name: String
    let colors: [Color]

    static let allPresets: [GradientPreset] = [
        GradientPreset(id: "sunset", name: "Sunset", colors: [.orange, .pink, .purple]),
        GradientPreset(id: "ocean", name: "Ocean", colors: [.blue, .cyan, .teal]),
        GradientPreset(id: "forest", name: "Forest", colors: [.green, .mint, .teal]),
        GradientPreset(id: "lavender", name: "Lavender", colors: [.purple, .indigo, .blue]),
        GradientPreset(id: "midnight", name: "Midnight", colors: [Color(white: 0.1), .indigo, Color(white: 0.1)]),
        GradientPreset(id: "flame", name: "Flame", colors: [.red, .orange, .yellow]),
        GradientPreset(id: "arctic", name: "Arctic", colors: [.white, .cyan, .blue]),
        GradientPreset(id: "slate", name: "Slate", colors: [Color(white: 0.2), Color(white: 0.35), Color(white: 0.2)]),
    ]
}

// MARK: - Background Type Display Name

extension BackgroundType {
    var displayName: String {
        switch self {
        case .wallpaper: return "Wallpaper"
        case .gradient: return "Gradient"
        case .solid: return "Solid"
        case .image: return "Image"
        }
    }
}

// MARK: - CodableColor SwiftUI Bridge

extension CodableColor {
    var color: Color {
        Color(red: red, green: green, blue: blue, opacity: alpha)
    }

    init(from color: Color) {
        let nsColor = NSColor(color).usingColorSpace(.sRGB) ?? NSColor(color)
        self.red = Double(nsColor.redComponent)
        self.green = Double(nsColor.greenComponent)
        self.blue = Double(nsColor.blueComponent)
        self.alpha = Double(nsColor.alphaComponent)
    }
}
