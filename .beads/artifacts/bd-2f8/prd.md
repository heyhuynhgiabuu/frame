# Phase 3: Polish & Effects

**Bead:** bd-2f8  
**Created:** 2026-02-05  
**Status:** Draft

## Bead Metadata

```yaml
depends_on: [bd-3vd] # Phase 2 is complete
parallel: true # Tasks can run in parallel where dependencies allow
conflicts_with: []
blocks: [] # Phase 4 will depend on this
estimated_hours: 60 # 4 weeks of development
```

---

## Problem Statement

### What problem are we solving?

Frame can now record screens and export videos, but recordings look raw and amateur compared to polished tools like Screen Studio. Users expect:

- Automatic zoom effects that follow cursor activity
- Visible keyboard shortcuts for tutorials
- Professional backgrounds instead of bare desktop
- Persistent project files for resuming work

Without these polish features, Frame recordings require manual post-processing in external tools, defeating the "beautiful by default" value proposition.

### Why now?

Phase 2 (Core Recording) is complete with:

- ✅ ScreenCaptureKit integration
- ✅ Audio capture (mic + system)
- ✅ Timeline UI
- ✅ MP4 export

The foundation is solid. Now we need the differentiating features that make Frame stand out from basic screen recorders.

### Who is affected?

- **Primary users:** Developers creating product demos and tutorials
- **Secondary users:** Content creators, educators, documentation teams

---

## Scope

### In-Scope

- Cursor zoom & smoothing with automatic detection
- Keyboard shortcut overlay display
- Background customization (colors, gradients, images, padding)
- Project file format (`.frame`) for save/load

### Out-of-Scope

- Webcam overlay (deferred - complex face tracking)
- Advanced timeline editing (trim, cut, split - Phase 4)
- AI-powered auto-zoom suggestions (Pro tier)
- Motion blur effects (optimization needed)
- Custom cursor styles (future)

---

## Proposed Solution

### Overview

Add an effects pipeline between capture and encoding that processes each frame:

1. Track cursor position and velocity
2. Apply smooth zoom towards areas of activity
3. Overlay keyboard shortcut badges when keys are pressed
4. Composite against customizable backgrounds with padding
5. Save/load all settings in a `.frame` project format

### User Flow

1. **Recording:** User records screen as normal
2. **Preview:** Timeline shows effects applied in real-time preview
3. **Settings:** User adjusts zoom intensity, keyboard style, background in sidebar
4. **Save:** Project saves as `.frame` file with all settings and raw footage
5. **Load:** User can reopen project later and continue editing
6. **Export:** Final video includes all effects baked in

---

## Requirements

### Functional Requirements

#### Cursor Zoom & Smoothing

Automatic zoom that follows cursor activity, making click targets more visible.

**Scenarios:**

- **WHEN** user clicks the mouse **THEN** frame smoothly zooms to 150% centered on click position
- **WHEN** user scrolls **THEN** frame zooms to scroll area
- **WHEN** cursor is idle for >2s **THEN** frame smoothly zooms out to 100%
- **WHEN** cursor moves rapidly **THEN** camera follows with eased motion (no jerky panning)
- **WHEN** zoom is disabled in settings **THEN** no zoom effects are applied

#### Keyboard Shortcut Display

Show pressed keys as badges overlaid on the video.

**Scenarios:**

- **WHEN** user presses a modifier key (Cmd/Ctrl/Alt/Shift) **THEN** badge appears in corner
- **WHEN** user presses key combo (e.g., Cmd+S) **THEN** combined badge shows "⌘S"
- **WHEN** key is released **THEN** badge fades out after 500ms
- **WHEN** multiple keys pressed rapidly **THEN** badges stack or combine logically
- **WHEN** keyboard display is disabled **THEN** no badges appear

#### Background Customization

Replace or pad the captured content with customizable backgrounds.

**Scenarios:**

- **WHEN** user selects solid color **THEN** background fills with that color
- **WHEN** user selects gradient **THEN** background uses gradient
- **WHEN** user uploads image **THEN** background uses image (scaled/tiled)
- **WHEN** user adjusts padding **THEN** captured content is inset with background visible
- **WHEN** user adjusts corner radius **THEN** captured content has rounded corners

#### Project File Format

Save complete project state for later resumption.

**Scenarios:**

- **WHEN** user clicks "Save Project" **THEN** `.frame` file is created
- **WHEN** user opens `.frame` file **THEN** project loads with all settings and footage
- **WHEN** project contains unsaved changes **THEN** prompt to save on close
- **WHEN** .frame file is corrupted **THEN** show error with recovery options
- **WHEN** .frame file version is newer than app **THEN** show upgrade prompt

