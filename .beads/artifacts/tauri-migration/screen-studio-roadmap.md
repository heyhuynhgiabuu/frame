# Screen Studio Feature Analysis & Frame Roadmap

**Date:** 2026-02-06  
**Status:** Planning  
**Goal:** Match Screen Studio's polish and features

---

## Screen Studio UI Analysis

### Header/Toolbar

- **Traffic lights** (close/minimize/maximize)
- **Project folder icon** - Open project folder
- **Trash icon** - Delete recording
- **Project title** - "Spreadsheet Demo.screenstudio"
- **Undo/Redo** arrows
- **Presets dropdown** - Save/load appearance presets
- **Layout toggle** - Show/hide panels
- **Settings gear**
- **Export button** (prominent, purple with sparkle icon)

### Preview Area

- **Gradient wallpaper background** - Beautiful macOS-style gradients
- **Window container** with:
  - Heavy drop shadow
  - Rounded corners (12-16px)
  - Inset padding (space between video and background edge)
- **Webcam bubble** - Circular, bottom-left, with border
- **Cursor** - Separate layer, can be enlarged/smoothed
- **Captions/subtitles** - "show US dollars." speech bubble

### Right Panel (Context-Sensitive)

- **Cursor icon** - Cursor settings
- **Camera icon** - Webcam settings
- **Chat icon** - Captions/transcript
- **Speaker icon** - Audio settings
- **Sliders icon** - Advanced settings
- **Wand icon** - Effects/magic

**Background Section:**

- Tabs: Wallpaper | Gradient | Color | Image
- **Wallpaper grid** - 12+ gradient presets (raycast.com credit)
- **Background blur** slider
- **Padding** slider
- **Reset** button

### Timeline (Multi-Track)

- **Aspect ratio selector** - "Wide 16:9" dropdown
- **Crop button**
- **Transport controls**: Previous | Play/Pause | Next
- **Split tool** (scissors)
- **Speed slider** (1x display)
- **Time markers** - 1s, 2s, 3s, 4s, 5s, 6s

**Tracks:**

1. **Clip track** (orange) - Main video with "7s ⏱1x" label
2. **Zoom track** (blue) - Auto-zoom regions with "Q 2x 🔒 Auto" label
3. **Trim handles** - Yellow markers at start/end

### Bottom Controls (Not in screenshot but from research)

- Keyboard shortcut display
- Click highlight rings
- Motion blur toggle

---

## Complete Feature List

### 🎬 Recording Features

1. **Screen capture** - Full screen, window, or region
2. **Webcam capture** - Circular overlay with position control
3. **System audio** - Record from all apps or selected
4. **Microphone audio** - With noise reduction
5. **iOS device recording** - USB-connected iPhone/iPad
6. **Cursor recording** - Separate metadata track
7. **Click recording** - Mouse down/up events
8. **Keyboard recording** - Shortcut display overlay
9. **Hide desktop icons** - Clean recording mode

### ✨ Visual Effects

1. **Background wallpapers** - Gradient presets (raycast-style)
2. **Background gradients** - Custom gradient editor
3. **Background colors** - Solid color picker
4. **Background images** - Custom image upload
5. **Background blur** - Depth effect
6. **Window shadow** - Deep drop shadow
7. **Window inset/padding** - Space around recording
8. **Window rounded corners** - Configurable radius
9. **Cursor smoothing** - Remove jitter
10. **Cursor enlargement** - Make cursor bigger
11. **Cursor auto-hide** - Hide when static
12. **Cursor loop** - Return to start position
13. **High-res cursor** - Replace with HD version
14. **Click effects** - Visual ripple on click
15. **Motion blur** - Professional animation feel

### 📹 Webcam Features

1. **Shape options** - Circle, square, rectangle
2. **Position presets** - 4 corners
3. **Size slider** - Percentage of video
4. **Border styling** - Color and width
5. **Auto zoom-out** - Avoid cursor overlap

### 🎯 Zoom & Animation

1. **Auto-zoom** - Detect cursor focus areas
2. **Manual zoom** - Click-to-add zoom regions
3. **Zoom timeline** - Drag to adjust duration
4. **Easing options** - Smooth in/out
5. **Scale control** - Zoom level (2x, 3x, etc.)

