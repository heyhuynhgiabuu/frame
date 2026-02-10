# Beads PRD: Wallpaper Thumbnail Presets in Background Inspector

**Bead:** bd-3lk  
**Created:** 2026-02-10  
**Status:** Draft

## Bead Metadata

```yaml
depends_on: [bd-13v]
parallel: true
conflicts_with: []
blocks: []
estimated_hours: 2
```

---

## Problem Statement

### What problem are we solving?

The Background inspector currently shows gradient color swatches for preset selection. Based on comparison with Screen Studio, users expect to see actual thumbnail previews of wallpapers/backgrounds, not just color representations. Color swatches don't provide enough visual context for users to understand what the final video will look like.

### Why now?

This was deferred from bd-13v (Editor UI improvements) due to requiring image assets. Now that the core UI improvements are shipped, we can add the visual polish that makes preset selection intuitive.

### Who is affected?

- **Primary users:** Content creators selecting backgrounds for their screen recordings
- **Secondary users:** Users exploring different visual styles for their videos

---

## Scope

### In-Scope

- Add 8-10 wallpaper thumbnail images to the project
- Replace color swatches in Background inspector with thumbnail grid
- Thumbnails should represent: gradients, solids, and subtle patterns
- Lazy loading / caching of thumbnail images
- Selected state highlighting on thumbnails

### Out-of-Scope

- User-uploaded custom wallpapers (future feature)
- Dynamic wallpaper generation
- Animated/live wallpapers
- macOS dynamic desktop wallpapers integration

---

## Proposed Solution

### Overview

Replace the current `LazyVGrid` of color swatches in `BackgroundInspector` with a grid of thumbnail images. Each thumbnail shows an actual preview of the background style (gradient, solid, or image). Users tap a thumbnail to apply that background.

### User Flow

1. User opens the Editor
2. Navigates to Background inspector
3. Sees grid of 8-10 thumbnail previews
4. Taps a thumbnail to apply that background
5. Preview canvas updates to show the selected background

---

## Requirements

### Functional Requirements

#### Thumbnail Grid

**Scenarios:**

- **WHEN** user opens Background inspector **THEN** they see a grid of 8-10 thumbnail images
- **WHEN** user taps a thumbnail **THEN** the corresponding background is applied to the video
- **WHEN** a thumbnail is selected **THEN** it shows a visual highlight/border
- **WHEN** thumbnails load **THEN** they show progressively (placeholder → thumbnail)

#### Asset Management

**Scenarios:**

- **WHEN** app launches **THEN** thumbnail images are bundled and accessible
- **WHEN** thumbnails display **THEN** they are appropriately sized (80x80pt @2x)
- **WHEN** user has accessibility settings enabled **THEN** thumbnails have sufficient contrast

### Non-Functional Requirements

- **Performance:** Thumbnails load in <100ms, cached after first load
- **Accessibility:** Each thumbnail has a descriptive label for VoiceOver
- **Compatibility:** Works on macOS 13.0+ (current minimum)
- **Bundle Size:** Thumbnail assets add <500KB to app bundle

---

## Success Criteria

- [ ] Background inspector shows 8-10 thumbnail images in a grid
  - Verify: Build app, open Editor → Background inspector, verify thumbnails visible
- [ ] Tapping thumbnail applies correct background to preview
  - Verify: Tap each thumbnail, verify preview canvas updates correctly
- [ ] Selected thumbnail shows visual highlight
  - Verify: Tap a thumbnail, verify it has selection border/highlight
- [ ] Build passes with no errors
  - Verify: `xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build`
- [ ] Assets included in app bundle
  - Verify: Build app, check `Frame.app/Contents/Resources/` for thumbnail images

---

## Technical Context

### Existing Patterns

- **Background Inspector:** `Frame/Views/Editor/Inspector/BackgroundInspector.swift` - Currently shows color swatches in a `LazyVGrid`
- **Background Types:** `.gradient`, `.solid`, `.wallpaper`, `.image` defined in `BackgroundType` enum
- **Asset Catalog:** App uses `Assets.xcassets` for bundled images
- **Selection State:** Currently uses `effects.backgroundType` and `effects.backgroundPreset` to track selection

### Key Files

- `Frame/Views/Editor/Inspector/BackgroundInspector.swift` - Main inspector view
- `Frame/Models/Project.swift` - `BackgroundType` enum and related models
- `Frame/Assets.xcassets/` - Where thumbnail images will be added

### Affected Files

```yaml
files:
  - Frame/Views/Editor/Inspector/BackgroundInspector.swift
  - Frame/Assets.xcassets/WallpaperThumbnails/ # New asset folder
```

---

## Risks & Mitigations

| Risk                         | Likelihood | Impact | Mitigation                                                             |
| ---------------------------- | ---------- | ------ | ---------------------------------------------------------------------- |
| No suitable wallpaper images | Low        | High   | Create simple gradient/pattern images in code or use system wallpapers |
| Thumbnail loading slow       | Low        | Medium | Pre-load thumbnails, use Image caching                                 |
| Asset bundle size too large  | Low        | Low    | Optimize images (WebP or compressed PNG), target <50KB each            |

---

## Open Questions

| Question                                                  | Owner | Due Date    | Status |
| --------------------------------------------------------- | ----- | ----------- | ------ |
| Should we generate gradients in code or use image assets? | TBD   | Before impl | Open   |
| How many presets? 8 or 10?                                | TBD   | Before impl | Open   |
| Do we need dark/light variants?                           | TBD   | Before impl | Open   |

---

## Tasks

### Create wallpaper thumbnail images [assets]

Create 8-10 thumbnail images (80x80pt @2x = 160x160px) representing different background styles: gradients, solids, and subtle patterns.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files: []
```

**Verification:**

- Images created in appropriate format (PNG or WebP)
- Images are 160x160px (80pt @2x)
- Total file size <500KB

### Add thumbnail assets to Xcode project [setup]

Add thumbnail images to `Assets.xcassets/WallpaperThumbnails/` and ensure they are included in the app target.

**Metadata:**

```yaml
depends_on: ["Create wallpaper thumbnail images"]
parallel: false
conflicts_with: []
files:
  - Frame/Assets.xcassets/WallpaperThumbnails/
  - Frame.xcodeproj/project.pbxproj
```

**Verification:**

- Build app, verify no asset warnings
- Check `Frame.app/Contents/Resources/Assets.car` contains thumbnails

### Replace color swatches with thumbnail grid [ui]

Update `BackgroundInspector.swift` to display thumbnail images instead of color swatches in the preset grid.

**Metadata:**

```yaml
depends_on: ["Add thumbnail assets to Xcode project"]
parallel: false
conflicts_with: []
files:
  - Frame/Views/Editor/Inspector/BackgroundInspector.swift
```

**Verification:**

- Build app, open Editor
- Open Background inspector
- Verify thumbnail grid displays (8-10 images)
- Tap each thumbnail, verify background applies correctly
- Verify selected thumbnail shows highlight border

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

- **Image generation:** If we can't find suitable wallpaper images, we can generate simple gradients in SwiftUI and snapshot them, or create them in a design tool
- **Selection highlight:** Use a 2pt border in the accent color (#7C7CFF) to match the app's visual style
- **Accessibility:** Ensure each thumbnail has `.accessibilityLabel()` describing the background style (e.g., "Sunset gradient", "Ocean blue", "Dark solid")