### Non-Functional Requirements

- **Performance:**
  - Effects processing <5ms per frame at 1080p60
  - Zoom transitions at 60fps minimum
  - Background compositing must not drop frames
- **Storage:**
  - `.frame` files use efficient binary format (MessagePack or similar)
  - Raw footage stored separately, linked by path
  - Project files <1MB without footage
- **Compatibility:**
  - `.frame` format versioned for forward/backward compatibility
  - macOS 12.3+ (Monterey and later)

---

## Success Criteria

- [ ] Cursor zoom automatically follows click activity
  - Verify: Record clicking in different areas, zoom follows smoothly
- [ ] Keyboard shortcuts display correctly
  - Verify: Record Cmd+C, Cmd+V sequence, badges appear
- [ ] Backgrounds can be customized
  - Verify: Change background color, see it in preview and export
- [ ] Projects can be saved and reopened
  - Verify: Save project, quit app, reopen project, all settings preserved
- [ ] Effects don't cause frame drops
  - Verify: `cargo test -p frame-core --test performance_test` passes

---

## Technical Context

### Existing Patterns

- **Effects Pipeline:** `design.md` shows effects pipeline after frame buffer
- **State Management:** `apps/desktop/src/app.rs` uses iced Elm architecture
- **Project Model:** `packages/core/src/project.rs` has Project struct
- **Capture Flow:** `packages/core/src/capture/platform.rs` provides raw frames

### Key Files

- `packages/core/src/project.rs` - Extend for effects settings
- `apps/desktop/src/app.rs` - Add effects state management
- `packages/renderer/src/lib.rs` - GPU-accelerated effects (stub exists)

### Affected Files

```yaml
files:
  - packages/core/src/effects/mod.rs # New effects module
  - packages/core/src/effects/zoom.rs # Cursor zoom logic
  - packages/core/src/effects/keyboard.rs # Key capture and display
  - packages/core/src/effects/background.rs # Background compositing
  - packages/core/src/project.rs # Extended for effects settings
  - packages/core/src/lib.rs # Export effects module
  - packages/ui-components/src/components/settings_panel.rs # Effects settings UI
  - apps/desktop/src/app.rs # Effects state integration
  - apps/desktop/src/ui/main.rs # Settings sidebar
```

---

## Risks & Mitigations

| Risk                                 | Likelihood | Impact | Mitigation                                    |
| ------------------------------------ | ---------- | ------ | --------------------------------------------- |
| Zoom jitter on rapid cursor movement | High       | Medium | Use Bézier easing, velocity-based damping     |
| Key event capture conflicts with app | Medium     | High   | Use CGEventTap (macOS) with careful filtering |
| Performance impact from GPU effects  | Medium     | High   | Profile early, fall back to CPU if needed     |
| Project format migration complexity  | Low        | Medium | Version field + migration functions           |
| Background image memory bloat        | Medium     | Low    | Lazy load, limit resolution                   |

---

## Open Questions

| Question                                                 | Owner | Due Date       | Status |
| -------------------------------------------------------- | ----- | -------------- | ------ |
| What easing curve for zoom transitions?                  | TBD   | Phase 3 Week 1 | Open   |
| Should keyboard badges be configurable (size, position)? | TBD   | Phase 3 Week 2 | Open   |
| Binary format: MessagePack vs Bincode vs custom?         | TBD   | Phase 3 Week 1 | Open   |
| Should raw footage be embedded or linked?                | TBD   | Phase 3 Week 1 | Open   |

---

## Tasks

### 1. Effects Module Foundation [core]

Create the effects module structure with pipeline integration points.

**Metadata:**

```yaml
depends_on: []
parallel: false
conflicts_with: []
files:
  - packages/core/src/effects/mod.rs
  - packages/core/src/lib.rs
```

**Verification:**

- `cargo test -p frame-core` passes
- Effects module compiles and exports types

### 2. Cursor Position Tracking [effects]

Track cursor position and velocity from capture stream.

**Metadata:**

```yaml
depends_on: ["Effects Module Foundation"]
parallel: true
conflicts_with: []
files:
  - packages/core/src/effects/cursor.rs
  - packages/core/src/capture/mod.rs
```

**Verification:**

- Cursor position available per frame
- Velocity calculation is smooth
- Unit tests for position interpolation

### 3. Zoom Effect Implementation [effects]

Implement smooth zoom that follows cursor activity.

**Metadata:**

