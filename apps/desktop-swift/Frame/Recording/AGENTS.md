# Recording

Screen, webcam, cursor, and keystroke capture pipeline.

## Where to Look

| Task                    | File                   | Notes                                             |
| ----------------------- | ---------------------- | ------------------------------------------------- |
| Screen capture logic    | `ScreenRecorder.swift` | SCStream → AVAssetWriter, webcam compositing      |
| Recording orchestration | `RecordingCoordinator` | Entry point — starts/stops all recorders together |
| Webcam capture          | `WebcamCaptureEngine`  | AVCaptureSession, publishes via `WebcamFrameBox`  |
| Mouse tracking          | `CursorRecorder`       | Records cursor positions, saves `.cursor.json`    |
| Keyboard capture        | `KeystrokeRecorder`    | Records key events, saves `.keystrokes.json`      |

## Architecture

```
RecordingCoordinator (orchestrator, @MainActor)
├── ScreenRecorder       → SCStream + AVAssetWriter
│   └── RecordingStreamOutput (SCStreamOutput delegate, private)
│       └── webcamFrameProvider  → reads from WebcamFrameBox
├── CursorRecorder       → mouse position tracking
└── KeystrokeRecorder    → keyboard event recording
```

## Threading

| Component             | Queue                                   | QoS                |
| --------------------- | --------------------------------------- | ------------------ |
| SCStream video        | `.global(qos: .userInitiated)` (shared) | `.userInitiated`   |
| SCStream audio        | `.global(qos: .userInitiated)` (shared) | `.userInitiated`   |
| AVCaptureSession      | `"com.frame.webcam-output"` (dedicated) | `.userInteractive` |
| AVAssetWriter appends | inline on SCStream callback queue       | —                  |
| State updates         | `@MainActor`                            | —                  |

**Note**: SCStream currently uses shared `.global()` queues, not dedicated per-output queues. Microphone capture is defined in `RecordingStreamOutput`'s switch statement but not wired as a stream output — only `.screen` and `.audio` outputs are added.

## Webcam Compositing Flow

1. `WebcamCaptureEngine.captureOutput` → stores `CIImage` in `WebcamFrameBox` (NSLock-guarded)
2. `RecordingCoordinator.startRecording` validates frame freshness (≤0.5s) → sets `webcamFrameProvider` closure on `ScreenRecorder`
3. `RecordingStreamOutput.handleVideoSample` reads provider → composites webcam via CIImage (scale → crop → mask → position → composite)
4. Grace window: 0.15s — uses cached frame if provider returns nil briefly

## Anti-Patterns

- **Never** call `startRunning()`/`stopRunning()` on main thread — use the session's dedicated queue
- **Never** create CIContext per-frame — `RecordingStreamOutput` creates once in init
- **Never** skip `isReadyForMoreMediaData` before `append()` — backpressure signal from AVAssetWriter
- **Never** set `alwaysDiscardsLateVideoFrames = false` on webcam output — causes memory buildup
- **Never** use `beginConfiguration()`/`commitConfiguration()` for single-property changes — they're for atomic multi-property changes

## Key Details

- Output format: `.mov` with H.264 video (bitrate = width × height × 8) + AAC audio (48kHz, 192kbps)
- Timestamps retimed to start from zero via `retimeSampleBuffer(offsetBy:)`
- Videos saved to `~/Movies/Frame Recordings/Frame_YYYY-MM-DD_HH-mm-ss.mov`
- `WebcamOverlayConfig`: position (4 corners), size (0.1–0.4 relative), shape (circle/roundedRect), padding
- `RecordingError.webcamFrameUnavailable` thrown if webcam enabled but no fresh frame at start