### ✂️ Editing

1. **Trim** - Cut start/end
2. **Split** - Divide into segments
3. **Speed up** - Accelerate boring parts
4. **Silence detection** - Auto-find gaps
5. **Crop** - Focus on area
6. **Aspect ratio** - 16:9, 4:3, 1:1, 9:16, 21:9

### 🎤 Audio

1. **Waveform visualization** - See audio levels
2. **Volume normalization** - Auto-level voice
3. **Noise reduction** - Remove background noise
4. **Transcript generation** - AI speech-to-text
5. **Captions/subtitles** - Overlay text

### 📤 Export

1. **MP4 export** - Up to 4K 60fps
2. **GIF export** - Optimized file size
3. **WebM export** - Web-friendly
4. **Export presets** - Twitter, YouTube, etc.
5. **Copy to clipboard** - Quick paste
6. **Shareable links** - Cloud upload
7. **Custom resolution** - Any size
8. **Custom FPS** - 24, 30, 60

### 🎨 Presets & Branding

1. **Save presets** - Store all settings
2. **Share presets** - Export/import
3. **Project templates** - Quick start

---

## Frame Implementation Roadmap

### Phase 1: Core Recording (DONE ✅)

- [x] Screen capture (ScreenCaptureKit)
- [x] Video encoding (ffmpeg-sidecar)
- [x] Auto-save & crash recovery
- [x] Basic UI with Tauri + SolidJS

### Phase 2: Beautiful Backgrounds 🎨

**Goal:** Match Screen Studio's wallpaper/container system

- [ ] **Gradient presets** - 12+ beautiful gradients
- [ ] **Gradient editor** - Custom gradient creation
- [ ] **Solid color picker** - Full color wheel
- [ ] **Image background** - Upload custom images
- [ ] **Background blur** - Depth of field effect
- [ ] **Window padding** - Inset control (0-100px)
- [ ] **Window shadow** - Configurable depth/blur/color
- [ ] **Window corners** - Radius slider (0-48px)

**Rust (frame-core):**

```rust
// New modules needed:
packages/core/src/effects/background.rs    // Gradient/image backgrounds
packages/core/src/effects/container.rs     // Padding, shadow, corners
packages/core/src/renderer/compositor.rs   // GPU compositing
```

### Phase 3: Cursor Magic 🖱️

**Goal:** Screen Studio's signature smooth cursor

- [ ] **Cursor metadata recording** - (x, y, timestamp) separate track
- [ ] **Click event recording** - mousedown/mouseup events
- [ ] **Cursor smoothing** - Catmull-Rom spline interpolation
- [ ] **Cursor enlargement** - Scale up in post
- [ ] **Click effect rings** - Visual feedback on click
- [ ] **Auto-hide cursor** - When static > 2s
- [ ] **High-res cursor swap** - Replace with HD version

**Rust (frame-core):**

```rust
packages/core/src/capture/cursor.rs        // Cursor metadata capture
packages/core/src/effects/cursor.rs        // Cursor rendering/smoothing
```

### Phase 4: Smart Zoom 🔍

**Goal:** Auto-zoom like Screen Studio

- [ ] **Manual zoom regions** - Click to add zoom keyframe
- [ ] **Zoom timeline track** - Visualize zoom regions
- [ ] **Auto-zoom detection** - Track active window/focus
- [ ] **Zoom easing** - Smooth in/out animations
- [ ] **Zoom scale control** - 1.5x, 2x, 3x, etc.

**Rust (frame-core):**

```rust
packages/core/src/effects/zoom.rs          // Zoom keyframes & interpolation
packages/core/src/capture/focus_detect.rs  // Window/focus detection
```

### Phase 5: Audio Excellence 🎤

**Goal:** Professional audio like Screen Studio

- [ ] **Waveform visualization** - Real-time + timeline
- [ ] **System audio capture** - All apps or selected
- [ ] **Microphone capture** - With level meter
- [ ] **Noise reduction** - Background noise removal
- [ ] **Volume normalization** - Auto-level
- [ ] **Silence detection** - Find gaps > 2s
- [ ] **Transcript generation** - whisper.cpp integration
- [ ] **Captions overlay** - Subtitle rendering

