# Beads PRD: Editor UI Improvements

**Bead:** bd-13v  
**Created:** 2026-02-10  
**Status:** Draft

## Bead Metadata

```yaml
depends_on: [] # Can start immediately
parallel: true # UI components can be developed in parallel
conflicts_with: [] # No current conflicts
blocks: [] # No dependent beads
estimated_hours: 6 # Medium effort, high impact
```

---

## Problem Statement

### What problem are we solving?

Frame's editor UI looks amateur compared to professional tools like Screen Studio. Through visual analysis of both apps side-by-side, we've identified specific deficiencies:

1. **Missing reset affordances** — Users cannot easily revert slider values to defaults, creating anxiety about experimentation
2. **Flat, 2D panels** — No glassmorphism depth; looks dated compared to modern macOS creative tools
3. **Bright green accent color** — Feels unprofessional and visually jarring; should use subtle purple/blue tones
4. **No audio waveform visualization** — Timeline lacks the professional polish of showing actual audio amplitude
5. **Sparse information density** — Generous padding wastes space; controls feel disconnected
6. **Color-only presets** — Background presets show swatches instead of visual wallpaper thumbnails
7. **Binary tab navigation** — Record/Edit toggle is limiting; needs mode pills like Screen Studio
8. **No timeline zoom controls** — Users cannot zoom in/out of the timeline for precise editing

The cumulative effect is that Frame feels like a "toy" app rather than a professional screen recording tool, which will hurt adoption among serious content creators.

### Why now?

Screen Studio has set the visual bar for modern screen recorders. Users comparing Frame to Screen Studio will immediately notice the UI quality gap. Before marketing or feature expansion, we need visual parity to compete.

### Who is affected?

- **Primary users:** Content creators who use screen recording professionally (YouTubers, educators, product marketers)
- **Secondary users:** Casual users who will perceive higher quality = more trustworthy tool

---

## Scope

### In-Scope (High Impact, Low/Medium Effort)

