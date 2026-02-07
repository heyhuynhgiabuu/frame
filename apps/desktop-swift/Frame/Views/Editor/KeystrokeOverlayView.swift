import SwiftUI

/// Renders recent keystroke events as a floating pill overlay on the preview canvas.
/// Keys appear at the bottom-center and fade out after a short duration.
struct KeystrokeOverlayView: View {
    let keystrokeEvents: [KeystrokeEvent]
    let effects: EffectsConfig
    let currentTime: TimeInterval

    /// How long each key press remains visible (seconds)
    private let displayDuration: TimeInterval = 1.5
    /// Maximum number of recent keys to show at once
    private let maxVisible = 6

    var body: some View {
        if effects.keystrokesEnabled {
            GeometryReader { geometry in
                VStack {
                    Spacer()

                    // Keystroke pill at the bottom
                    keystrokePill
                        .padding(.bottom, 24)
                        .frame(maxWidth: .infinity)
                }
            }
        }
    }

    // MARK: - Keystroke Pill

    @ViewBuilder
    private var keystrokePill: some View {
        let recentKeys = recentKeyPresses

        if !recentKeys.isEmpty {
            HStack(spacing: 6) {
                ForEach(recentKeys) { entry in
                    keyBadge(entry)
                        .opacity(opacity(for: entry))
                }
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .background {
                Capsule()
                    .fill(.ultraThinMaterial)
                    .shadow(color: .black.opacity(0.2), radius: 8, y: 4)
            }
            .animation(.easeInOut(duration: 0.15), value: recentKeys.count)
        }
    }

    /// Renders a single key badge (modifiers + key label).
    private func keyBadge(_ event: KeystrokeEvent) -> some View {
        HStack(spacing: 2) {
            // Modifier symbols
            ForEach(event.modifiers.symbols, id: \.self) { symbol in
                Text(symbol)
                    .font(.system(size: effects.keystrokeFontSize * 0.85, weight: .medium, design: .rounded))
                    .foregroundStyle(.secondary)
            }

            // Key label
            Text(event.characters)
                .font(.system(size: effects.keystrokeFontSize, weight: .semibold, design: .rounded))
                .foregroundStyle(.primary)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background {
            RoundedRectangle(cornerRadius: 6)
                .fill(Color(white: 0.15).opacity(0.6))
                .overlay {
                    RoundedRectangle(cornerRadius: 6)
                        .stroke(Color.white.opacity(0.1), lineWidth: 0.5)
                }
        }
    }

    // MARK: - Helpers

    /// Filters to only recent key-down events within the display window.
    private var recentKeyPresses: [KeystrokeEvent] {
        let windowStart = currentTime - displayDuration

        return keystrokeEvents
            .filter { event in
                event.isDown
                && event.timestamp >= windowStart
                && event.timestamp <= currentTime
                // Skip standalone modifier-only events (they'll show as prefixes)
                && !isModifierOnly(event)
            }
            .suffix(maxVisible)
            .map { $0 }   // Convert ArraySlice to Array
    }

    /// Whether this event is a standalone modifier press (no actual key).
    private func isModifierOnly(_ event: KeystrokeEvent) -> Bool {
        let modifierLabels: Set<String> = ["⌘", "⇧", "⌥", "⌃", "fn"]
        return modifierLabels.contains(event.characters)
    }

    /// Fade out opacity as the event ages.
    private func opacity(for event: KeystrokeEvent) -> Double {
        let age = currentTime - event.timestamp
        let fadeStart = displayDuration * 0.6
        if age < fadeStart {
            return 1.0
        }
        let fadeProgress = (age - fadeStart) / (displayDuration - fadeStart)
        return max(0, 1.0 - fadeProgress)
    }
}