**Rust (frame-core):**

```rust
packages/core/src/audio/waveform.rs        // Waveform analysis
packages/core/src/audio/noise_reduction.rs // Noise removal
packages/core/src/audio/transcript.rs      // Speech-to-text
```

### Phase 6: Advanced Webcam 📷

**Goal:** Professional webcam overlay

- [ ] **Shape options** - Circle, rounded square, rectangle
- [ ] **Corner positions** - 4 anchor points
- [ ] **Size slider** - 10-40% of video
- [ ] **Border styling** - Width, color, gradient
- [ ] **Auto zoom-out** - Avoid cursor overlap
- [ ] **Background blur** - Focus on face

**Rust (frame-core):**

```rust
packages/core/src/capture/webcam.rs        // Already exists, enhance
packages/core/src/effects/webcam_overlay.rs // Already exists, enhance
```

### Phase 7: Timeline Power ⏱️

**Goal:** Multi-track editing like Screen Studio

- [ ] **Multi-track timeline** - Video, audio, zoom, cursor
- [ ] **Waveform track** - Audio visualization
- [ ] **Zoom track** - Visual zoom regions
- [ ] **Trim handles** - Drag to trim
- [ ] **Split tool** - Cut at playhead
- [ ] **Speed regions** - Speed up/slow down sections
- [ ] **Keyframe system** - Animate any property

**Frontend (SolidJS):**

- Complete timeline rewrite with multi-track support
- Drag-and-drop keyframe editing

### Phase 8: Export Excellence 📤

**Goal:** Match Screen Studio's export quality

- [ ] **4K 60fps export** - High quality
- [ ] **Optimized GIF** - Small file size
- [ ] **Export presets** - Platform-specific
- [ ] **Progress modal** - With preview
- [ ] **Copy to clipboard** - Quick share
- [ ] **Cloud upload** - Shareable links
- [ ] **Batch export** - Multiple formats

### Phase 9: Presets & Polish ✨

**Goal:** Professional feel

- [ ] **Save/load presets** - All settings
- [ ] **Import/export presets** - Share with others
- [ ] **Keyboard shortcuts** - Power user features
- [ ] **Undo/redo** - Full history
- [ ] **Auto-save projects** - Never lose work
- [ ] **Startup screen** - Recent projects

---

## Priority Matrix

| Feature              | Impact | Effort | Priority |
| -------------------- | ------ | ------ | -------- |
| Gradient backgrounds | High   | Low    | P0       |
| Window shadow/inset  | High   | Low    | P0       |
| Cursor smoothing     | High   | Medium | P1       |
| Auto-zoom            | High   | High   | P2       |
| Click effects        | Medium | Low    | P1       |
| Waveform timeline    | Medium | Medium | P2       |
| Noise reduction      | Medium | Medium | P2       |
| Transcription        | Medium | High   | P3       |
| Cloud sharing        | Low    | High   | P3       |

---

## Estimated Timeline

| Phase                | Duration | Cumulative |
| -------------------- | -------- | ---------- |
| Phase 2: Backgrounds | 1 week   | Week 1     |
| Phase 3: Cursor      | 1 week   | Week 2     |
| Phase 4: Zoom        | 2 weeks  | Week 4     |
| Phase 5: Audio       | 2 weeks  | Week 6     |
| Phase 6: Webcam      | 1 week   | Week 7     |
| Phase 7: Timeline    | 2 weeks  | Week 9     |
| Phase 8: Export      | 1 week   | Week 10    |
| Phase 9: Polish      | 2 weeks  | Week 12    |

**Total: ~12 weeks to feature parity with Screen Studio**

---

## Immediate Next Steps

1. **Phase 2: Backgrounds** - Start with gradient presets
   - Port raycast-style gradients to Frame
   - Implement container padding/shadow
   - Build gradient picker UI

2. **Phase 3: Cursor** - Record cursor separately
   - Add cursor metadata capture to frame-core
   - Implement spline smoothing algorithm
   - Add click effect rendering