1. **Reset buttons on sliders** — Add "Reset" text button next to each SliderRow
2. **Reduce panel padding** — Tighten spacing from ~16px to ~8-12px in inspector panel
3. **Change accent color** — Move from bright green to subtle purple/blue (#7C7CFF or #6366F1)
4. **Glassmorphism panels** — Add NSVisualEffectView blur to inspector sidebar
5. **Show preset thumbnails** — Load actual wallpaper images in preset grid

### In-Scope (High Impact, Medium Effort)

6. **Audio waveform visualization** — Render audio amplitude in timeline using Swift Charts
7. **Embed webcam in preview** — Float webcam overlay within the video preview

### Out-of-Scope

- **Timeline zoom controls** — Complex new interaction pattern, defer to v2
- **Liquid Glass materials** — Requires macOS Tahoe+; Frame targets macOS 13+
- **Custom Metal shaders** — Overkill for current scope; use system materials
- **Keyboard shortcut system** — Separate feature, not UI polish

---

## Proposed Solution

### Overview

Transform Frame's editor from flat and utilitarian to layered and professional by implementing Screen Studio's proven UI patterns: glassmorphism depth, reset affordances, audio visualization, and refined spacing/color.

### User Flow

1. **Opening editor** — User sees a blurred glass sidebar that lets the desktop peek through
2. **Adjusting padding** — User drags slider, sees "Reset" button appear; clicks to revert
3. **Selecting background** — User sees actual wallpaper thumbnails, not just color swatches
4. **Editing timeline** — Orange/blue audio waveform visible alongside video track
5. **Previewing with webcam** — Webcam appears as floating overlay in corner of preview

---

## Requirements

### Functional Requirements

#### Reset Buttons

**Scenarios:**

- **WHEN** user adjusts any slider from default value **THEN** a "Reset" button appears to the right of the value display
- **WHEN** user clicks "Reset" **THEN** slider returns to default value and button disappears
- **WHEN** slider is already at default **THEN** no Reset button is shown

#### Glassmorphism Panels

**Scenarios:**

- **WHEN** editor is open **THEN** inspector panel shows subtle blur effect letting background show through
- **WHEN** window moves or resizes **THEN** blur updates dynamically
- **WHEN** on macOS 13+ **THEN** use `.ultraThinMaterial` with proper blending

#### Audio Waveform

**Scenarios:**

- **WHEN** video with audio is loaded **THEN** timeline shows audio amplitude as waveform
- **WHEN** timeline is scrubbed **THEN** waveform renders only visible portion (viewport caching)
- **WHEN** audio is muted **THEN** waveform shows at reduced opacity (50%)

#### Webcam Overlay

**Scenarios:**

- **WHEN** webcam is enabled in settings **THEN** preview shows webcam as floating corner overlay
- **WHEN** user drags webcam overlay **THEN** it repositions within preview bounds
- **WHEN** recording starts **THEN** webcam overlay position is captured in final output

### Non-Functional Requirements

- **Performance:** Waveform rendering must not drop timeline scrubbing below 60fps
- **Accessibility:** Reset buttons must have minimum 44pt touch target; support VoiceOver
- **Compatibility:** Glassmorphism works on macOS 13.0+ (current minimum)

---

## Success Criteria

- [ ] Reset buttons appear on all inspector sliders (Padding, Corners, Shadow, etc.)
  - Verify: Build app, open Editor, adjust any slider, confirm Reset button visible
- [ ] Inspector panel has glassmorphism blur effect
  - Verify: Build app, open Editor, verify sidebar shows blur behind it
- [ ] Accent color changed from green to purple/blue
  - Verify: Check slider thumbs, value labels, active states use new accent
- [ ] Audio waveform visible in timeline
  - Verify: Load video with audio, confirm orange/blue waveform appears in timeline
- [ ] Preset grid shows wallpaper thumbnails
  - Verify: Open Background inspector, confirm 10 preset thumbnails visible
- [ ] Webcam floats as overlay in preview
  - Verify: Enable webcam, confirm it appears as draggable overlay in preview
- [ ] Build passes with no errors
  - Verify: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`

---

## Technical Context

### Existing Patterns

- **Slider Component:** `SliderRow` in `Frame/Views/Editor/Inspector/InspectorComponents.swift` — VStack with label, value, and Slider
- **Glass Effect:** `VisualEffectBackground` wrapper exists; `EditorView` uses `.hudWindow` material
- **Inspector Panel:** Right sidebar in `EditorView.swift` uses `HSplitView` with 280px width
- **Color Model:** `CodableColor` in `Models/Project.swift` — stores color data
- **Effects Config:** `EffectsConfig` passed via `Binding` to all inspector views

### Key Files

- `Frame/Views/Editor/EditorView.swift` — Main editor with HSplitView, inspector panel
- `Frame/Views/Editor/Inspector/InspectorComponents.swift` — `SliderRow` component
- `Frame/Views/Editor/Inspector/BackgroundInspector.swift` — Background presets panel
- `Frame/Views/Editor/TimelineView.swift` — Timeline with scrubber (assumed name)
- `Frame/App/AppState.swift` — Global state including `EffectsConfig`
- `Frame/Views/Shared/VisualEffectBackground.swift` — Glassmorphism wrapper

### Affected Files

```yaml
files:
  - Frame/Views/Editor/Inspector/InspectorComponents.swift # SliderRow with Reset
  - Frame/Views/Editor/EditorView.swift # Glassmorphism on sidebar
  - Frame/Views/Editor/Inspector/BackgroundInspector.swift # Thumbnail presets
  - Frame/Views/Editor/TimelineView.swift # Audio waveform (or create)
  - Frame/Views/Editor/PreviewCanvas.swift # Webcam overlay
  - Frame/Utilities/Colors.swift # Or create accent color extension
  - Frame/Models/EffectsConfig.swift # Default values for reset
```

---

## Risks & Mitigations

| Risk                                                | Likelihood | Impact | Mitigation                                        |
| --------------------------------------------------- | ---------- | ------ | ------------------------------------------------- |
| Audio waveform hurts performance                    | Medium     | High   | Use viewport caching; only render visible samples |
| Glassmorphism looks wrong on light mode             | Low        | Medium | Test both modes; add fallback to solid color      |
| Webcam overlay interferes with preview interactions | Medium     | Medium | Make overlay draggable but not blocking clicks    |
| Thumbnail loading is slow                           | Low        | Low    | Cache thumbnails; async load with placeholder     |

---

## Open Questions

| Question                                      | Owner | Due Date    | Status |
| --------------------------------------------- | ----- | ----------- | ------ |
| What are the default values for each slider?  | TBD   | Before impl | Open   |
| Do we have sample wallpapers for thumbnails?  | TBD   | Before impl | Open   |
| Should Reset buttons show on hover or always? | TBD   | Before impl | Open   |

---

## Tasks

### Add Reset buttons to SliderRow component [ui]

SliderRow displays a "Reset" button when value differs from default, clicking it reverts to default.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - Frame/Views/Editor/Inspector/InspectorComponents.swift
  - Frame/Models/EffectsConfig.swift
```

**Verification:**

- Build app, open Editor
- Adjust Padding slider from default
- Verify "Reset" button appears
- Click Reset, verify value returns to default

### Reduce inspector panel padding [ui]

Reduce padding from ~16px to ~8-12px in all inspector views for higher information density.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - Frame/Views/Editor/Inspector/BackgroundInspector.swift
  - Frame/Views/Editor/Inspector/CursorInspector.swift
  - Frame/Views/Editor/Inspector/KeyboardInspector.swift
  - Frame/Views/Editor/Inspector/WebcamInspector.swift
  - Frame/Views/Editor/Inspector/ZoomInspector.swift
  - Frame/Views/Editor/Inspector/AudioInspector.swift
```

**Verification:**

- Build app, open Editor
- Compare spacing to previous build
- Verify no clipping or overflow issues

### Change accent color to purple/blue [ui]

Replace bright green accent color with professional purple/blue (#7C7CFF or similar) across all UI elements.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - Frame/Utilities/Colors.swift # Create if doesn't exist
  - Frame/Views/Editor/Inspector/InspectorComponents.swift
```

**Verification:**

- Build app, open Editor
- Verify slider thumbs use new accent
- Verify value labels use new accent
- Check all inspector views for consistency

### Add glassmorphism blur to inspector panel [ui]

Apply NSVisualEffectView blur to inspector sidebar for depth and modern appearance.

**Metadata:**

```yaml
depends_on: ["Reduce inspector panel padding"]
parallel: false
conflicts_with: []
files:
  - Frame/Views/Editor/EditorView.swift
```

**Verification:**

- Build app, open Editor
- Verify sidebar shows blur effect
- Test on both light and dark mode
- Move window over different backgrounds to verify dynamic blur

### Show wallpaper thumbnails in preset grid [ui]

Replace color swatches with actual wallpaper thumbnail images in Background inspector.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - Frame/Views/Editor/Inspector/BackgroundInspector.swift
  - Frame/Assets/ # Add wallpaper images
```

**Verification:**

- Build app, open Editor
- Open Background inspector
- Verify 10 thumbnail images visible
- Verify selecting thumbnail applies correct background

### Add audio waveform visualization to timeline [feature]

Render audio amplitude as orange/blue waveform in timeline using Swift Charts with viewport caching.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - Frame/Views/Editor/TimelineView.swift # Modify or create
  - Frame/Playback/AudioWaveformGenerator.swift # Create
```

**Verification:**

- Build app, open Editor
- Load video with audio
- Verify orange/blue waveform appears in timeline
- Scrub timeline, verify smooth 60fps performance
- Load video without audio, verify no waveform shown

### Embed webcam as floating overlay in preview [feature]

Display webcam feed as draggable floating overlay within the video preview canvas.

**Metadata:**

```yaml
depends_on: ["Add glassmorphism blur to inspector panel"]
parallel: false
conflicts_with: []
files:
  - Frame/Views/Editor/PreviewCanvas.swift
  - Frame/Models/EffectsConfig.swift # Add webcam position
```

**Verification:**

- Build app, open Editor
- Enable webcam in settings
- Verify webcam appears as overlay in preview
- Drag webcam to new position
- Start recording, verify position is captured in output

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

- **Reset button UX:** Screen Studio shows Reset only when value differs from default. Consider adding hover state for discoverability.
- **Glassmorphism fallback:** If `.ultraThinMaterial` looks too subtle, try `.thinMaterial` or add a subtle background color behind the blur.
- **Audio waveform performance:** Only render 2-3x viewport width to allow smooth scrubbing without visible loading.
- **Webcam overlay:** Use drag gesture with bounds checking; snap to corners if dragged near edge ( Screen Studio pattern).