```yaml
depends_on: ["Cursor Position Tracking"]
parallel: false
conflicts_with: []
files:
  - packages/core/src/effects/zoom.rs
  - packages/core/src/effects/mod.rs
```

**Verification:**

- Click triggers zoom to 150%
- Idle triggers zoom out
- Transitions use Bézier easing
- No jitter at 60fps

### 4. Keyboard Event Capture [effects]

Capture keyboard events system-wide using CGEventTap.

**Metadata:**

```yaml
depends_on: ["Effects Module Foundation"]
parallel: true
conflicts_with: []
files:
  - packages/core/src/effects/keyboard.rs
```

**Verification:**

- Modifier keys detected (Cmd, Ctrl, Alt, Shift)
- Key combos captured correctly
- Events timestamped for sync with video

### 5. Keyboard Badge Rendering [ui]

Render keyboard shortcut badges as overlay.

**Metadata:**

```yaml
depends_on: ["Keyboard Event Capture"]
parallel: false
conflicts_with: []
files:
  - packages/ui-components/src/components/keyboard_badge.rs
  - packages/ui-components/src/lib.rs
```

**Verification:**

- Badges show correct key symbols
- macOS symbols used (⌘, ⌥, ⇧, ⌃)
- Fade out animation works

### 6. Background Compositing [effects]

Implement background colors, gradients, and padding.

**Metadata:**

```yaml
depends_on: ["Effects Module Foundation"]
parallel: true
conflicts_with: []
files:
  - packages/core/src/effects/background.rs
```

**Verification:**

- Solid color backgrounds work
- Gradient backgrounds render
- Padding insets content correctly
- Corner radius clips content

### 7. Project Format Specification [core]

Define and implement `.frame` file format.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - packages/core/src/project.rs
  - packages/core/src/project/format.rs
```

**Verification:**

- Format versioned for migrations
- Serialize/deserialize round-trips
- File size under 1MB for typical project

### 8. Settings UI Panel [ui]

Create sidebar panel for effects settings.

**Metadata:**

```yaml
depends_on:
  - "Zoom Effect Implementation"
  - "Background Compositing"
parallel: false
conflicts_with: []
files:
  - packages/ui-components/src/components/settings_panel.rs
  - apps/desktop/src/ui/main.rs
```

**Verification:**

- Zoom intensity slider works
- Background color picker works
- Padding slider works
- Settings persist in project

### 9. Effects Pipeline Integration [integration]

Wire effects into the capture-to-encode pipeline.

**Metadata:**

```yaml
depends_on:
  - "Zoom Effect Implementation"
  - "Keyboard Badge Rendering"
  - "Background Compositing"
parallel: false
conflicts_with: []
files:
  - apps/desktop/src/app.rs
  - packages/core/src/encoder.rs
```

**Verification:**

- Preview shows effects in real-time
- Export includes effects baked in
- No frame drops at 1080p60

### 10. Project Save/Load [persistence]

Implement save and load for `.frame` projects.

**Metadata:**

```yaml
depends_on:
  - "Project Format Specification"
  - "Effects Pipeline Integration"
parallel: false
conflicts_with: []
files:
  - apps/desktop/src/app.rs
  - packages/core/src/project.rs
```

**Verification:**

- File > Save creates .frame file
- File > Open loads .frame file
- All settings preserved
- Unsaved changes prompt works

### 11. Performance Optimization [performance]

Profile and optimize effects for 60fps target.

**Metadata:**

```yaml
depends_on:
  - "Effects Pipeline Integration"
parallel: false
conflicts_with: []
files:
  - packages/core/src/effects/mod.rs
  - packages/renderer/src/lib.rs
```

**Verification:**

- `cargo bench` shows <5ms per frame
- No dropped frames in stress test
- Memory usage stable over time

### 12. Documentation [docs]

Document effects API and project format.

**Metadata:**

```yaml
depends_on:
  - "Effects Pipeline Integration"
  - "Project Save/Load"
parallel: true
conflicts_with: []
files:
  - docs/EFFECTS.md
  - docs/PROJECT_FORMAT.md
  - README.md
```

**Verification:**

- Effects API documented
- Project format spec documented
- README updated with new features

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

- **Zoom Algorithm:** Consider Screen Studio's approach - smooth follow with "magnetic" click points
- **Key Capture:** macOS requires Accessibility permissions for CGEventTap
- **Project Format:** Consider using SQLite instead of flat file for complex projects
- **GPU Rendering:** packages/renderer exists but is a stub - may need wgpu setup
- **Testing:** Create visual regression tests for effects output
