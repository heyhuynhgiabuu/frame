# Frame

An open-core screen recorder built for developers. Beautiful by default, extensible by design.

100% Swift. Native macOS.

## Quick Start

Open `apps/desktop-swift/Frame.xcodeproj` in Xcode and press ⌘R.

Or build from command line:

```bash
xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build
```

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
│           └── Views/        # SwiftUI views
└── docs/                 # Documentation
```

## Tech Stack

| Component      | Technology        | Purpose                       |
| -------------- | ----------------- | ----------------------------- |
| Language       | Swift 5.9+        | Application logic             |
| UI Framework   | SwiftUI + AppKit  | Native macOS interface        |
| Screen Capture | ScreenCaptureKit  | macOS native screen recording |
| Webcam         | AVFoundation      | Camera input                  |
| Video Encoding | AVAssetWriter     | Hardware-accelerated encoding |
| GPU Rendering  | CoreImage + Metal | Effects, webcam preview       |
| Display Sync   | CoreVideo         | CVDisplayLink frame timing    |

## License

MIT/Apache-2.0 for core, commercial license for Pro features
