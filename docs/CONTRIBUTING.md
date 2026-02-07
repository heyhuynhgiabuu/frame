# Contributing to Frame

Thank you for your interest in contributing to Frame! This guide will help you get started.

---

## Code of Conduct

- Be respectful and inclusive
- Welcome newcomers
- Focus on constructive feedback
- Respect different viewpoints and experiences

---

## How to Contribute

### Reporting Bugs

Include:

- **macOS version** (e.g., 14.2.1)
- **Xcode version** (e.g., 15.2)
- **Hardware** (e.g., MacBook Pro M1, 16GB RAM)
- **Steps to reproduce**
- **Expected vs actual behavior**
- **Screenshots/videos** if applicable
- **Console logs** (from Console.app or Xcode)

### Pull Requests

1. **Fork** the repository
2. **Create a branch** from `main` (e.g., `feature/my-feature` or `fix/bug-description`)
3. **Make your changes** following coding standards
4. **Test** in Xcode (⌘R to run, ⌘U for tests)
5. **Submit a pull request** with a clear description

---

## Development Setup

See [SETUP.md](SETUP.md) for detailed instructions.

Quick start:

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/frame.git
cd frame

# Open in Xcode
open apps/desktop-swift/Frame.xcodeproj

# Press ⌘R to build and run
```

---

## Coding Standards

### Swift

- Follow [Swift API Design Guidelines](https://www.swift.org/documentation/api-design-guidelines/)
- Use SwiftUI for new views
- Use `@Observable` for state (not `ObservableObject` unless needed for Combine)
- Handle errors with `throws` / `Result` — no force unwraps (`!`) in production
- Use `async/await` for asynchronous code
- Keep functions focused and under 50 lines when possible

Example:

```swift
/// Starts screen recording with the given configuration.
///
/// - Parameter config: Recording settings (frame rate, audio, etc.)
/// - Throws: `RecordingError` if permissions are denied or capture fails
func startRecording(config: RecordingConfig) async throws {
    guard Permissions.hasScreenRecording else {
        throw RecordingError.screenRecordingPermissionDenied
    }
    // ...
}
```

### Git Commits

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(recording): add webcam compositing during capture
fix(overlay): resolve webcam preview freeze during recording
refactor(export): simplify encoding pipeline
```

---

## Project Structure

```
apps/desktop-swift/
└── Frame/
    ├── App/          # App entry point, AppState
    ├── Recording/    # Screen, webcam, cursor, keystroke capture
    ├── Playback/     # Video playback engine
    ├── Export/       # Export config & engine
    ├── Overlay/      # Floating panels (toolbar, webcam preview)
    ├── Effects/      # Zoom engine, visual effects
    ├── Models/       # Data models
    ├── Utilities/    # Permissions, helpers
    └── Views/        # SwiftUI views
```

---

## Testing

### In Xcode

- **Run app:** ⌘R
- **Run tests:** ⌘U
- **Build only:** ⌘B

### Command Line

```bash
xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build
xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame test
```

---

## Before Submitting

- [ ] Code compiles without warnings (⌘B in Xcode)
- [ ] All tests pass (⌘U in Xcode)
- [ ] No force unwraps (`!`) in production code
- [ ] Tested on macOS 13.0+ target
- [ ] Commit messages follow conventions
- [ ] Documentation updated if needed

---

## Areas for Contribution

- **Effects** — Cursor smoothing, zoom, motion blur
- **Export formats** — Additional codecs and formats
- **Accessibility** — VoiceOver support, keyboard navigation
- **Performance** — Recording optimization, memory usage
- **UI/UX** — Design improvements, animations

---

## License

By contributing to Frame, you agree that your contributions will be licensed under the MIT and Apache-2.0 licenses.
