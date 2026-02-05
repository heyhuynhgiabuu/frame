# Phase 4: Timeline Editing

**Bead:** bd-cdx  
**Created:** 2026-02-05  
**Status:** Draft

## Bead Metadata

```yaml
depends_on: [bd-2f8] # Phase 3 (Effects) must be complete
parallel: true
conflicts_with: []
blocks: [] # Phase 5 (Pro Features) will depend on this
estimated_hours: 80 # ~5 weeks of development
```

---

## Problem Statement

### What problem are we solving?

Frame can record and export videos with effects, but users cannot edit their recordings within the app. To trim mistakes, remove pauses, or split clips, users must:

1. Export the video
2. Open an external video editor (iMovie, DaVinci, etc.)
3. Edit there
4. Lose all Frame effects if they want to re-edit

This breaks the "beautiful by default" workflow and forces context-switching that developers hate.

### Why now?

Phase 3 (Effects) is complete with:

- ✅ Cursor zoom & smoothing
- ✅ Keyboard shortcut display
- ✅ Background customization
- ✅ Project file format (.frame)

The effects pipeline and project persistence are solid. Timeline editing is the final free-tier feature before Pro features (cloud sync, teams).

### Who is affected?

- **Primary users:** Developers recording tutorials who make mistakes and need to trim
- **Secondary users:** Anyone who records and wants basic post-processing without leaving Frame

---

## Scope

### In-Scope

- **Trim:** Set in/out points to remove content from start/end
- **Cut:** Remove a section from the middle of a recording
- **Split:** Divide a clip into multiple segments at a point
- **Undo/Redo:** Reversible editing operations with history
- **Non-destructive:** Original footage preserved; edits stored as metadata
- **Preview:** See edits applied in real-time before export

### Out-of-Scope

- Multi-track timeline (single video track only)
- Audio waveform visualization (future)
- Transitions between clips (fade, wipe, etc.)
- Speed ramping/time stretch
- Copy/paste clips between projects
- Keyboard-driven frame-by-frame navigation (partial - basic only)

---

## Proposed Solution

### Overview

Extend the existing Timeline widget to support edit operations. Edits are stored as a list of `EditOperation` structs in the Project, allowing non-destructive editing where the original recording is preserved.

### User Flow

1. **Record:** User records a screen session
2. **Review:** Timeline shows the full recording with playhead
3. **Mark Edit Points:** User clicks timeline to position playhead, then:
   - Press `I` to mark In point (trim start)
   - Press `O` to mark Out point (trim end)
   - Press `X` to cut selection
   - Press `S` to split at playhead
4. **Preview:** Timeline updates to show edited result
5. **Undo:** Press `Cmd+Z` to undo, `Cmd+Shift+Z` to redo
6. **Export:** Final video renders with edits applied

### Edit Operations Model

```rust
pub enum EditOperation {
    Trim { start: Duration, end: Duration },
    Cut { from: Duration, to: Duration },
    Split { at: Duration },
}

pub struct EditHistory {
    operations: Vec<EditOperation>,
    current_index: usize, // For undo/redo
}
```

---

## Requirements

### Functional Requirements

#### Trim

Remove content from the beginning and/or end of a recording.

**Scenarios:**

- **WHEN** user presses `I` at 00:05 **THEN** content before 00:05 is marked for removal
- **WHEN** user presses `O` at 00:30 **THEN** content after 00:30 is marked for removal
- **WHEN** trim handles are dragged **THEN** preview updates in real-time
- **WHEN** trim is applied **THEN** playhead resets to new start

#### Cut

Remove a section from the middle of a recording.

**Scenarios:**

- **WHEN** user selects a range (I → O) and presses `X` **THEN** that section is removed
- **WHEN** cut is applied **THEN** content after the cut shifts earlier
- **WHEN** cut removes all content **THEN** error message is shown (cannot create empty video)

#### Split

Divide a clip into multiple segments.

**Scenarios:**

- **WHEN** user positions playhead and presses `S` **THEN** clip splits into two segments
- **WHEN** split creates segment shorter than 0.5s **THEN** warning is shown
- **WHEN** user drags between split segments **THEN** segments can be reordered (stretch goal)

#### Undo/Redo

Reversible editing with full history.

**Scenarios:**

- **WHEN** user presses `Cmd+Z` **THEN** last operation is undone
- **WHEN** user presses `Cmd+Shift+Z` **THEN** undone operation is redone
- **WHEN** new operation is performed after undo **THEN** redo history is cleared
- **WHEN** undo stack is empty **THEN** `Cmd+Z` is disabled

#### Timeline Visual Updates

Timeline UI reflects edit state clearly.

**Scenarios:**

