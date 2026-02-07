# Tauri + SolidJS Migration Design

**Date:** 2026-02-05  
**Status:** Approved  
**Scope:** Full rewrite of UI layer, keep Rust core

---

## Overview

Migrate Frame's desktop app from iced.rs to Tauri + SolidJS. The Rust core (`frame-core`) remains unchanged - only the UI layer is replaced.

### Why Migrate?

- iced.rs UI is functional but visually limited
- SolidJS provides richer UI capabilities with Tailwind CSS
- Better debugging with DevTools
- Hot reload during development
- Screen Studio-quality aesthetics achievable

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Tauri App (apps/desktop-tauri)          │
├─────────────────────────────────────────────────────────────┤
│  SolidJS Frontend (src/)                                    │
│  ├── components/     # UI components (Timeline, Effects...) │
│  ├── stores/         # Solid stores for state management    │
│  └── styles/         # Tailwind CSS                         │
├─────────────────────────────────────────────────────────────┤
│  Tauri Commands (src-tauri/)                                │
│  └── commands.rs     # Thin wrappers calling frame-core     │
├─────────────────────────────────────────────────────────────┤
│  frame-core (packages/core) - UNCHANGED                     │
│  ├── capture/        # Screen capture (ScreenCaptureKit)    │
│  ├── encoder/        # Video encoding (ffmpeg-sidecar)      │
│  ├── effects/        # Shadow, inset, webcam overlay        │
│  ├── auto_save.rs    # Auto-save & crash recovery           │
│  └── export_preset.rs # 7 export presets                    │
└─────────────────────────────────────────────────────────────┘
```

### Key Decisions

| Decision     | Choice          | Rationale                                      |
| ------------ | --------------- | ---------------------------------------------- |
| UI Framework | SolidJS         | Fine-grained reactivity, small bundle, fast    |
| Styling      | Tailwind CSS v4 | Utility-first, easy dark theme                 |
| Desktop      | Tauri 2.x       | Native Rust integration, small binary          |
| Core Reuse   | 100%            | All capture/encoding logic stays in frame-core |
| Old App      | Deprecate       | Keep `apps/desktop` but stop development       |

---

## UI Design - Screen Studio Style

### Recording View

```
┌────────────────────────────────────────────────────────────┐
│  [● Record] [📷 Webcam: Off ▾] [🎤 Audio: System ▾]        │
│                                                            │
│     ┌─────────────────────────────────────┐               │
│     │                                     │               │
│     │     Live Preview / Screen Select    │               │
│     │                                     │               │
│     └─────────────────────────────────────┘               │
│                                                            │
│  Source: [Full Screen ▾] [Display 1 ▾]                    │
└────────────────────────────────────────────────────────────┘
```

### Editor View

```
┌────────────────────────────────────────────────────────────┐
│  ┌─────────────────────────────┐  ┌─────────────────────┐  │
│  │                             │  │ EFFECTS PANEL       │  │
│  │      Video Preview          │  │ ☑ Shadow            │  │
│  │      (with effects)         │  │ ☑ Inset             │  │
│  │                             │  │ ☐ Rounded           │  │
│  │                             │  │ Aspect: 16:9 ▾      │  │
│  └─────────────────────────────┘  │ Webcam: ◐ ▾         │  │
│                                   └─────────────────────┘  │
│  ┌────────────────────────────────────────────────────────┐│
│  │ ▶ 00:00 ═══════●════════════════════════ 00:30        ││
│  │     Timeline with trim handles                         ││
│  └────────────────────────────────────────────────────────┘│
│  [Export: MP4 ▾] [Preset: Twitter ▾]        [📋] [Export] │
└────────────────────────────────────────────────────────────┘
```

### Component Tree

```
App
├── RecordingView
│   ├── SourceSelector (screen/window/region)
│   ├── WebcamToggle
│   ├── AudioSelector
│   └── RecordButton
├── EditorView
│   ├── VideoPreview
│   ├── EffectsPanel
│   │   ├── ShadowControl
│   │   ├── InsetControl
│   │   ├── AspectRatioSelector
│   │   └── WebcamOverlayControl
│   ├── Timeline
│   │   ├── Playhead
│   │   └── TrimHandles
│   └── ExportBar
│       ├── FormatSelector
│       ├── PresetSelector
│       └── ExportButton
└── Shared
    ├── Toast
    └── Modal
```

---

## Tauri Commands

Thin wrappers around existing `frame-core` functions:

### Recording

```rust
#[tauri::command]
async fn start_recording(config: RecordingConfig) -> Result<String, String>
// → RecordingService::start_recording()

#[tauri::command]
async fn stop_recording() -> Result<RecordingResult, String>
// → RecordingService::stop_recording()

#[tauri::command]
fn get_capture_sources() -> Result<Vec<CaptureSource>, String>
// → capture::list_sources()
```

### Effects

```rust
#[tauri::command]
fn apply_shadow(config: ShadowConfig) -> Result<(), String>
// → effects::shadow::apply_shadow()

