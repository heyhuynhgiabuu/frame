# Phase 2: Core Recording Implementation

**Bead:** bd-3vd  
**Created:** 2026-02-05  
**Status:** Draft

## Bead Metadata

```yaml
depends_on: [] # Phase 1 is complete
parallel: true # Tasks can run in parallel where dependencies allow
conflicts_with: []
blocks: [] # Phase 3 will depend on this
estimated_hours: 40 # 4 weeks of development
```

---

## Problem Statement

### What problem are we solving?

Frame currently has a basic UI shell and project structure, but lacks the core screen recording functionality that makes it a screen recorder. Users cannot actually capture their screen, record audio, or export videos. This is the foundational feature that makes Frame useful.

### Why now?

Phase 1 (Foundation) is complete with:

- ✅ Monorepo structure
- ✅ Basic iced.rs UI with state management
- ✅ Project file format
- ✅ Error handling framework

Without Phase 2, Frame is just a UI prototype. Screen recording is the primary value proposition.

### Who is affected?

- **Primary users:** Developers who need to create product demos, tutorials, and documentation
- **Secondary users:** Content creators, educators, and team leads sharing screen recordings

---

## Scope

### In-Scope

- [x] ScreenCaptureKit integration for macOS screen capture
- [x] Audio capture (microphone + system audio via BlackHole)
- [x] Basic timeline UI for reviewing recordings
- [x] MP4 export with H.264/H.265 encoding
- [x] Recording controls (start, stop, pause, resume)
- [x] Real-time preview during recording
- [x] Project auto-save during recording

### Out-of-Scope

- [ ] Cursor zoom and smoothing effects (Phase 3)
- [ ] Webcam overlay (Phase 3)
- [ ] Advanced timeline editing (trim, cut, split) (Phase 3)
- [ ] Cloud sync and sharing (Phase 4)
- [ ] Windows/Linux support (future phases)
- [ ] Hardware acceleration optimization (future)

---

## Proposed Solution

### Overview

Implement native macOS screen recording using ScreenCaptureKit, capture system and microphone audio using CoreAudio with BlackHole virtual audio driver, and encode to MP4 using ffmpeg. Provide a simple timeline UI for reviewing recordings before export.

### User Flow

1. **User opens Frame app** → Sees main window with "Start Recording" button
2. **User clicks "Start Recording"** → App requests screen recording permission
3. **Permission granted** → Recording starts, timer shows elapsed time
4. **User clicks "Stop Recording"** → Recording stops, timeline UI appears
5. **User reviews recording** → Can play/pause preview
6. **User clicks "Export"** → Exports to MP4 file
7. **Export complete** → File opens in Finder or user shares

---

## Requirements

### Functional Requirements

#### Screen Capture

Native macOS screen recording with ScreenCaptureKit.

**Scenarios:**

- **WHEN** user clicks "Start Recording" AND has permission THEN recording starts immediately
- **WHEN** user clicks "Stop Recording" THEN recording stops and saves to project
- **WHEN** screen recording permission is denied THEN show permission instructions dialog
- **WHEN** recording is active THEN show elapsed time and red recording indicator
- **WHEN** disk space is low (<1GB) THEN warn user and stop recording automatically

#### Audio Capture

Capture both microphone and system audio simultaneously.

**Scenarios:**

- **WHEN** recording starts AND microphone is available THEN microphone audio is captured
- **WHEN** recording starts AND BlackHole is installed THEN system audio is captured
- **WHEN** BlackHole is not installed THEN record without system audio and show setup instructions
- **WHEN** user mutes microphone THEN microphone audio is not captured
- **WHEN** audio device is disconnected mid-recording THEN continue recording video only

#### Timeline UI

Basic timeline for reviewing recordings.

**Scenarios:**

- **WHEN** recording stops THEN timeline appears with video preview
- **WHEN** user clicks play THEN preview plays from current position
- **WHEN** user clicks pause THEN preview pauses
- **WHEN** user drags timeline scrubber THEN preview jumps to that position
- **WHEN** user clicks "Export" THEN export dialog opens

#### Export

Export recordings to MP4 format.

**Scenarios:**

- **WHEN** user clicks "Export" AND selects location THEN video exports to that location
- **WHEN** export is in progress THEN progress bar shows completion percentage
- **WHEN** export completes THEN success notification appears
- **WHEN** export fails THEN error message appears with retry option
- **WHEN** user selects H.265 codec AND device supports it THEN use hardware acceleration

### Non-Functional Requirements

- **Performance:**
  - 1080p60 recording with <10% CPU usage on M1 Mac
  - <100ms latency for preview
  - Export 1-minute 1080p video in <30 seconds
- **Security:**
  - No cloud upload without explicit user consent
  - Project files stored locally only
  - Screen recording permission handled per Apple guidelines