- **WHEN** trim is active **THEN** trimmed regions are grayed out
- **WHEN** cut is active **THEN** cut region shows X pattern overlay
- **WHEN** split exists **THEN** split point shows divider line
- **WHEN** hovering over edit region **THEN** cursor changes to indicate draggable

### Non-Functional Requirements

- **Performance:** Timeline scrubbing must be <16ms (60fps) even with multiple edits
- **Memory:** Edit operations stored as metadata only; no frame duplication
- **Persistence:** Edits survive app restart (saved in .frame project file)
- **Accessibility:** All edit operations accessible via keyboard shortcuts

---

## Success Criteria

- [ ] Can trim start/end of a recording using I/O keys
  - Verify: `Record 30s video, trim to 10-20s, export, check duration is 10s`
- [ ] Can cut a middle section out
  - Verify: `Record 30s video, cut 10-20s, export, check gap is removed`
- [ ] Can split a clip at playhead position
  - Verify: `Split at 15s, verify two separate segments appear in timeline`
- [ ] Undo/redo works for all operations
  - Verify: `Make 3 edits, undo all 3, redo 2, verify state matches`
- [ ] Edits persist in project file
  - Verify: `Make edits, close app, reopen, verify edits are preserved`
- [ ] Export respects edit operations
  - Verify: `Export edited video, verify output reflects all edits`
- [ ] `cargo clippy --workspace -- -D warnings` passes
  - Verify: `cargo clippy --workspace -- -D warnings`

---

## Technical Context

### Existing Patterns

- **Timeline Widget:** `packages/ui-components/src/components/timeline.rs` - Canvas-based with Clip struct
- **Project Model:** `packages/core/src/project.rs` - Recording and Project structs with serde
- **Effects Pipeline:** `packages/core/src/effects/pipeline.rs` - Frame processing pattern

### Key Files

- `packages/ui-components/src/components/timeline.rs` - Timeline UI (extend)
- `packages/core/src/project.rs` - Add EditOperation, EditHistory
- `packages/core/src/encoder.rs` - Modify to respect edit operations during export

### Affected Files

```yaml
files:
  - packages/core/src/project.rs # Add EditOperation, EditHistory
  - packages/core/src/lib.rs # Export new types
  - packages/core/src/encoder.rs # Apply edits during encoding
  - packages/ui-components/src/components/timeline.rs # Edit UI
  - packages/ui-components/src/components/mod.rs # Export new types
  - apps/desktop/src/ui/main.rs # Wire up keyboard shortcuts
  - apps/desktop/src/app.rs # Handle edit messages
```

---

## Risks & Mitigations

| Risk                              | Likelihood | Impact | Mitigation                                       |
| --------------------------------- | ---------- | ------ | ------------------------------------------------ |
| Timeline scrubbing becomes laggy  | Medium     | High   | Profile early; use edit metadata not actual cuts |
| Undo/redo state corruption        | Low        | High   | Immutable operations list; extensive testing     |
| Edit operations don't match video | Medium     | High   | Round to frame boundaries; add validation        |
| Complex multi-edit combinations   | Medium     | Medium | Start with simple linear edits; iterate          |

---

## Open Questions

| Question                                   | Owner | Due Date   | Status |
| ------------------------------------------ | ----- | ---------- | ------ |
| Should segments be reorderable after split | TBD   | 2026-02-10 | Open   |
| Maximum undo history depth?                | TBD   | 2026-02-10 | Open   |
| Frame-accurate vs time-based cuts?         | TBD   | 2026-02-10 | Open   |

---

## Tasks

### Edit Operations Data Model [core]

Core data types for representing edit operations stored in the project file.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - packages/core/src/project.rs
  - packages/core/src/lib.rs
```

**Verification:**

- `cargo clippy -p frame-core -- -D warnings`
- Unit tests for EditOperation serialization/deserialization

---

### Edit History with Undo/Redo [core]

EditHistory struct with push, undo, redo, and can_undo/can_redo methods.

**Metadata:**

```yaml
depends_on: ["Edit Operations Data Model"]
parallel: true
conflicts_with: []
files:
  - packages/core/src/project.rs
```

**Verification:**

- `cargo test -p frame-core edit_history`
- Test: push → undo → redo cycle preserves operations

---

### Timeline Selection State [ui]

Add selection state to Timeline (in_point, out_point, selected_range) for marking edit regions.

**Metadata:**

```yaml
depends_on: []
parallel: true
conflicts_with: []
files:
  - packages/ui-components/src/components/timeline.rs
```

**Verification:**

- `cargo clippy -p frame-ui -- -D warnings`
- Timeline can store and display selection range

---

### Timeline Trim Handles [ui]

Visual handles at start/end that can be dragged to set trim points.

**Metadata:**

```yaml
depends_on: ["Timeline Selection State"]
parallel: false
conflicts_with: []
files:
  - packages/ui-components/src/components/timeline.rs
