import SwiftUI

// MARK: - Inspector Section Header

/// Reusable labeled section for inspector panels.
func inspectorSection<Content: View>(
    _ title: String,
    @ViewBuilder content: () -> Content
) -> some View {
    VStack(alignment: .leading, spacing: 8) {
        Text(title)
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(.secondary)
            .textCase(.uppercase)

        content()
    }
}

// MARK: - Slider Row

/// A labeled slider with value display.
struct SliderRow: View {
    let label: String
    @Binding var value: Double
    let range: ClosedRange<Double>
    var format: String = "%.1f"
    var multiplier: Double = 1.0

    var body: some View {
        VStack(spacing: 2) {
            HStack {
                Text(label)
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)

                Spacer()

                Text(String(format: format, value * multiplier))
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .frame(minWidth: 40, alignment: .trailing)
            }

            Slider(value: $value, in: range)
                .controlSize(.small)
        }
    }
}