- **Compatibility:**
  - macOS 12.3+ (Monterey) for ScreenCaptureKit
  - BlackHole 0.5.0+ for system audio
- **Reliability:**
  - Auto-save project every 30 seconds during recording
  - Graceful handling of out-of-disk-space errors
  - Recovery from unexpected crashes

---

## Success Criteria

- [ ] User can start/stop screen recording
  - Verify: `cargo run` in apps/desktop, click "Start Recording", verify timer increments
- [ ] Recording captures screen content correctly
  - Verify: Record 10 seconds, stop, verify video file exists and plays correctly
- [ ] Audio is captured (microphone + system)
  - Verify: Record with audio, export, verify audio plays in exported video
- [ ] Timeline UI shows recording preview
  - Verify: After recording stops, timeline appears with playable preview
- [ ] Export to MP4 works
  - Verify: Click "Export", select location, verify MP4 file is created and playable
- [ ] Project auto-saves during recording
  - Verify: Check project directory for auto-save files during recording

---

## Technical Context

### Existing Patterns

- **State Management:** `apps/desktop/src/app.rs` uses iced.rs Elm architecture with AppState enum
- **Error Handling:** `packages/core/src/error.rs` defines FrameError with thiserror
- **Project Structure:** `packages/core/src/project.rs` defines Project with auto-save
- **Capture Abstraction:** `packages/core/src/capture/mod.rs` defines ScreenCapture trait

### Key Files

- `packages/core/src/capture/platform.rs` - ScreenCaptureKit implementation (stub exists)
- `packages/core/src/encoder.rs` - ffmpeg video encoding (stub exists)
- `apps/desktop/src/ui/main.rs` - Main UI views (Idle, Recording, Preview, Exporting)
- `apps/desktop/src/app.rs` - Application state machine

### Affected Files

```yaml
files:
  - packages/core/src/capture/platform.rs # ScreenCaptureKit implementation
  - packages/core/src/capture/mod.rs # Audio capture integration
  - packages/core/src/encoder.rs # ffmpeg encoding implementation
  - packages/core/src/project.rs # Auto-save during recording
  - apps/desktop/src/app.rs # Recording state management
  - apps/desktop/src/ui/main.rs # Timeline UI
  - apps/desktop/src/ui/timeline.rs # New timeline component
  - apps/desktop/Cargo.toml # Add audio dependencies
```

---

## Risks & Mitigations

| Risk                             | Likelihood | Impact | Mitigation                                                        |
| -------------------------------- | ---------- | ------ | ----------------------------------------------------------------- |
| ScreenCaptureKit API complexity  | High       | Medium | Start with basic implementation, iterate; use Apple's sample code |
| Audio sync issues                | Medium     | High   | Use timestamp-based synchronization; test extensively             |
| ffmpeg integration complexity    | Medium     | Medium | Use ffmpeg-next crate; start with basic H.264                     |
| Performance issues on older Macs | Medium     | Medium | Target M1+ first; add performance settings                        |
| BlackHole installation friction  | High       | Medium | Provide clear setup instructions; make optional                   |
| Disk space exhaustion            | Low        | High   | Monitor disk space; auto-stop at 1GB free                         |

---

## Open Questions

| Question                              | Owner | Due Date       | Status |
| ------------------------------------- | ----- | -------------- | ------ |
| Should we support multiple displays?  | TBD   | Phase 2 Week 2 | Open   |
| What's the max recording duration?    | TBD   | Phase 2 Week 1 | Open   |
| Do we need pause/resume or just stop? | TBD   | Phase 2 Week 1 | Open   |

---

## Tasks

### 1. ScreenCaptureKit Integration [capture]

Implement native macOS screen capture using ScreenCaptureKit.

**Metadata:**

```yaml
depends_on: []
parallel: false
conflicts_with: []
files:
  - packages/core/src/capture/platform.rs
  - packages/core/src/capture/mod.rs
```

**Verification:**

- `cargo test -p frame-core` passes
- Screen recording starts and stops without errors
- Captured frames are written to disk

### 2. Audio Capture Implementation [audio]

Implement microphone and system audio capture using CoreAudio.

**Metadata:**

```yaml
depends_on: ["ScreenCaptureKit Integration"]
parallel: false
conflicts_with: []
files:
  - packages/core/src/capture/mod.rs
  - packages/core/Cargo.toml
```

**Verification:**

- Microphone audio is captured during recording
- System audio is captured when BlackHole is installed
- Audio syncs with video (test with clap)

### 3. Video Encoding with ffmpeg [encoding]

Implement MP4 encoding using ffmpeg-next crate.

**Metadata:**

```yaml
depends_on: ["ScreenCaptureKit Integration"]
parallel: true
conflicts_with: []
files:
  - packages/core/src/encoder.rs
  - packages/core/Cargo.toml
```