```

**Verification:**

- Drag handles visible at clip edges
- Dragging updates trim state

---

### Timeline Cut Region Visualization [ui]

Show cut regions with distinct visual treatment (grayed out, X pattern).

**Metadata:**

```yaml
depends_on: ["Timeline Selection State"]
parallel: true
conflicts_with: []
files:
  - packages/ui-components/src/components/timeline.rs
```

**Verification:**

- Cut regions render differently from active content
- Multiple cuts render correctly

---

### Timeline Split Point Visualization [ui]

Show split points as vertical dividers between segments.

**Metadata:**

```yaml
depends_on: ["Timeline Selection State"]
parallel: true
conflicts_with: []
files:
  - packages/ui-components/src/components/timeline.rs
```

**Verification:**

- Split points visible as distinct markers
- Clicking near split selects the right segment

---

### Keyboard Shortcuts for Editing [desktop]

Wire up I (in), O (out), X (cut), S (split), Cmd+Z (undo), Cmd+Shift+Z (redo).

**Metadata:**

```yaml
depends_on: ["Edit Operations Data Model", "Timeline Selection State"]
parallel: false
conflicts_with: []
files:
  - apps/desktop/src/app.rs
  - apps/desktop/src/ui/main.rs
```

**Verification:**

- All shortcuts trigger correct actions
- Shortcuts disabled when not applicable

---

### Trim Operation Implementation [core]

Apply trim to EditHistory and validate trim doesn't create empty video.

**Metadata:**

```yaml
depends_on: ["Edit History with Undo/Redo"]
parallel: true
conflicts_with: []
files:
  - packages/core/src/project.rs
```

**Verification:**

- `cargo test -p frame-core trim`
- Trim persists in project save/load

---

### Cut Operation Implementation [core]

Apply cut to EditHistory, shift subsequent content timestamps.

**Metadata:**

```yaml
depends_on: ["Edit History with Undo/Redo"]
parallel: true
conflicts_with: []
files:
  - packages/core/src/project.rs
```

**Verification:**

- `cargo test -p frame-core cut`
- Cut persists and loads correctly

---

### Split Operation Implementation [core]

Apply split to EditHistory, create segment boundaries.

**Metadata:**

```yaml
depends_on: ["Edit History with Undo/Redo"]
parallel: true
conflicts_with: []
files:
  - packages/core/src/project.rs
```

**Verification:**

- `cargo test -p frame-core split`
- Split creates expected segment count

---

### Encoder Edit Support [core]

Modify encoder to skip cut regions and respect trim boundaries during export.

**Metadata:**

```yaml
depends_on: ["Trim Operation Implementation", "Cut Operation Implementation"]
parallel: false
conflicts_with: []
files:
  - packages/core/src/encoder.rs
```

**Verification:**

- Export with trim produces shorter video
- Export with cut removes the cut section

---

### Edit Preview in Timeline [integration]

Timeline preview updates in real-time as edits are applied.

**Metadata:**

```yaml
depends_on:
  [
    "Timeline Trim Handles",
    "Timeline Cut Region Visualization",
    "Trim Operation Implementation",
    "Cut Operation Implementation",
  ]
parallel: false
conflicts_with: []
files:
  - apps/desktop/src/app.rs
  - packages/ui-components/src/components/timeline.rs
```

**Verification:**

- Make edit, see timeline update immediately
- Scrub through edited timeline, playhead respects edits

---

### Project Edit Persistence [persistence]

Ensure edits save/load correctly in .frame project files.

**Metadata:**

```yaml
depends_on: ["Edit History with Undo/Redo"]
parallel: true
conflicts_with: []
files:
  - packages/core/src/project.rs
```

**Verification:**

- Save project with edits, close, reopen, edits intact
- Version bump if needed for format change

---

### Documentation [docs]

Update AGENTS.md files with edit operation patterns and usage.

**Metadata:**

```yaml
depends_on: ["Edit Preview in Timeline", "Encoder Edit Support"]
parallel: true
conflicts_with: []
files:
  - packages/core/AGENTS.md
  - packages/ui-components/AGENTS.md
  - apps/desktop/AGENTS.md
```

**Verification:**

- AGENTS.md includes edit operation examples
- Keyboard shortcuts documented

---

## Notes

- Frame-accurate editing would require knowing the video's frame rate and aligning cuts to frame boundaries. For MVP, time-based (millisecond) precision is acceptable.
- The effects pipeline processes frames AFTER edit operations are applied, so zoom/keyboard effects work correctly on the edited timeline.
- Undo history should be capped (e.g., 50 operations) to prevent unbounded memory growth.
