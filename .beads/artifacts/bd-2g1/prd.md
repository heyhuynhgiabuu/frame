# Phase 5: MVP Polish PRD

**Bead:** bd-2g1
**Type:** Epic
**Status:** In Progress
**Created:** 2026-02-05

## Bead Metadata

```yaml
depends_on: [bd-cdx] # Phase 4 Timeline Editing
parallel: false
conflicts_with: []
blocks: []
estimated_hours: 60
```

## Overview

Phase 5 focuses on achieving **competitive parity** with Screen Studio before building Pro/Cloud features. The gaps identified through competitor analysis are: webcam overlay, aspect ratio switching, region recording, export presets (including GIF), and visual polish (shadow/inset).

## Goals

1. **Webcam Overlay**: Record and display face camera with positioning options
2. **Aspect Ratio**: Switch output between horizontal, vertical, and square
3. **Region Recording**: Select an area of screen to record
4. **Export Presets**: Web-optimized, social media, GIF export
5. **Visual Polish**: Shadow, inset effects on video
6. **Quick Share**: Copy to clipboard for instant sharing

## Non-Goals

- Cloud sync and shareable links (deferred to Phase 6)
- Team workspaces (deferred to Phase 6)
- iOS device recording (future phase)
- AI transcription/captions (future phase)
- Motion blur (low priority)

## Technical Approach

### Webcam Capture

- Use `nokhwa` crate for cross-platform camera access
- Camera runs as separate capture stream, synced with screen
- Position: corner overlay (bottom-left, bottom-right, top-left, top-right)
- Shape: circle or rounded rectangle
- Auto zoom-out when cursor approaches (optional)

### Aspect Ratio

- Output dimensions calculated at export time, not during capture
- Supported ratios: 16:9 (horizontal), 9:16 (vertical), 1:1 (square), 4:3 (classic)
- Background fills letterbox/pillarbox areas
- Center or offset positioning within new aspect

### Region Recording

- Selection overlay during "Select Region" mode
- Stored as `CaptureRegion { x, y, width, height }` in project
- Region can be adjusted post-recording (non-destructive)

### Export System

- Preset system: `ExportPreset { codec, resolution, bitrate, format }`
- Built-in presets: "Web", "YouTube", "Twitter/X", "Instagram", "GIF"
- GIF export: use `gifski` crate for high-quality palette optimization
- Custom preset creation and saving

### Visual Effects

- Shadow: drop shadow on video frame, configurable offset/blur/color
- Inset: simulated depth effect around video edges
- Rounded corners: already implemented, ensure consistent with new effects

---

## Tasks

### Webcam Capture Service [core]

Add webcam capture using `nokhwa` crate.

**Verification:**

- `cargo build -p frame-core --features webcam` succeeds
- `WebcamCapture::list_devices()` returns available cameras
- `WebcamCapture::start()` begins capture loop
- `WebcamCapture::stop()` cleanly terminates
- Frame data matches expected dimensions

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files: [packages/core/Cargo.toml, packages/core/src/capture/webcam.rs]
```

### Webcam Overlay Compositor [core]

Composite webcam frame onto screen recording.

**Verification:**

- Webcam appears at configured corner position
- Circle and rounded-rectangle shapes work
- Webcam scales proportionally to output resolution
- Webcam can be toggled on/off during playback
- Performance: <5ms overhead per frame

**Metadata:**

```yaml
depends_on: [core-1]
parallel: false
conflicts_with: []
files: [packages/core/src/effects/webcam_overlay.rs, packages/core/src/effects/pipeline.rs]
```

### Webcam UI Settings [ui]

UI for webcam configuration in settings panel.

**Verification:**

- Camera device dropdown populated with available cameras
- Preview shows live webcam feed
- Position selector (4 corners)
- Shape toggle (circle/rounded rect)
- Size slider (small/medium/large)
- On/off toggle

**Metadata:**

```yaml
depends_on: [core-1]
parallel: true
conflicts_with: []
files:
  [
    packages/ui-components/src/components/webcam_settings.rs,
    packages/ui-components/src/components/settings_panel.rs,
  ]
