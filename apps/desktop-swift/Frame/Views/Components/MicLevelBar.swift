import SwiftUI

/// Polished live audio level bar for the toolbar mic button.
/// A smooth capsule that fills with a gradient and softly glows.
struct MicLevelBar: View {
    let level: CGFloat

    var body: some View {
        GeometryReader { geo in
            let fillWidth = max(0, min(geo.size.width, geo.size.width * level))

            ZStack(alignment: .leading) {
                // Track
                Capsule()
                    .fill(.white.opacity(0.12))

                // Fill
                Capsule()
                    .fill(
                        LinearGradient(
                            colors: gradientColors,
                            startPoint: .leading,
                            endPoint: .trailing
                        )
                    )
                    .frame(width: fillWidth)
                    .shadow(color: glowColor.opacity(0.6), radius: 4, x: 0, y: 0)
            }
        }
        .animation(.easeOut(duration: 0.06), value: level)
    }

    // Green → Yellow → Red gradient based on level
    private var gradientColors: [Color] {
        if level < 0.5 {
            return [.green, .green]
        } else if level < 0.75 {
            return [.green, .yellow]
        } else {
            return [.green, .yellow, .red]
        }
    }

    // Glow matches the dominant color
    private var glowColor: Color {
        if level < 0.5 {
            return .green
        } else if level < 0.75 {
            return .yellow
        } else {
            return .red
        }
    }
}
