# Beads PRD: Full Export Settings Panel with MP4/GIF Options

**Bead:** bd-3t5  
**Created:** 2026-02-10  
**Status:** Draft

## Bead Metadata

```yaml
depends_on: []
parallel: true
conflicts_with: []
blocks: []
estimated_hours: 3
```

---

## Problem Statement

### What problem are we solving?

Frame's current export is minimal - just an export button with no settings. Screen Studio provides a full export panel with format selection (MP4/GIF), frame rate, output size, and quality settings. Without these options, users cannot optimize exports for different use cases (social media, documentation, presentations).

### Why now?

Export settings are a basic requirement for any video tool. This is a quick win that significantly improves user control. It's also a prerequisite for proper GIF export which users frequently request.

### Who is affected?

- **Primary users:** Anyone exporting videos for different platforms
- **Secondary users:** Users who need GIFs for documentation or social media

---

## Scope

### In-Scope

- Export settings panel (popup/modal)
- Format selector: MP4 / GIF
- Frame rate: 30fps / 60fps
- Output size: 720p / 1080p / 4K / Original
- Quality: Low / Medium / High
- Export to file button
- Copy to clipboard button
- Export progress indicator
- Export completion notification

### Out-of-Scope

- Custom frame rates (only 30/60)
- Custom resolution inputs
- Watermark options
- Batch export
- Cloud upload integration

---

## Proposed Solution

### Overview

Expand the existing ExportPanel to show a full settings interface when user clicks Export. Show format options, quality settings, and destination choice (file or clipboard). Display progress during export and completion status.

### User Flow

1. User clicks Export button in toolbar
2. Export settings panel appears
3. User selects format (MP4/GIF)
4. User selects frame rate (30/60)
5. User selects output size
6. User selects quality
7. User clicks "Export to File" or "Copy to Clipboard"
8. Progress indicator shows during export
9. Success notification appears when done

---

## Requirements

### Functional Requirements

#### Format Selection

**Scenarios:**

- **WHEN** user selects MP4 **THEN** video exports as H.264 MP4
- **WHEN** user selects GIF **THEN** video exports as animated GIF
- **WHEN** GIF selected **THEN** frame rate limited to 30fps max

#### Frame Rate

**Scenarios:**

- **WHEN** 30fps selected **THEN** export uses 30 frames per second
- **WHEN** 60fps selected **THEN** export uses 60 frames per second
- **WHEN** GIF format **THEN** 60fps option disabled

#### Output Size

**Scenarios:**

- **WHEN** 720p selected **THEN** output scaled to 1280x720
- **WHEN** 1080p selected **THEN** output scaled to 1920x1080
- **WHEN** 4K selected **THEN** output scaled to 3840x2160
- **WHEN** Original selected **THEN** output uses recorded resolution

#### Quality

**Scenarios:**

- **WHEN** Low selected **THEN** higher compression, smaller file
- **WHEN** High selected **THEN** lower compression, larger file
- **WHEN** GIF format **THEN** quality affects color palette size

#### Export Actions

**Scenarios:**

- **WHEN** Export to File clicked **THEN** save panel opens
- **WHEN** destination selected **THEN** export begins
- **WHEN** Copy to Clipboard clicked **THEN** export to temp file then copy
- **WHEN** export complete **THEN** success notification shown

#### Progress

**Scenarios:**

- **WHEN** export starts **THEN** progress bar appears
- **WHEN** export progresses **THEN** progress % updates
- **WHEN** export completes **THEN** progress bar fills and dismisses
- **WHEN** export fails **THEN** error message shown

### Non-Functional Requirements

- **Performance:** Export uses hardware encoding when available
- **File Size:** GIF exports optimized for web use
- **UX:** Settings persist between sessions

---

## Success Criteria

- [ ] Export panel shows format/frame rate/size/quality options
  - Verify: Click Export, see all settings
- [ ] MP4 export works with selected settings
  - Verify: Export MP4, file created correctly
- [ ] GIF export works with selected settings
  - Verify: Export GIF, animated GIF created
- [ ] Export to file opens save panel
  - Verify: Click Export to File, save panel appears
- [ ] Copy to clipboard works
  - Verify: Click Copy to Clipboard, paste works elsewhere
- [ ] Progress indicator shows during export
  - Verify: Start export, see progress bar
- [ ] Build passes with no errors
  - Verify: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`

---

## Technical Context

### Existing Patterns

- **ExportPanel:** `Frame/Views/Export/ExportPanel.swift` exists but minimal.
- **ExportEngine:** `Frame/Export/ExportEngine.swift` handles AVAssetWriter export.
- **GIF Export:** Not yet implemented. Need to add GIF generation.
- **Settings Persistence:** EffectsConfig pattern for saving user preferences.

### Key Files

- `Frame/Views/Export/ExportPanel.swift` - Expand with full settings UI
- `Frame/Export/ExportConfig.swift` - Add export settings model
- `Frame/Export/ExportEngine.swift` - Update to use settings
- `Frame/Export/GIFExporter.swift` - NEW: GIF generation
- `Frame/Models/Project.swift` - Add export settings persistence

### Affected Files

```yaml
files:
  - Frame/Views/Export/ExportPanel.swift # Expand UI
  - Frame/Export/ExportConfig.swift # Settings model
  - Frame/Export/ExportEngine.swift # Use settings
  - Frame/Export/GIFExporter.swift # NEW
  - Frame/Models/Project.swift # Persistence
```

---

## Risks & Mitigations

| Risk                               | Likelihood | Impact | Mitigation                      |
| ---------------------------------- | ---------- | ------ | ------------------------------- |
| GIF export quality poor            | Medium     | Medium | Use proper GIF encoding library |
| Export settings UI cluttered       | Low        | Low    | Clean segmented controls        |
| Export fails with certain settings | Low        | High   | Validation and error handling   |

---

## Tasks

### Create export settings model [backend]

Define ExportConfig struct with all export options.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - Frame/Export/ExportConfig.swift
  - Frame/Models/Project.swift
```

**Verification:**

- ExportConfig struct created
- All properties: format, fps, size, quality
- Encoding/decoding works

### Build export settings UI [ui]

Create full export panel with format selector, frame rate, size, quality controls.

**Metadata:**

```yaml
depends_on: ["Create export settings model"]
parallel: false
conflicts_with: []
files:
  - Frame/Views/Export/ExportPanel.swift
```

**Verification:**

- Open Export panel, see all controls
- Format selector works
- Settings update correctly

### Add GIF export capability [backend]

Implement GIF generation for GIF format exports.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - Frame/Export/GIFExporter.swift
```

**Verification:**

- Export GIF, file created
- Animation plays correctly
- File size reasonable

### Update ExportEngine [backend]

Modify ExportEngine to use ExportConfig settings.

**Metadata:**

```yaml
depends_on: ["Create export settings model", "Add GIF export capability"]
parallel: false
conflicts_with: []
files:
  - Frame/Export/ExportEngine.swift
```

**Verification:**

- Export uses selected settings
- Different formats work
- Quality settings affect output

### Add progress and completion UX [ui]

Implement progress indicator and completion notifications.

**Metadata:**

```yaml
depends_on: ["Update ExportEngine"]
parallel: false
conflicts_with: []
files:
  - Frame/Views/Export/ExportPanel.swift
```

**Verification:**

- Progress bar shows during export
- Progress % updates
- Success notification on completion
