# Beads PRD: AI Captions/Transcription Inspector with Whisper

**Bead:** bd-299  
**Created:** 2026-02-10  
**Status:** Draft

## Bead Metadata

```yaml
depends_on: []
parallel: true
conflicts_with: []
blocks: []
estimated_hours: 6
```

---

## Problem Statement

### What problem are we solving?

Frame lacks captions/transcription capabilities. Screen Studio offers AI-powered captions using OpenAI Whisper with options for model size, language, and custom prompts. Without captions, videos are less accessible and harder to follow for viewers who prefer reading or need accessibility support.

### Why now?

Captions are standard in professional video tools. Screen Studio's implementation is polished and user-friendly. Adding captions brings Frame closer to feature parity and improves accessibility significantly.

### Who is affected?

- **Primary users:** Content creators making tutorials, courses, demos
- **Secondary users:** Viewers who need captions for accessibility

---

## Scope

### In-Scope

- New Captions inspector panel
- Whisper AI integration (local processing)
- Model selector: Base/Small/Medium
- Language auto-detection + manual selection
- Custom prompt input for specialized vocabulary
- Generate transcript button with progress
- Caption display in preview canvas
- Caption size slider
- Edit transcript UI
- Export transcript option

### Out-of-Scope

- Real-time captioning during recording
- Third-party transcription services (AWS, Google)
- Caption styling (fonts, colors, backgrounds)
- Multi-language captions in one video
- Caption animation effects

---

## Proposed Solution

### Overview

Add a Captions inspector that integrates OpenAI Whisper running locally. Users can generate transcripts from recorded audio, adjust caption display size, and edit transcripts for accuracy. Captions appear as overlay on the preview canvas.

### User Flow

1. User opens Captions inspector
2. Selects AI model (Base/Small/Medium)
3. Reviews auto-detected language (or changes it)
4. Optionally enters custom prompt for product names
5. Clicks "Generate Transcript"
6. Waits for processing (progress shown)
7. Sees captions appear on video
8. Adjusts caption size if needed
9. Clicks "Edit Transcript" to fix any errors
10. Exports transcript as .srt if needed

---

## Requirements

### Functional Requirements

#### Caption Generation

**Scenarios:**

- **WHEN** user clicks Generate Transcript **THEN** Whisper processes audio
- **WHEN** processing **THEN** progress indicator shows
- **WHEN** no microphone audio recorded **THEN** button disabled with explanation
- **WHEN** model is Base **THEN** faster processing, less accuracy
- **WHEN** model is Medium **THEN** slower processing, more accuracy

#### Language Support

**Scenarios:**

- **WHEN** video loaded **THEN** language auto-detected from audio
- **WHEN** user changes language **THEN** new transcript generated
- **WHEN** language not detected **THEN** defaults to English

#### Custom Prompts

**Scenarios:**

- **WHEN** user enters custom prompt **THEN** used during transcription
- **WHEN** prompt contains product names **THEN** recognized correctly in transcript

#### Caption Display

**Scenarios:**

- **WHEN** transcript generated **THEN** captions appear on preview canvas
- **WHEN** caption size slider adjusted **THEN** text size changes
- **WHEN** captions enabled toggle off **THEN** captions hidden
- **WHEN** video plays **THEN** captions sync with audio timing

#### Transcript Editing

**Scenarios:**

- **WHEN** user clicks Edit Transcript **THEN** text editor opens
- **WHEN** user edits text **THEN** changes saved and captions updated
- **WHEN** user exports transcript **THEN** .srt file downloaded

### Non-Functional Requirements

- **Performance:** Base model completes in <2x video duration
- **Privacy:** All processing local, no audio sent to cloud
- **Accessibility:** Captions meet WCAG 2.1 AA standards
- **Compatibility:** Requires macOS 13.0+ (already minimum)

---

## Success Criteria

- [ ] Captions inspector shows model/language/prompt options
  - Verify: Open Inspector, see all controls
- [ ] Generate Transcript processes audio with progress
  - Verify: Click generate, see progress, transcript created