**Verification:**

- Raw frames are encoded to H.264 MP4
- Audio is muxed into video
- Output file plays correctly in QuickTime

### 4. Recording Controls UI [ui]

Update UI for recording start/stop with permission handling.

**Metadata:**

```yaml
depends_on: ["ScreenCaptureKit Integration"]
parallel: true
conflicts_with: []
files:
  - apps/desktop/src/app.rs
  - apps/desktop/src/ui/main.rs
```

**Verification:**

- "Start Recording" button initiates capture
- Permission dialog appears if needed
- Recording indicator shows during capture
- "Stop Recording" stops capture and shows timeline

### 5. Timeline UI Component [ui]

Create timeline view for reviewing recordings.

**Metadata:**

```yaml
depends_on: ["Recording Controls UI"]
parallel: false
conflicts_with: []
files:
  - apps/desktop/src/ui/timeline.rs
  - apps/desktop/src/ui/mod.rs
  - apps/desktop/src/app.rs
```

**Verification:**

- Timeline appears after recording stops
- Play/pause controls work
- Scrubber jumps to position
- Current time is displayed

### 6. Export Dialog and Progress [export]

Implement export functionality with progress indication.

**Metadata:**

```yaml
depends_on:
  - "Video Encoding with ffmpeg"
  - "Timeline UI Component"
parallel: false
conflicts_with: []
files:
  - apps/desktop/src/app.rs
  - apps/desktop/src/ui/main.rs
  - packages/core/src/project.rs
```

**Verification:**

- Export dialog opens with location picker
- Progress bar shows during export
- Success notification appears
- Exported file opens in Finder

### 7. Project Auto-Save [persistence]

Implement auto-save during recording to prevent data loss.

**Metadata:**

```yaml
depends_on: ["ScreenCaptureKit Integration"]
parallel: true
conflicts_with: []
files:
  - packages/core/src/project.rs
  - apps/desktop/src/app.rs
```

**Verification:**

- Project auto-saves every 30 seconds during recording
- Recovery works if app crashes mid-recording
- Auto-save files are cleaned up after successful export

### 8. Error Handling and Edge Cases [reliability]

Implement comprehensive error handling for all failure modes.

**Metadata:**

```yaml
depends_on:
  - "ScreenCaptureKit Integration"
  - "Audio Capture Implementation"
  - "Video Encoding with ffmpeg"
parallel: false
conflicts_with: []
files:
  - packages/core/src/error.rs
  - apps/desktop/src/app.rs
```

**Verification:**

- Permission denied shows helpful instructions
- Disk full stops recording gracefully
- Audio device disconnect handled
- All errors show user-friendly messages

### 9. Integration Testing [testing]

Create integration tests for end-to-end recording flow.

**Metadata:**

```yaml
depends_on:
  - "ScreenCaptureKit Integration"
  - "Audio Capture Implementation"
  - "Video Encoding with ffmpeg"
parallel: false
conflicts_with: []
files:
  - packages/core/tests/recording_test.rs
  - apps/desktop/tests/e2e_test.rs
```

**Verification:**

- `cargo test --workspace` passes
- E2E test records 5-second video and verifies output
- Audio sync test passes

### 10. Documentation [docs]

Create API documentation and setup guide.

**Metadata:**

```yaml
depends_on:
  - "ScreenCaptureKit Integration"
  - "Audio Capture Implementation"
parallel: true
conflicts_with: []
files:
  - docs/API.md
  - docs/SETUP.md
  - README.md
```

**Verification:**

- API docs cover all public functions
- Setup guide explains BlackHole installation
- README has updated build instructions

---

## Dependency Legend

| Field            | Purpose                                           | Example                                    |
| ---------------- | ------------------------------------------------- | ------------------------------------------ |
| `depends_on`     | Must complete before this task starts             | `["Setup database", "Create schema"]`      |
| `parallel`       | Can run concurrently with other parallel tasks    | `true` / `false`                           |
| `conflicts_with` | Cannot run in parallel (same files)               | `["Update config"]`                        |
| `files`          | Files this task modifies (for conflict detection) | `["src/db/schema.ts", "src/db/client.ts"]` |

---

## Notes

- **ScreenCaptureKit** requires macOS 12.3+ (Monterey)
- **BlackHole** is required for system audio - provide setup instructions
- **ffmpeg** must be installed on development machines
- Consider using **cpal** for cross-platform audio in future
- Hardware acceleration for H.265 on Apple Silicon is a nice-to-have

---

## Next Steps

After this PRD is approved:

1. Convert to executable tasks: `skill({ name: "prd-task" })`
2. Create child beads for each task
3. Begin implementation with Task 1: ScreenCaptureKit Integration