```

### Aspect Ratio Calculator [core]

Calculate output dimensions and letterbox/pillarbox areas.

**Verification:**

- `AspectRatio::Horizontal16x9.dimensions(1920)` returns (1920, 1080)
- `AspectRatio::Vertical9x16.dimensions(1080)` returns (1080, 1920)
- `AspectRatio::Square.dimensions(1080)` returns (1080, 1080)
- Letterbox/pillarbox coordinates calculated correctly
- Works with any input resolution

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files: [packages/core/src/effects/aspect_ratio.rs]
```

### Aspect Ratio Export Integration [core]

Apply aspect ratio during export encoding.

**Verification:**

- Exported video has correct dimensions for selected ratio
- Background color fills letterbox/pillarbox
- Video content centered or offset per settings
- No stretching or squishing of content

**Metadata:**

```yaml
depends_on: [core-3]
parallel: false
conflicts_with: []
files: [packages/core/src/encoder.rs]
```

### Aspect Ratio UI [ui]

Aspect ratio selector in export dialog.

**Verification:**

- Dropdown with presets: Original, 16:9, 9:16, 1:1, 4:3
- Preview updates to show selected ratio
- Custom ratio input (advanced)
- Remembers last used ratio

**Metadata:**

```yaml
depends_on: [core-3]
parallel: true
conflicts_with: []
files: [packages/ui-components/src/components/export_dialog.rs]
```

### Region Selection Overlay [ui]

Interactive overlay for selecting recording region.

**Verification:**

- Full-screen transparent overlay appears
- Drag to draw selection rectangle
- Handles for resizing selection
- Dimension labels shown (width x height)
- Cancel with Escape, confirm with Enter/click outside

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files: [packages/ui-components/src/components/region_selector.rs]
```

### Region Capture Integration [core]

Store and apply region to capture stream.

**Verification:**

- `CaptureRegion` stored in project settings
- ScreenCaptureKit captures only selected region
- Region can be changed after recording (cropping)
- Full screen capture still works (region = None)

**Metadata:**

```yaml
depends_on: [ui-3]
parallel: false
conflicts_with: []
files: [packages/core/src/capture/mod.rs, packages/core/src/project.rs]
```

### Export Preset System [core]

Data model and serialization for export presets.

**Verification:**

- `ExportPreset` struct with codec, resolution, bitrate, format
- Built-in presets load correctly
- Custom presets save to disk
- Presets can be imported/exported as JSON

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files: [packages/core/src/export_preset.rs]
```

### Built-in Export Presets [core]

Define standard presets for common use cases.

**Verification:**

- "Web" preset: H.264, 1080p, 8Mbps, MP4
- "YouTube" preset: H.264, 4K, 20Mbps, MP4
- "Twitter/X" preset: H.264, 720p, 5Mbps, MP4 (max 2:20)
- "Instagram" preset: H.264, 1080x1080, 8Mbps, MP4
- "GIF" preset: GIF, 480p, 10fps, optimized palette

**Metadata:**

```yaml
depends_on: [core-5]
parallel: false
conflicts_with: []
files: [packages/core/src/export_preset.rs]
```

### GIF Export [core]

High-quality GIF export using `gifski`.

**Verification:**

- `cargo build -p frame-core --features gif` succeeds
- GIF output plays smoothly at configured FPS
- Palette optimization produces good colors
- File size reasonable (<10MB for 10s clip)
- Handles transparency if background removed

**Metadata:**

```yaml
depends_on: [core-5]
parallel: true
conflicts_with: []
files: [packages/core/Cargo.toml, packages/core/src/encoder/gif.rs]
```

### Export Presets UI [ui]

Preset selection and management in export dialog.

**Verification:**

- Dropdown shows all presets (built-in + custom)
- Selected preset populates all settings
- "Save as preset" creates custom preset
- "Delete" removes custom presets (not built-in)
- Settings can be modified after selecting preset