- [ ] Captions appear on preview canvas
  - Verify: Play video, see captions synced
- [ ] Caption size slider adjusts text size
  - Verify: Drag slider, caption size changes
- [ ] Edit Transcript opens editor
  - Verify: Click edit, editor opens, changes save
- [ ] Export Transcript saves .srt
  - Verify: Click export, .srt file saved
- [ ] Build passes with no errors
  - Verify: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`

---

## Technical Context

### Existing Patterns

- **Inspector Panels:** All inspectors in `Frame/Views/Editor/Inspector/` follow same pattern with @Binding effects.
- **Whisper:** OpenAI Whisper cpp available as Swift package or can be embedded.
- **Audio Extraction:** `AudioWaveformGenerator.swift` already extracts audio using AVAssetReader.
- **Preview Overlays:** `KeystrokeOverlayView.swift` shows how to overlay text on preview.

### Key Files

- `Frame/Views/Editor/Inspector/CaptionsInspector.swift` - NEW inspector panel
- `Frame/Captions/WhisperTranscription.swift` - NEW transcription engine
- `Frame/Captions/CaptionOverlayView.swift` - NEW caption display
- `Frame/Views/Editor/PreviewCanvas.swift` - Add caption overlay
- `Frame/Models/Project.swift` - Add caption settings to EffectsConfig

### Affected Files

```yaml
files:
  - Frame/Views/Editor/Inspector/CaptionsInspector.swift # NEW
  - Frame/Captions/WhisperTranscription.swift # NEW
  - Frame/Captions/CaptionOverlayView.swift # NEW
  - Frame/Views/Editor/PreviewCanvas.swift # Add overlay
  - Frame/Models/Project.swift # Caption settings
```

---

## Risks & Mitigations

| Risk                              | Likelihood | Impact | Mitigation                                 |
| --------------------------------- | ---------- | ------ | ------------------------------------------ |
| Whisper binary size large         | High       | Medium | Use CoreML models, smaller footprint       |
| Processing time too slow          | Medium     | High   | Offer Base model default, async processing |
| Memory usage during transcription | Medium     | Medium | Process in chunks, not entire audio        |
| Language detection inaccurate     | Low        | Low    | Allow manual override                      |

---

## Tasks

### Create Whisper transcription engine [backend]

Integrate Whisper with Swift wrapper for local transcription.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - Frame/Captions/WhisperTranscription.swift
```

**Verification:**

- Build app, Whisper loads
- Test transcription on sample audio
- Verify different models work

### Add caption data model [backend]

Create CaptionSegment struct and add caption settings to EffectsConfig.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - Frame/Models/Project.swift
```

**Verification:**

- CaptionSegment struct created
- EffectsConfig has caption properties
- Encoding/decoding works

### Create Captions inspector [ui]

Build inspector panel with model selector, language picker, prompt input, generate button.

**Metadata:**

```yaml
depends_on: ["Create Whisper transcription engine"]
parallel: false
conflicts_with: []
files:
  - Frame/Views/Editor/Inspector/CaptionsInspector.swift
```

**Verification:**

- Open Inspector, see all controls
- Model selector works
- Language picker works
- Generate button triggers transcription

### Add caption overlay to preview [ui]

Render captions on preview canvas with timing sync.

**Metadata:**

```yaml
depends_on: ["Add caption data model"]
parallel: false
conflicts_with: []
files:
  - Frame/Captions/CaptionOverlayView.swift
  - Frame/Views/Editor/PreviewCanvas.swift
```

**Verification:**

- Captions visible on preview
- Sync with audio timing
- Size slider adjusts text

### Create transcript editor [ui]

Build text editor for correcting transcription errors.

**Metadata:**

```yaml
depends_on: ["Create Captions inspector"]
parallel: false
conflicts_with: []
files:
  - Frame/Views/Editor/TranscriptEditor.swift
```

**Verification:**

- Edit button opens editor
- Changes save correctly
- Export to .srt works
