# Frame Setup Guide

Complete guide for setting up Frame development environment.

---

## Prerequisites

### Required

- **macOS 13.0+** (Ventura) — ScreenCaptureKit requires macOS 13.0 or later
- **Xcode 15.0+** — For building the Swift desktop app

### Optional

- **Bun 1.0+** — For JS tooling (linting/formatting)
- **BlackHole 0.5.0+** — Virtual audio driver for system audio capture

---

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/frame/frame.git
cd frame
```

### 2. Open in Xcode

```bash
open apps/desktop-swift/Frame.xcodeproj
```

### 3. Run

Press **⌘R** in Xcode to build and run.

Or from command line:

```bash
xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build
```

---

## Permissions Setup

Frame requires these macOS permissions to function:

### Screen Recording

1. Open **System Settings** → **Privacy & Security** → **Screen Recording**
2. Enable Frame (or your terminal/Xcode if running in debug)
3. You may need to restart the app after granting permission

### Camera

- Granted via system prompt on first webcam use
- Can be managed in **System Settings** → **Privacy & Security** → **Camera**

### Microphone

- Granted via system prompt on first audio capture
- Can be managed in **System Settings** → **Privacy & Security** → **Microphone**

---

## BlackHole Setup (Optional — System Audio)

BlackHole is required for capturing system audio (app sounds, video audio, etc.):

### Install

```bash
# Using Homebrew (recommended)
brew install blackhole-2ch
```

Or download from [existential.audio/blackhole](https://existential.audio/blackhole/).

### Configure

1. Open **Audio MIDI Setup** (search in Spotlight)
2. Click **+** → **Create Multi-Output Device**
3. Check both **BlackHole 2ch** and your speakers/headphones
4. Right-click the multi-output device → **Use This Device for Sound Output**
5. In Frame settings, select **BlackHole 2ch** as system audio input

---

## IDE Setup

### Xcode (Primary)

Xcode is the primary development environment:

1. Open `apps/desktop-swift/Frame.xcodeproj`
2. Select the **Frame** scheme
3. **⌘R** to run, **⌘B** to build, **⌘U** to test

### VS Code (Optional — for docs/JS)

Recommended extensions:

- **Biome** — JS linting and formatting
- **Swift** (sswg.swift-lang) — Basic Swift support

---

## Project Structure

```
frame/
├── apps/
│   └── desktop-swift/    # Swift/macOS native app (Xcode project)
│       └── Frame/
│           ├── App/          # AppState, entry point
│           ├── Recording/    # Screen, webcam, cursor capture
│           ├── Playback/     # Video playback
│           ├── Export/       # Export engine
│           ├── Overlay/      # Floating panels
│           ├── Effects/      # Zoom, visual effects
│           ├── Models/       # Data models
│           ├── Utilities/    # Permissions
│           └── Views/        # SwiftUI views
├── docs/                 # Documentation
├── package.json          # Bun workspace (JS tooling only)
└── biome.json            # Biome configuration
```

---

## Troubleshooting

### Build Errors

#### "ScreenCaptureKit not available"

- Ensure you're on macOS 13.0 or later
- Check: `sw_vers -productVersion`

#### "Permission denied" when recording

1. Open **System Settings** → **Privacy & Security** → **Screen Recording**
2. Add Xcode and/or Frame
3. Restart the app

### Runtime Issues

#### Webcam preview frozen during recording

- This was fixed with GPU-backed CIImageView rendering
- If still occurring, check Console.app for errors

#### No audio in recording

1. Check microphone permissions
2. If using system audio, ensure BlackHole is installed and configured
3. Check audio levels in Frame settings

#### Recording stops unexpectedly

- Check available disk space (need at least 1GB free)
- Check Console.app for crash logs
- Try reducing recording resolution or frame rate

---

## Next Steps

1. Read the [Documentation](README.md)
2. Check out [Contributing Guide](CONTRIBUTING.md)
3. Open the Xcode project and explore the codebase
