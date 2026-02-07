import SwiftUI

/// Webcam overlay inspector panel.
struct WebcamInspector: View {
    @Binding var effects: EffectsConfig

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            // Section: Enable
            inspectorSection("Webcam Overlay") {
                Toggle("Show webcam", isOn: $effects.webcamEnabled)
                    .font(.system(size: 12))
            }

            if effects.webcamEnabled {
                Divider()

                // Section: Position
                inspectorSection("Position") {
                    Picker("Position", selection: $effects.webcamPosition) {
                        Text("Top Left").tag(WebcamPosition.topLeft)
                        Text("Top Right").tag(WebcamPosition.topRight)
                        Text("Bottom Left").tag(WebcamPosition.bottomLeft)
                        Text("Bottom Right").tag(WebcamPosition.bottomRight)
                    }
                    .pickerStyle(.radioGroup)
                    .labelsHidden()
                }

                Divider()

                // Section: Size
                inspectorSection("Size") {
                    SliderRow(
                        label: "Scale",
                        value: $effects.webcamSize,
                        range: 0.1...0.4,
                        format: "%.0f%%",
                        multiplier: 100
                    )
                }

                Divider()

                // Section: Shape
                inspectorSection("Shape") {
                    Picker("Shape", selection: $effects.webcamShape) {
                        ForEach(WebcamShape.allCases, id: \.self) { shape in
                            Text(shape.displayName).tag(shape)
                        }
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                }
            }
        }
    }
}

// MARK: - WebcamShape Display Name

extension WebcamShape {
    var displayName: String {
        switch self {
        case .circle: return "Circle"
        case .roundedRectangle: return "Rounded"
        case .rectangle: return "Rectangle"
        }
    }
}
