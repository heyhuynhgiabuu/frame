import SwiftUI

// MARK: - Accent Color

/// Professional purple/blue accent color for UI elements.
extension Color {
    static let frameAccent = Color(red: 0.486, green: 0.486, blue: 1.0) // #7C7CFF
}

// MARK: - Inspector Section Header

/// Reusable labeled section for inspector panels.
func inspectorSection<Content: View>(
    _ title: String,
    @ViewBuilder content: () -> Content
) -> some View {
    VStack(alignment: .leading, spacing: 6) {
        Text(title)
            .font(.system(size: 10, weight: .semibold))
            .foregroundStyle(.secondary)

        content()
    }
}

// MARK: - Slider Row

/// A labeled slider with value display and reset button.
struct SliderRow: View {
    let label: String
    @Binding var value: Double
    let range: ClosedRange<Double>
    let defaultValue: Double
    var format: String = "%.1f"
    var multiplier: Double = 1.0
    var unit: String = ""

    private var displayValue: String {
        let formatted = String(format: format, value * multiplier)
        return unit.isEmpty ? formatted : "\(formatted) \(unit)"
    }

    private var isAtDefault: Bool {
        abs(value - defaultValue) < 0.001
    }

    var body: some View {
        VStack(spacing: 2) {
            HStack(spacing: 8) {
                Text(label)
                    .font(.system(size: 11))
                    .foregroundStyle(.secondary)

                Spacer()

                if !isAtDefault {
                    Button("Reset") {
                        withAnimation(.easeInOut(duration: 0.15)) {
                            value = defaultValue
                        }
                    }
                    .font(.system(size: 10, weight: .medium))
                    .foregroundColor(.frameAccent)
                    .buttonStyle(.plain)
                    .opacity(0.8)
                    .help("Reset to default")
                }

                Text(displayValue)
                    .font(.system(size: 10, weight: .medium, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .frame(minWidth: 40, alignment: .trailing)
            }

            Slider(value: $value, in: range)
                .controlSize(.small)
                .tint(.frameAccent)
        }
    }
}
