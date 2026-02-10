# Beads PRD: Audio Waveform Visualization in Timeline

**Bead:** bd-1c0  
**Created:** 2026-02-10  
**Status:** Draft

## Bead Metadata

```yaml
depends_on: [bd-13v]
parallel: true
conflicts_with: []
blocks: []
estimated_hours: 3
```

---

## Problem Statement

### What problem are we solving?

Frame's timeline currently lacks audio visualization. Users cannot see the audio waveform, making it difficult to:

- Identify when audio starts/stops
- Edit videos with audio cues (e.g., syncing zoom effects to audio peaks)
- Verify that audio was actually recorded

Screen Studio and other professional screen recorders show orange/blue audio waveforms alongside the video track, providing immediate visual feedback about audio presence and intensity.

### Why now?

This was deferred from bd-13v (Editor UI improvements) due to requiring more complex work with audio buffers and Swift Charts. Now that other UI improvements are shipped, we can add this professional feature.

### Who is affected?

- **Primary users:** Content creators who edit videos with audio synchronization needs
- **Secondary users:** Users who want to verify audio was captured correctly

---

## Scope

### In-Scope

- Extract audio track from video file
- Generate waveform data (amplitude over time)
- Render waveform as orange/blue gradient in timeline
- Show waveform only for visible time range (viewport caching)
- Update waveform when playhead moves
- Hide waveform when audio is muted or absent

### Out-of-Scope

- Multi-track audio (system audio + microphone separately)
- Audio editing (cut, trim, fade)
- Real-time waveform generation during recording
- Spectrogram or frequency visualization
- Export waveform as separate image

---

## Proposed Solution

### Overview

Extract the audio track from the video file using AVAssetReader, generate amplitude samples at regular intervals, cache the data, and render it as an orange-to-blue gradient waveform in the timeline view using Swift Charts. The waveform updates as the user scrubs through the timeline.

### User Flow

1. User opens the Editor with a video that has audio
2. Timeline shows orange/blue audio waveform beneath the video track
3. As user scrubs, waveform updates to show current viewport
4. Peaks in waveform help identify audio events (clicks, speech, music)

---

## Requirements

### Functional Requirements

#### Waveform Generation

**Scenarios:**

- **WHEN** a video with audio is loaded **THEN** the audio waveform is generated and displayed
- **WHEN** the video has no audio track **THEN** no waveform is shown
- **WHEN** the audio is muted **THEN** the waveform is shown at reduced opacity (50%)
- **WHEN** the waveform generation completes **THEN** it is cached for the session

#### Waveform Rendering

**Scenarios:**

- **WHEN** user views the timeline **THEN** waveform is visible as orange/blue gradient bars
- **WHEN** user scrubs to a different time **THEN** waveform updates to show visible range
- **WHEN** user zooms the timeline **THEN** waveform resolution adjusts appropriately
- **WHEN** the playhead moves **THEN** waveform is smooth (no dropped frames)

#### Performance

**Scenarios:**

- **WHEN** timeline is scrubbed rapidly **THEN** waveform maintains 60fps
- **WHEN** video is longer than 10 minutes **THEN** only visible samples are rendered (viewport caching)

### Non-Functional Requirements

- **Performance:** Waveform generation <2 seconds for 1-minute video, timeline scrubbing at 60fps
- **Memory:** Audio samples cached in memory, cleared when video closed
- **Accessibility:** Waveform has sufficient contrast (orange/blue on dark background)
- **Compatibility:** Works on macOS 13.0+ (current minimum)

---

## Success Criteria

- [ ] Audio waveform appears in timeline for videos with audio
  - Verify: Build app, load video with audio, verify orange/blue waveform visible
- [ ] Waveform updates smoothly when scrubbing timeline
  - Verify: Scrub timeline rapidly, verify no stuttering or dropped frames
- [ ] No waveform shown for videos without audio
  - Verify: Load video without audio, verify no waveform area
- [ ] Waveform shown at reduced opacity when audio muted
  - Verify: Mute audio, verify waveform at 50% opacity
