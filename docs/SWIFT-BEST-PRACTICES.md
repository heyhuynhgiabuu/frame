# Swift Best Practices Reference — Frame

Comprehensive reference for developing Frame, a native macOS screen recorder. Sourced from Apple documentation (Context7), real-world open-source examples (grep.app), and Swift community best practices.

**Last updated:** 2026-02-07

---

## Table of Contents

1. [ScreenCaptureKit](#1-screencapturekit)
2. [AVFoundation — Camera Capture](#2-avfoundation--camera-capture)
3. [AVAssetWriter — Video Encoding](#3-avassetwriter--video-encoding)
4. [CoreImage & Metal — GPU Rendering](#4-coreimage--metal--gpu-rendering)
5. [CoreVideo — CVDisplayLink](#5-corevideo--cvdisplaylink)
6. [SwiftUI + AppKit Integration](#6-swiftui--appkit-integration)
7. [Floating Panels (NSPanel)](#7-floating-panels-nspanel)
8. [Swift Concurrency](#8-swift-concurrency)
9. [Combine Framework](#9-combine-framework)
10. [@Observable State Management](#10-observable-state-management)
11. [Error Handling Patterns](#11-error-handling-patterns)
12. [Performance Guidelines](#12-performance-guidelines)

---

## 1. ScreenCaptureKit

### SCStream Configuration

```swift
let config = SCStreamConfiguration()

// Resolution: match display × scaleFactor for retina
config.width = Int(display.width) * Int(display.scaleFactor)
config.height = Int(display.height) * Int(display.scaleFactor)

// Frame rate
config.minimumFrameInterval = CMTime(value: 1, timescale: 60)  // 60fps

// Queue depth: 3 (default), max 8. Higher = more memory, less stalling
// Recommendation: 5 for recording apps
config.queueDepth = 5

// Audio
config.capturesAudio = true
config.excludesCurrentProcessAudio = true
config.captureMicrophone = true

// Cursor
config.showsCursor = true
config.showMouseClicks = true

// Color & pixel format
config.pixelFormat = kCVPixelFormatType_32BGRA
```

### Stream Setup — Separate Queues per Output Type

```swift
let videoQueue = DispatchQueue(label: "com.frame.screen.video", qos: .userInteractive)
let audioQueue = DispatchQueue(label: "com.frame.screen.audio", qos: .userInteractive)
let micQueue   = DispatchQueue(label: "com.frame.screen.mic",   qos: .userInteractive)

let stream = SCStream(filter: filter, configuration: config, delegate: self)
try stream.addStreamOutput(self, type: .screen, sampleHandlerQueue: videoQueue)
try stream.addStreamOutput(self, type: .audio, sampleHandlerQueue: audioQueue)
try stream.addStreamOutput(self, type: .microphone, sampleHandlerQueue: micQueue)

try await stream.startCapture()
```

**Key rule:** Always use separate dispatch queues for video, audio, and microphone outputs. Never share a single queue — it creates priority inversion and frame drops.

### Content Filtering — Excluding Windows

```swift
// Get available content
let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)

// Option 1: Exclude specific windows (e.g., floating panels)
let filter = SCContentFilter(
    display: targetDisplay,
    excludingWindows: panelWindows  // [SCWindow]
)

// Option 2: Exclude entire app + except specific windows
let filter = SCContentFilter(
    display: targetDisplay,
    excludingApplications: [myApp],
    exceptingWindows: []
)

// Option 3: Find own app by bundle ID
let ownApp = content.applications.first {
    $0.bundleIdentifier == Bundle.main.bundleIdentifier
}
```

### Dynamic Configuration Updates (No Restart)

```swift
// Update capture settings without stopping/restarting
try await stream.updateConfiguration(newConfig)
try await stream.updateContentFilter(newFilter)
```

### Error Handling

```swift
// SCStreamDelegate
func stream(_ stream: SCStream, didStopWithError error: any Error) {
    let nsError = error as NSError
    switch nsError.code {
    case SCStreamError.attemptToUpdateFilterState.rawValue:
        // Filter update failed — retry or notify user
    case SCStreamError.systemStoppedStream.rawValue:
        // System killed stream (e.g., display sleep)
    case SCStreamError.failedToStart.rawValue:
        // Permissions or resource issue
    default:
        break
    }
}
```

### Best Practices

- **Retina displays:** Always multiply dimensions by `scaleFactor`
- **queueDepth:** Use 5 for recording; 3 for preview-only
- **Filter updates:** Use `updateContentFilter()` instead of stopping/restarting the stream
- **Memory:** Monitor `queueDepth` — higher values use more memory per frame × depth
- **Permissions:** Always check `SCShareableContent.current` before starting; handle `.notAuthorized` gracefully

---

## 2. AVFoundation — Camera Capture

### AVCaptureSession Setup

```swift
let session = AVCaptureSession()
session.beginConfiguration()

// Preset
session.sessionPreset = .medium  // 480p — good for webcam PIP

// Input
guard let camera = AVCaptureDevice.default(for: .video),
      let input = try? AVCaptureDeviceInput(device: camera) else { return }
if session.canAddInput(input) {
    session.addInput(input)
}

// Output
let output = AVCaptureVideoDataOutput()
output.videoSettings = [
    kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA
]
output.setSampleBufferDelegate(self, queue: captureQueue)
output.alwaysDiscardsLateVideoFrames = true  // Critical for live preview
if session.canAddOutput(output) {
    session.addOutput(output)
}

session.commitConfiguration()
session.startRunning()
```

### Best Practices

- **beginConfiguration/commitConfiguration:** Always wrap setup changes in this pair for atomic configuration
- **alwaysDiscardsLateVideoFrames = true:** Essential for live preview to prevent memory buildup
- **Dedicated queue:** Use a separate serial `DispatchQueue` for the delegate, never the main queue
- **sessionPreset:** Use `.medium` (480p) for webcam PIP; `.high` (720p) only if needed for export
- **Error handling:** Check `canAddInput`/`canAddOutput` before adding — never force
- **Cleanup:** Call `stopRunning()` on the session queue, not the main thread

---

## 3. AVAssetWriter — Video Encoding

### Setup

```swift
let writer = try AVAssetWriter(outputURL: outputURL, fileType: .mp4)

// Video input
let videoSettings: [String: Any] = [
    AVVideoCodecKey: AVVideoCodecType.h264,
    AVVideoWidthKey: width,
    AVVideoHeightKey: height,
    AVVideoCompressionPropertiesKey: [
        AVVideoAverageBitRateKey: 10_000_000,  // 10 Mbps
        AVVideoExpectedSourceFrameRateKey: 30,
        AVVideoProfileLevelKey: AVVideoProfileLevelH264HighAutoLevel
    ]
]
let videoInput = AVAssetWriterInput(mediaType: .video, outputSettings: videoSettings)
videoInput.expectsMediaDataInRealTime = true  // Critical for live recording

// Pixel buffer adaptor for raw frames
let adaptor = AVAssetWriterInputPixelBufferAdaptor(
    assetWriterInput: videoInput,
    sourcePixelBufferAttributes: [
        kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
        kCVPixelBufferWidthKey as String: width,
        kCVPixelBufferHeightKey as String: height
    ]
)

writer.add(videoInput)
writer.startWriting()
writer.startSession(atSourceTime: .zero)
```

### Thread Safety Warning

> **Critical:** It is NOT safe to call `finishWriting` concurrently with `appendSampleBuffer`. Use a serial dispatch queue for all writer operations. Allow 2 frames of latency for dispatch_async + appendSampleBuffer.

```swift
private let writerQueue = DispatchQueue(label: "com.frame.writer", qos: .userInitiated)

func appendFrame(_ pixelBuffer: CVPixelBuffer, at time: CMTime) {
    writerQueue.async { [weak self] in
        guard let self, self.videoInput.isReadyForMoreMediaData else { return }
        self.adaptor.append(pixelBuffer, withPresentationTime: time)
    }
}

func finish() async {
    await withCheckedContinuation { continuation in
        writerQueue.async { [weak self] in
            self?.videoInput.markAsFinished()
            self?.writer.finishWriting {
                continuation.resume()
            }
        }
    }
}
```

### Best Practices

- **expectsMediaDataInRealTime = true:** Always set for live capture; tells the writer to tolerate timing irregularities
- **Serial queue:** All append/finish operations on the same serial queue
- **isReadyForMoreMediaData:** Always check before appending — backpressure signal
- **markAsFinished():** Call on each input before `finishWriting()`
- **Error recovery:** Check `writer.status` and `writer.error` after operations

---

## 4. CoreImage & Metal — GPU Rendering

### CIContext — GPU vs Software

```swift
// For real-time rendering (webcam preview, effects)
let gpuContext = CIContext(options: [
    .useSoftwareRenderer: false,     // Use GPU
    .highQualityDownsample: false,   // Speed over quality for live
    .cacheIntermediates: false       // Reduce memory for live preview
])

// For export/final rendering (quality matters)
let exportContext = CIContext(options: [
    .useSoftwareRenderer: false,
    .highQualityDownsample: true
])
```

### CIImage to CGImage (for display)

```swift
// Render CIImage to CGImage for display
if let cgImage = ciContext.createCGImage(ciImage, from: ciImage.extent) {
    // Use cgImage for CALayer.contents or NSImage
    layer.contents = cgImage
}
```

### GPU-Backed Preview Rendering with CALayer

```swift
// Direct rendering to CALayer — bypasses NSImage/SwiftUI Image
class CIImageView: NSView {
    private let ciContext = CIContext(options: [.useSoftwareRenderer: false])

    override func draw(_ dirtyRect: NSRect) {
        guard let ciImage = currentImage,
              let cgImage = ciContext.createCGImage(ciImage, from: ciImage.extent) else { return }
        layer?.contents = cgImage
    }
}
```

### Best Practices

- **Never use software renderer** for real-time preview — GPU is 10-100x faster
- **Cache CIContext:** Create once, reuse for all renders — expensive to create
- **cacheIntermediates: false** for live preview to reduce memory
- **highQualityDownsample: true** only for export rendering
- **Metal backend:** CIContext defaults to Metal on macOS 10.13+ — no extra config needed
- **Thread safety:** CIContext is thread-safe; CIImage is immutable and safe to pass between threads

---

## 5. CoreVideo — CVDisplayLink

### Setup Pattern

```swift
private var displayLink: CVDisplayLink?

func startDisplayLink() {
    CVDisplayLinkCreateWithActiveCGDisplays(&displayLink)
    guard let displayLink else { return }

    CVDisplayLinkSetOutputCallback(displayLink, { (_, _, _, _, _, userInfo) -> CVReturn in
        let view = Unmanaged<CIImageView>.fromOpaque(userInfo!).takeUnretainedValue()
        DispatchQueue.main.async {
            view.needsDisplay = true  // Trigger redraw on next frame
        }
        return kCVReturnSuccess
    }, Unmanaged.passUnretained(self).toOpaque())

    CVDisplayLinkStart(displayLink)
}

func stopDisplayLink() {
    if let displayLink {
        CVDisplayLinkStop(displayLink)
    }
    displayLink = nil
}
```

### Best Practices

- **Main thread rendering:** CVDisplayLink callback runs on a background thread — always dispatch UI updates to main
- **takeUnretainedValue:** Use `Unmanaged.passUnretained` / `takeUnretainedValue` (not retained) to avoid retain cycles
- **Stop on dealloc:** Always `CVDisplayLinkStop` in `deinit` or when view is removed
- **Frame rate:** CVDisplayLink matches display refresh rate (60Hz typically); throttle if needed
- **Alternative (macOS 14+):** Consider `CADisplayLink` for newer deployments — simpler API, same functionality

---

## 6. SwiftUI + AppKit Integration

### NSViewRepresentable

```swift
struct MetalPreviewView: NSViewRepresentable {
    let frameBox: WebcamFrameBox

    func makeCoordinator() -> Coordinator {
        Coordinator(frameBox: frameBox)
    }

    func makeNSView(context: Context) -> CIImageView {
        let view = CIImageView(frameBox: frameBox)
        return view
    }

    func updateNSView(_ nsView: CIImageView, context: Context) {
        // Update view properties when SwiftUI state changes
        // WARNING: Never set frame/bounds directly — SwiftUI controls layout
    }

    static func dismantleNSView(_ nsView: CIImageView, coordinator: Coordinator) {
        nsView.stopDisplayLink()  // Cleanup
    }

    class Coordinator {
        let frameBox: WebcamFrameBox
        init(frameBox: WebcamFrameBox) { self.frameBox = frameBox }
    }
}
```

### Key Rules

- **Lifecycle:** `makeCoordinator()` → `makeNSView()` → `updateNSView()` → `dismantleNSView()`
- **Never set frame/bounds** on the managed NSView — SwiftUI controls layout
- **Coordinator** for delegation/target-action bridging
- **dismantleNSView** for cleanup (stop timers, display links, cancel subscriptions)

### NSVisualEffectView for Glass Effects

```swift
struct VisualEffectBackground: NSViewRepresentable {
    let material: NSVisualEffectView.Material
    let blendingMode: NSVisualEffectView.BlendingMode

    func makeNSView(context: Context) -> NSVisualEffectView {
        let view = NSVisualEffectView()
        view.material = material
        view.blendingMode = blendingMode
        view.state = .active
        return view
    }

    func updateNSView(_ nsView: NSVisualEffectView, context: Context) {
        nsView.material = material
        nsView.blendingMode = blendingMode
    }
}
```

---

## 7. Floating Panels (NSPanel)

### Reusable FloatingPanel Base Class

Based on patterns from Enchanted, NativeYoutube, Unlost, BoltAI, and HuggingChat:

```swift
class FloatingPanel: NSPanel {
    init(contentRect: NSRect, content: NSView) {
        super.init(
            contentRect: contentRect,
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )

        // Transparency
        isOpaque = false
        backgroundColor = .clear
        hasShadow = true

        // Floating behavior
        level = .floating
        isFloatingPanel = true
        hidesOnDeactivate = false

        // Multi-space
        collectionBehavior = [
            .canJoinAllSpaces,
            .fullScreenAuxiliary,
            .stationary
        ]

        // Screen capture exclusion
        sharingType = .none  // Invisible to ScreenCaptureKit

        // Non-activating
        isMovableByWindowBackground = true

        // Content
        contentView = content
    }

    // Allow keyboard input when focused
    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { false }
}
```

### Screen Capture Exclusion — Two Approaches

```swift
// Approach 1: sharingType = .none (Preferred — simple, automatic)
panel.sharingType = .none
// Panel is automatically invisible to ALL screen capture, including SCStream

// Approach 2: SCContentFilter(exceptingWindows:) (Explicit exclusion)
let panelWindows = overlayManager.allPanelWindows()  // [SCWindow]
let filter = SCContentFilter(
    display: targetDisplay,
    excludingWindows: panelWindows
)
// Must update filter when panels are added/removed
```

**Recommendation:** Use `sharingType = .none` as primary. It's simpler, automatic, and works with all capture APIs. Use `SCContentFilter` exclusion only as a fallback or for fine-grained control.

### Hosting SwiftUI Content in NSPanel

```swift
let hostingView = NSHostingView(rootView: RecordingToolbarView())
hostingView.frame = NSRect(x: 0, y: 0, width: 400, height: 60)

let panel = FloatingPanel(
    contentRect: hostingView.frame,
    content: hostingView
)
panel.center()
panel.orderFront(nil)
```

### Best Practices

- **`.nonactivatingPanel`:** Prevents panel from stealing focus from other apps
- **`.fullScreenAuxiliary`:** Panel remains visible when user enters fullscreen in another app
- **`.canJoinAllSpaces`:** Panel appears on all Spaces/desktops
- **`hidesOnDeactivate = false`:** Panel stays visible when app loses focus
- **`sharingType = .none`:** Exclude from screen capture
- **`canBecomeKey = true`:** Allow keyboard interaction within the panel
- **`canBecomeMain = false`:** Panel should never be the main window

---

## 8. Swift Concurrency

### Actor Isolation

```swift
// MainActor for UI state
@MainActor
@Observable
class AppState {
    var isRecording = false
    var currentProject: Project?

    // nonisolated for computed properties that don't access state
    nonisolated func appVersion() -> String { "1.0.0" }
}
```

### Atomicity Warning

> **Critical:** Actors guarantee safety from data races but NOT atomicity across suspension points. Each `await` is a potential interleaving point.

```swift
actor RecordingState {
    var frameCount = 0

    // BAD: Not atomic across await
    func process() async {
        let count = frameCount       // Read
        await heavyWork()            // Suspension point — other code may modify frameCount
        frameCount = count + 1       // Write — may overwrite concurrent changes
    }

    // GOOD: Complete reads/writes before awaiting
    func processCorrectly() async {
        frameCount += 1              // Atomic increment
        await heavyWork()            // Now safe
    }
}
```

### nonisolated(nonsending) — No Thread Hop

```swift
// Function stays on caller's executor — zero overhead
nonisolated(nonsending) func measure<T>(
    _ label: String,
    block: () async throws -> T
) async rethrows -> T {
    let start = ContinuousClock.now
    let result = try await block()
    print("\(label): \(ContinuousClock.now - start)")
    return result
}
```

### Sendable Conformance

```swift
// Immutable value types are Sendable automatically
struct RecordingConfig: Sendable {
    let captureType: CaptureType
    let frameRate: Int
}

// Mutable classes: use actor or @MainActor
@MainActor final class OverlayManager: Sendable {
    // All state access on MainActor
}

// For external types: nonisolated(unsafe) with external synchronization
nonisolated(unsafe) var sharedState: SomeType  // Only if you guarantee safety
```

### Task Patterns

```swift
// Structured concurrency
func startRecording() async throws {
    try await withThrowingTaskGroup(of: Void.self) { group in
        group.addTask { try await self.startScreenCapture() }
        group.addTask { try await self.startAudioCapture() }
        group.addTask { try await self.startWebcamCapture() }
        try await group.waitForAll()
    }
}

// Cancellation handling
func captureLoop() async {
    while !Task.isCancelled {
        guard let frame = await nextFrame() else { continue }
        process(frame)
    }
}
```

### Best Practices

- **@MainActor on UI classes:** All `@Observable` state classes should be `@MainActor`
- **nonisolated for pure functions:** Mark computed properties and helper functions that don't access state
- **No work across await:** Complete state mutations before suspension points
- **Structured concurrency:** Use `TaskGroup` for concurrent work with automatic cancellation
- **Check Task.isCancelled:** Always handle cancellation in long-running loops
- **Avoid global mutable state:** Use actor isolation or `@MainActor` instead

---

## 9. Combine Framework

### Subscriber Patterns

```swift
// Simple sink
let cancellable = publisher
    .sink { value in
        handleValue(value)
    }

// With error handling
let cancellable = publisher
    .sink(
        receiveCompletion: { completion in
            switch completion {
            case .finished: break
            case .failure(let error): handleError(error)
            }
        },
        receiveValue: { value in
            handleValue(value)
        }
    )

// Store cancellables
private var cancellables = Set<AnyCancellable>()

publisher
    .sink { value in /* ... */ }
    .store(in: &cancellables)
```

### Throttle & Debounce for Frame Events

```swift
// Throttle: Emit at most once per interval (for cursor position updates)
cursorPositionPublisher
    .throttle(for: .milliseconds(16), scheduler: DispatchQueue.main, latest: true)  // ~60fps
    .sink { position in updateCursor(position) }
    .store(in: &cancellables)

// Debounce: Wait for quiescence (for settings changes)
settingsPublisher
    .debounce(for: .seconds(0.5), scheduler: DispatchQueue.main)
    .sink { settings in applySettings(settings) }
    .store(in: &cancellables)
```

### Bridging Combine → async/await

```swift
// Convert publisher to async sequence
for await value in publisher.values {
    process(value)
}
```

### Best Practices

- **Cancel subscriptions:** Store in `Set<AnyCancellable>` — auto-cancelled on dealloc
- **Throttle for UI updates:** Use `.throttle` for high-frequency events (cursor, frame positions)
- **Debounce for actions:** Use `.debounce` for user-triggered changes (settings, search)
- **Scheduler awareness:** Use `DispatchQueue.main` for UI updates, `.global()` for processing
- **Prefer async/await:** For new code, prefer Swift concurrency over Combine where possible

---

## 10. @Observable State Management

### Migration from ObservableObject (macOS 14+)

```swift
// ❌ OLD (ObservableObject)
class AppState: ObservableObject {
    @Published var isRecording = false
    @Published var projects: [Project] = []
}

// ✅ NEW (@Observable)
@Observable
class AppState {
    var isRecording = false       // No @Published needed
    var projects: [Project] = []  // Auto-observed
}
```

### View Usage

```swift
// ❌ OLD
struct ContentView: View {
    @StateObject private var state = AppState()        // Declaration
    @EnvironmentObject var state: AppState             // Access
    .environmentObject(state)                          // Injection

// ✅ NEW
struct ContentView: View {
    @State private var state = AppState()              // Declaration
    @Environment(AppState.self) private var state       // Access
    .environment(state)                                // Injection
}
```

### Performance — Granular Observation

```swift
@Observable
class AppState {
    var isRecording = false
    var frameCount = 0
    var currentProject: Project?
}

// Only re-renders when isRecording changes — not frameCount
struct RecordingBadge: View {
    @Environment(AppState.self) var state
    var body: some View {
        if state.isRecording {  // Only this property is observed
            Circle().fill(.red)
        }
    }
}
```

### Best Practices

- **No @Published needed:** All stored properties are auto-observed with `@Observable`
- **Granular updates:** SwiftUI only re-renders when accessed properties change — more efficient than `@Published`
- **@State for ownership:** Use `@State` at declaration site (replaces `@StateObject`)
- **@Environment for access:** Use `@Environment(Type.self)` (replaces `@EnvironmentObject`)
- **Combine with @MainActor:** `@MainActor @Observable class` ensures thread safety for UI state

---

## 11. Error Handling Patterns

### Frame Error Convention

```swift
// Define domain-specific errors
enum FrameError: LocalizedError {
    case captureNotAuthorized
    case captureStreamFailed(underlying: Error)
    case webcamUnavailable
    case exportFailed(reason: String)
    case invalidConfiguration(String)

    var errorDescription: String? {
        switch self {
        case .captureNotAuthorized:
            return "Screen recording permission not granted"
        case .captureStreamFailed(let error):
            return "Screen capture failed: \(error.localizedDescription)"
        case .webcamUnavailable:
            return "No webcam available"
        case .exportFailed(let reason):
            return "Export failed: \(reason)"
        case .invalidConfiguration(let detail):
            return "Invalid configuration: \(detail)"
        }
    }
}
```

### Result Pattern for Callbacks

```swift
typealias FrameResult<T> = Result<T, FrameError>

func startCapture(completion: @escaping (FrameResult<Void>) -> Void) {
    // ...
}
```

### Best Practices

- **Never force unwrap (`!`)** in production code — use `guard let` or `if let`
- **LocalizedError:** Always implement `errorDescription` for user-facing errors
- **throws vs Result:** Use `throws` for async functions; `Result` for callbacks
- **Log errors:** Always log errors with context before propagating

---

## 12. Performance Guidelines

### Recording Pipeline

| Stage                     | Thread               | Priority         | Notes                                    |
| ------------------------- | -------------------- | ---------------- | ---------------------------------------- |
| Screen capture (SCStream) | Dedicated queue      | .userInteractive | Separate queues per output type          |
| Webcam capture            | Dedicated queue      | .userInitiated   | alwaysDiscardsLateVideoFrames = true     |
| Frame compositing         | Background queue     | .userInitiated   | CIContext GPU rendering                  |
| AVAssetWriter             | Serial queue         | .userInitiated   | Never call finishWriting concurrently    |
| Webcam preview            | CVDisplayLink → Main | .default         | Display link callback dispatches to main |
| UI updates                | Main thread          | .userInteractive | @MainActor on all UI state               |

### Memory Management

- **CVPixelBuffer:** Reuse via `CVPixelBufferPool` — avoid allocating per frame
- **CIImage:** Immutable — safe to pass between threads, but don't retain chains
- **CIContext:** Create once, reuse — expensive initialization
- **Cancellables:** Store in `Set<AnyCancellable>` — auto-cleaned on dealloc
- **CVDisplayLink:** Always stop in `deinit` — prevents dangling callbacks

### Thread Safety Rules

1. **SCStream outputs:** Each output type gets its own dispatch queue
2. **AVAssetWriter:** All operations on a single serial queue
3. **AVCaptureSession:** Call `startRunning()`/`stopRunning()` on session queue, not main
4. **CIContext:** Thread-safe — can share across threads
5. **CIImage:** Immutable — thread-safe
6. **NSPanel/NSView:** Main thread only (AppKit requirement)
7. **@Observable state:** Use `@MainActor` for thread safety

### Frame Drop Prevention

```swift
// 1. Check readiness before appending
guard writerInput.isReadyForMoreMediaData else {
    print("⚠️ Writer not ready — dropping frame")
    return
}

// 2. Discard late webcam frames
videoOutput.alwaysDiscardsLateVideoFrames = true

// 3. Use appropriate queue depth
streamConfig.queueDepth = 5  // Balance memory vs stalling

// 4. Monitor with signposts
import os.signpost
let log = OSLog(subsystem: "com.frame", category: "performance")
os_signpost(.begin, log: log, name: "processFrame")
// ... work ...
os_signpost(.end, log: log, name: "processFrame")
```

---

## Quick Reference Card

| What             | How                                   | Why                               |
| ---------------- | ------------------------------------- | --------------------------------- |
| Screen capture   | SCStream + separate queues            | High-performance, macOS native    |
| Webcam           | AVCaptureSession + .medium preset     | 480p sufficient for PIP           |
| Video encoding   | AVAssetWriter + serial queue          | Hardware-accelerated, thread-safe |
| GPU rendering    | CIContext(useSoftwareRenderer: false) | 10-100x faster than CPU           |
| Display sync     | CVDisplayLink → main thread           | Smooth frame delivery             |
| Floating panels  | NSPanel + sharingType = .none         | Non-activating, capture-invisible |
| State management | @MainActor @Observable                | Granular SwiftUI updates          |
| Concurrency      | TaskGroup + actors                    | Structured, cancellable           |
| Error handling   | throws + FrameError enum              | Never force unwrap                |
| Performance      | os_signpost + queue separation        | Profile, don't guess              |

---

## References

- [Apple ScreenCaptureKit Documentation](https://developer.apple.com/documentation/screencapturekit)
- [Apple SwiftUI Documentation](https://developer.apple.com/documentation/swiftui)
- [Apple AVFoundation Documentation](https://developer.apple.com/documentation/avfoundation)
- [Swift Migration Guide — Concurrency](https://www.swift.org/migration/documentation/swift-6-concurrency/)
- [Swift Async Algorithms](https://github.com/apple/swift-async-algorithms)
- Real-world patterns from: Enchanted, NativeYoutube, Unlost, BoltAI, OpenClaw