#[tauri::command]
fn apply_inset(config: InsetConfig) -> Result<(), String>
// → effects::inset::apply_inset()

#[tauri::command]
fn calculate_aspect_ratio(input: AspectInput) -> AspectOutput
// → effects::aspect_ratio::calculate()

#[tauri::command]
fn apply_webcam_overlay(config: WebcamOverlayConfig) -> Result<(), String>
// → effects::webcam_overlay::composite()
```

### Export

```rust
#[tauri::command]
async fn export_video(config: ExportConfig) -> Result<PathBuf, String>
// → Encoder + finalize

#[tauri::command]
async fn export_gif(config: GifConfig) -> Result<PathBuf, String>
// → encoder::gif::encode_gif()

#[tauri::command]
fn get_export_presets() -> Vec<ExportPreset>
// → export_preset::default_presets()

#[tauri::command]
fn copy_to_clipboard(path: PathBuf) -> Result<(), String>
// → arboard crate
```

### State Management

```rust
struct AppState {
    recording_service: Arc<Mutex<RecordingService>>,
}
```

---

## Project Structure

```
apps/desktop-tauri/
├── src-tauri/                    # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands.rs
│   │   └── state.rs
│   └── icons/
│
├── src/                          # SolidJS frontend
│   ├── index.html
│   ├── index.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── recording/
│   │   │   ├── RecordButton.tsx
│   │   │   ├── SourceSelector.tsx
│   │   │   └── WebcamToggle.tsx
│   │   ├── editor/
│   │   │   ├── VideoPreview.tsx
│   │   │   ├── Timeline.tsx
│   │   │   ├── EffectsPanel.tsx
│   │   │   └── ExportBar.tsx
│   │   └── shared/
│   │       ├── Button.tsx
│   │       ├── Dropdown.tsx
│   │       └── Toast.tsx
│   ├── stores/
│   │   ├── recording.ts
│   │   ├── editor.ts
│   │   └── export.ts
│   ├── lib/
│   │   └── tauri.ts
│   └── styles/
│       └── globals.css
│
├── package.json
├── vite.config.ts
└── tailwind.config.ts
```

---

## Tech Stack

### Frontend

| Package         | Purpose               |
| --------------- | --------------------- |
| solid-js        | Reactive UI framework |
| @solidjs/router | Client-side routing   |
| tailwindcss v4  | Utility-first CSS     |
| @tauri-apps/api | Tauri JS bindings     |
| motion          | Animations            |

### Backend (Tauri)

| Package    | Purpose                   |
| ---------- | ------------------------- |
| tauri 2.x  | Desktop framework         |
| frame-core | Existing capture/encoding |
| serde      | JSON serialization        |
| tokio      | Async runtime             |

---

## Implementation Plan

### Phase 1: Scaffold (Day 1)

- [ ] Create `apps/desktop-tauri/` with Tauri + SolidJS template
- [ ] Configure workspace to include new app
- [ ] Add `frame-core` dependency
- [ ] Verify build: `cargo tauri dev`

### Phase 2: Core Commands (Day 2-3)

- [ ] Implement recording commands
- [ ] Implement source listing
- [ ] Add AppState with RecordingService
- [ ] Test recording via console

### Phase 3: Recording UI (Day 4-5)

- [ ] Build RecordButton component
- [ ] Build SourceSelector
- [ ] Build WebcamToggle and AudioSelector
- [ ] Wire to Tauri commands

### Phase 4: Editor UI (Day 6-8)

- [ ] Build VideoPreview component
- [ ] Build Timeline with playhead
- [ ] Build EffectsPanel
- [ ] Build WebcamOverlayControl
- [ ] Wire effects to commands

### Phase 5: Export (Day 9-10)

- [ ] Build ExportBar
- [ ] Implement export commands
- [ ] Add export progress modal
- [ ] Implement clipboard copy

### Phase 6: Polish (Day 11-14)

- [ ] Dark theme (Screen Studio aesthetic)
- [ ] Animations
- [ ] Error handling & toasts
- [ ] Keyboard shortcuts
- [ ] Window controls
- [ ] App icon and packaging

---

## Success Criteria

- [ ] Recording works: start, stop, saves video
- [ ] All effects work: shadow, inset, aspect ratio, webcam overlay
- [ ] Export works: MP4, GIF, all 7 presets
- [ ] Clipboard copy works
- [ ] UI matches Screen Studio aesthetic
- [ ] Performance: <50MB RAM idle, <200MB recording
- [ ] Build size: <30MB DMG

---

## Migration Path

1. **Parallel development**: New app in `apps/desktop-tauri/`, old app unchanged
2. **Feature parity**: Match all iced.rs features before switching
3. **Deprecation**: Mark `apps/desktop` as deprecated in README
4. **Cleanup**: Remove iced.rs app after stable release

---

## Open Questions

None - design approved and ready for implementation.