- [ ] Build passes with no errors
  - Verify: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`

---

## Technical Context

### Existing Patterns

- **Timeline View:** `Frame/Views/Editor/TimelineView.swift` - Main timeline with scrubber, trim handles, playhead
- **Playback Engine:** `Frame/Playback/PlaybackEngine.swift` - Manages AVPlayer and video loading
- **Audio Handling:** Currently uses AVPlayer for playback, no audio extraction
- **Swift Charts:** Available on macOS 13.0+ for data visualization

### Key Files

- `Frame/Views/Editor/TimelineView.swift` - Timeline UI, add waveform view here
- `Frame/Playback/PlaybackEngine.swift` - Video loading, trigger waveform generation
- `Frame/Playback/AudioWaveformGenerator.swift` - NEW: Extract and process audio

### Affected Files

```yaml
files:
  - Frame/Views/Editor/TimelineView.swift # Add waveform view
  - Frame/Playback/PlaybackEngine.swift # Trigger generation on video load
  - Frame/Playback/AudioWaveformGenerator.swift # NEW: Audio extraction
```

---

## Risks & Mitigations

| Risk                                         | Likelihood | Impact | Mitigation                                                     |
| -------------------------------------------- | ---------- | ------ | -------------------------------------------------------------- |
| Audio extraction slow for long videos        | Medium     | Medium | Progressive/async generation, show loading indicator           |
| Memory usage too high with large audio files | Medium     | Medium | Downsample to reasonable resolution (e.g., 1 sample per 100ms) |
| Swift Charts performance issues              | Low        | High   | Use viewport clipping, only render visible samples             |
| DRM-protected video audio unreadable         | Low        | Low    | Gracefully handle failure, show "Audio unavailable"            |

---

## Open Questions

| Question                                                 | Owner | Due Date    | Status |
| -------------------------------------------------------- | ----- | ----------- | ------ |
| What audio sample resolution? (e.g., 1 sample per 100ms) | TBD   | Before impl | Open   |
| Orange/blue gradient or solid color?                     | TBD   | Before impl | Open   |
| Show waveform above or below video track?                | TBD   | Before impl | Open   |

---

## Tasks

### Create AudioWaveformGenerator [backend]

Create AudioWaveformGenerator.swift to extract audio samples from video files using AVAssetReader and generate waveform data.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - Frame/Playback/AudioWaveformGenerator.swift
```

**Verification:**

- Unit test: Extract audio from sample video, verify sample count
- Verify samples are normalized to 0.0-1.0 range
- Verify generation completes in <2 seconds for 1-min video

### Add waveform data to PlaybackEngine [integration]

Update PlaybackEngine to trigger waveform generation when video loads and store the waveform data.

**Metadata:**

```yaml
depends_on: ["Create AudioWaveformGenerator"]
parallel: false
conflicts_with: []
files:
  - Frame/Playback/PlaybackEngine.swift
```

**Verification:**

- Load video with audio, verify waveform data available in PlaybackEngine
- Load video without audio, verify nil waveform data
- Verify data cleared when video closed

### Create waveform view in TimelineView [ui]

Add Swift Charts waveform visualization to TimelineView, showing orange/blue gradient bars for audio amplitude.

**Metadata:**

```yaml
depends_on: ["Add waveform data to PlaybackEngine"]
parallel: false
conflicts_with: []
files:
  - Frame/Views/Editor/TimelineView.swift
```

**Verification:**

- Build app, load video with audio
- Verify orange/blue waveform visible in timeline
- Scrub timeline, verify smooth updates
- Mute audio, verify waveform at 50% opacity

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

- **Sample resolution:** Start with 1 sample per 100ms (10 samples per second). For a 10-minute video = 6000 samples, well within memory limits.
- **Waveform style:** Orange (#FF6B35) to blue (#4ECDC4) gradient matches Screen Studio aesthetic
- **Viewport rendering:** Use Swift Charts `.chartXScale(domain:)` to show only visible time range
- **Async generation:** Use Task { } for waveform generation to avoid blocking UI
- **Caching:** Store AudioWaveform in PlaybackEngine, regenerate only when video changes