**Metadata:**

```yaml
depends_on: [core-5, core-6]
parallel: true
conflicts_with: []
files: [packages/ui-components/src/components/export_dialog.rs]
```

### Shadow Effect [core]

Drop shadow effect on video frame.

**Verification:**

- Shadow renders behind video content
- Configurable: offset_x, offset_y, blur_radius, color
- Shadow respects rounded corners
- Performance: <2ms per frame overhead

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files: [packages/core/src/effects/shadow.rs, packages/core/src/effects/pipeline.rs]
```

### Inset Effect [core]

Inset/depth effect on video edges.

**Verification:**

- Subtle depth effect visible on edges
- Configurable: intensity, color (light/dark)
- Works with rounded corners
- Can be disabled independently

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files: [packages/core/src/effects/inset.rs, packages/core/src/effects/pipeline.rs]
```

### Visual Effects UI [ui]

Settings for shadow and inset in settings panel.

**Verification:**

- Shadow section: enable toggle, offset sliders, blur slider, color picker
- Inset section: enable toggle, intensity slider
- Live preview updates as settings change
- Reset to defaults button

**Metadata:**

```yaml
depends_on: [core-8, core-9]
parallel: true
conflicts_with: []
files: [packages/ui-components/src/components/settings_panel.rs]
```

### Copy to Clipboard [desktop]

Copy exported video/GIF directly to clipboard.

**Verification:**

- "Copy to Clipboard" button in export complete dialog
- MP4 copies as file reference
- GIF copies as image data (pasteable into apps)
- macOS pasteboard integration works
- Success/failure feedback shown

**Metadata:**

```yaml
depends_on: [core-7]
parallel: true
conflicts_with: []
files: [apps/desktop/src/clipboard.rs, apps/desktop/src/ui/export.rs]
```

### Desktop Integration [desktop]

Wire all new features into desktop app.

**Verification:**

- Webcam toggle in recording controls
- Aspect ratio in export flow
- Region selection mode accessible from menu
- Export presets working end-to-end
- Shadow/inset visible in preview

**Metadata:**

```yaml
depends_on:
  [
    core-1,
    core-2,
    ui-1,
    ui-2,
    ui-3,
    core-3,
    core-4,
    ui-4,
    core-5,
    core-6,
    core-7,
    ui-5,
    core-8,
    core-9,
    ui-6,
    desktop-1,
  ]
parallel: false
conflicts_with: []
files: [apps/desktop/src/app.rs, apps/desktop/src/ui/]
```

### Documentation [docs]

Update AGENTS.md and add feature documentation.

**Verification:**

- AGENTS.md updated for new modules
- Webcam setup guide
- Export presets documentation
- Region recording guide

**Metadata:**

```yaml
depends_on: [all above]
parallel: false
conflicts_with: []
files: [AGENTS.md, packages/core/AGENTS.md]
```

---

## Acceptance Criteria

1. User can record with webcam overlay (4 positions, 2 shapes)
2. User can export to different aspect ratios (16:9, 9:16, 1:1, 4:3)
3. User can select a region of screen to record
4. User can export using presets (Web, YouTube, Twitter, Instagram, GIF)
5. User can export high-quality GIF
6. Video has configurable shadow and inset effects
7. User can copy exported video/GIF to clipboard
8. All verification steps pass
9. `cargo clippy --workspace -- -D warnings` passes
10. `cargo test --workspace` passes

## Dependencies to Add

```toml
# packages/core/Cargo.toml
[dependencies]
nokhwa = { version = "0.10", features = ["input-avfoundation"], optional = true }
gifski = { version = "1.12", optional = true }
arboard = "3.2"  # Clipboard access

[features]
webcam = ["nokhwa"]
gif = ["gifski"]
```

## Out of Scope (Phase 6+)

- Cloud sync and shareable links
- Team workspaces
- iOS device recording
- AI transcription/captions
- Motion blur effect
- Multi-clip merge
