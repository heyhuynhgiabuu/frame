# Phase 5: MVP Polish - Technical Specification

**Bead:** bd-2g1
**Type:** Epic
**Status:** In Progress
**Created:** 2026-02-05

## Architecture Overview

This spec extends the existing Frame architecture with 6 new capabilities:

```
┌─────────────────────────────────────────────────────────────────┐
│                      Desktop App (iced.rs)                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │  Auth UI    │  │ Region      │  │ Export      │              │
│  │ (webcam)    │  │ Selector    │  │ Presets UI  │              │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘              │
└─────────┼────────────────┼────────────────┼─────────────────────┘
          │                │                │
          ▼                ▼                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      UI Components (iced)                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │ Webcam      │  │ Region      │  │ Export      │              │
│  │ Settings    │  │ Overlay     │  │ Dialog      │              │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘              │
└─────────┼────────────────┼────────────────┼─────────────────────┘
          │                │                │
          ▼                ▼                ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Core Library                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │ Webcam      │  │ Region      │  │ Export      │              │
│  │ Capture     │  │ Capture     │  │ Presets     │              │
│  └──────┬──────┘  └─────────────┘  └──────┬──────┘              │
│         │                                  │                     │
│  ┌──────▼──────────────────────────────────▼────────────────┐   │
│  │                 Effects Pipeline                          │   │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐  │   │
│  │  │Webcam  │ │Aspect  │ │Shadow  │ │Inset   │ │Existing│  │   │
│  │  │Overlay │ │Ratio   │ │Effect  │ │Effect  │ │Effects │  │   │
│  │  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘  │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│  ┌───────────────────────────▼──────────────────────────────┐   │
│  │                    Encoder                                │   │
│  │  ┌────────────────┐  ┌────────────────┐                  │   │
│  │  │ MP4 (existing) │  │ GIF (gifski)   │                  │   │
│  │  └────────────────┘  └────────────────┘                  │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Module Specifications

### 1. Webcam Capture (`packages/core/src/capture/webcam.rs`)

**Purpose:** Capture video from connected webcam devices.

**Dependencies:**

```toml
nokhwa = { version = "0.10", features = ["input-avfoundation"], optional = true }
```

**Key Types:**

```rust
/// Webcam capture configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebcamConfig {
    pub device_id: Option<String>,  // None = default camera
    pub resolution: Resolution,      // Desired resolution
    pub frame_rate: u32,            // Target FPS
}

/// Information about available webcam
#[derive(Debug, Clone)]
pub struct WebcamDevice {
    pub id: String,
    pub name: String,
    pub resolutions: Vec<Resolution>,
}

/// Webcam capture service
pub struct WebcamCapture {
    device: Option<Camera>,
    config: WebcamConfig,
    running: Arc<AtomicBool>,
}

impl WebcamCapture {
    pub fn list_devices() -> FrameResult<Vec<WebcamDevice>>;
    pub fn new(config: WebcamConfig) -> FrameResult<Self>;
    pub async fn start(&mut self) -> FrameResult<()>;
    pub async fn stop(&mut self) -> FrameResult<()>;
    pub async fn next_frame(&mut self) -> FrameResult<Option<Frame>>;
}
```

**Feature Flag:** `webcam`

### 2. Webcam Overlay (`packages/core/src/effects/webcam_overlay.rs`)

**Purpose:** Composite webcam frame onto screen recording.

**Key Types:**

```rust
/// Webcam overlay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebcamOverlayConfig {
    pub enabled: bool,
    pub position: WebcamPosition,
    pub shape: WebcamShape,
    pub size: WebcamSize,
    pub border_color: Option<Color>,
    pub border_width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebcamPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebcamShape {
    Circle,
    RoundedRect { corner_radius: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebcamSize {
    Small,   // 15% of output width
    Medium,  // 25% of output width
    Large,   // 35% of output width
}

pub struct WebcamOverlay {
    config: WebcamOverlayConfig,
    mask_cache: Option<Vec<u8>>,  // Cached shape mask
}

impl WebcamOverlay {
    pub fn new(config: WebcamOverlayConfig) -> Self;
    pub fn composite(&mut self, screen: &mut Frame, webcam: &Frame) -> FrameResult<()>;
}
```

### 3. Aspect Ratio (`packages/core/src/effects/aspect_ratio.rs`)

**Purpose:** Calculate output dimensions and letterbox/pillarbox areas.

**Key Types:**

```rust
/// Supported aspect ratios
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AspectRatio {
    Original,           // Keep source aspect
    Horizontal16x9,     // 16:9 (1920x1080)
    Vertical9x16,       // 9:16 (1080x1920)
    Square,             // 1:1 (1080x1080)
    Classic4x3,         // 4:3 (1440x1080)
    Custom { width: u32, height: u32 },
}

/// Content alignment within aspect ratio frame
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum ContentAlignment {
    #[default]
    Center,
    Top,
    Bottom,
    Left,
    Right,
}

/// Result of aspect ratio calculation
pub struct AspectResult {
    pub output_width: u32,
    pub output_height: u32,
    pub content_x: u32,      // X offset for content
    pub content_y: u32,      // Y offset for content
    pub content_width: u32,  // Scaled content width
    pub content_height: u32, // Scaled content height
    pub letterbox: Option<LetterboxInfo>,
}

pub struct LetterboxInfo {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

impl AspectRatio {
    /// Calculate dimensions for given base width
    pub fn dimensions(&self, base_width: u32) -> (u32, u32);

    /// Calculate full aspect result for content placement
    pub fn calculate(
        &self,
        source_width: u32,
        source_height: u32,
        alignment: ContentAlignment,
    ) -> AspectResult;
}
```

### 4. Export Presets (`packages/core/src/export_preset.rs`)

**Purpose:** Define and manage export configuration presets.

**Key Types:**

```rust
/// Video codec options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
    ProRes,
}

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportOutputFormat {
    Mp4,
    Mov,
    Gif,
}

/// Quality preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityPreset {
    Low,      // 720p, 4Mbps
    Medium,   // 1080p, 8Mbps
    High,     // 1080p, 15Mbps
    Ultra,    // 4K, 25Mbps
    Custom,
}

/// Complete export preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPreset {
    pub id: String,              // Unique identifier
    pub name: String,            // Display name
    pub builtin: bool,           // Cannot be deleted if true
    pub codec: VideoCodec,
    pub format: ExportOutputFormat,
    pub quality: QualityPreset,
    pub resolution: Option<Resolution>,  // None = source resolution
    pub bitrate_kbps: Option<u32>,       // None = auto from quality
    pub frame_rate: Option<u32>,         // None = source FPS
    pub aspect_ratio: AspectRatio,
    // GIF-specific
    pub gif_fps: Option<u32>,            // For GIF export
    pub gif_max_colors: Option<u32>,     // Palette size (2-256)
}

/// Built-in presets
pub fn builtin_presets() -> Vec<ExportPreset>;

/// Preset manager
pub struct PresetManager {
    presets: Vec<ExportPreset>,
    custom_path: PathBuf,
}

impl PresetManager {
    pub fn load() -> FrameResult<Self>;
    pub fn all(&self) -> &[ExportPreset];
    pub fn get(&self, id: &str) -> Option<&ExportPreset>;
    pub fn save_custom(&mut self, preset: ExportPreset) -> FrameResult<()>;
    pub fn delete_custom(&mut self, id: &str) -> FrameResult<()>;
}
```

**Built-in Presets:**

| Name      | Codec | Format | Resolution | Bitrate | Notes            |
| --------- | ----- | ------ | ---------- | ------- | ---------------- |
| Web       | H.264 | MP4    | 1080p      | 8 Mbps  | General web use  |
| YouTube   | H.264 | MP4    | 4K         | 20 Mbps | High quality     |
| Twitter/X | H.264 | MP4    | 720p       | 5 Mbps  | Max 2:20 warning |
| Instagram | H.264 | MP4    | 1080x1080  | 8 Mbps  | Square           |
| GIF       | N/A   | GIF    | 480p       | N/A     | 10fps, optimized |

### 5. GIF Encoder (`packages/core/src/encoder/gif.rs`)

**Purpose:** High-quality GIF export using gifski.

**Dependencies:**

```toml
gifski = { version = "1.12", optional = true }
```

**Key Types:**

```rust
/// GIF encoder configuration
pub struct GifEncoderConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub quality: u8,        // 1-100
    pub max_colors: u32,    // 2-256
    pub loop_count: Option<u32>,  // None = infinite
}

pub struct GifEncoder {
    config: GifEncoderConfig,
    collector: Option<Collector>,
    writer: Option<Writer>,
}

impl GifEncoder {
    pub fn new(config: GifEncoderConfig, output: &Path) -> FrameResult<Self>;
    pub fn add_frame(&mut self, frame: &Frame, timestamp: Duration) -> FrameResult<()>;
    pub fn finish(self) -> FrameResult<PathBuf>;
}
```

**Feature Flag:** `gif`

### 6. Shadow Effect (`packages/core/src/effects/shadow.rs`)

**Purpose:** Apply drop shadow to video frame.

**Key Types:**

```rust
/// Shadow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowConfig {
    pub enabled: bool,
    pub offset_x: f32,      // Pixels
    pub offset_y: f32,      // Pixels
    pub blur_radius: f32,   // Gaussian blur sigma
    pub color: Color,       // Shadow color with alpha
    pub spread: f32,        // Shadow expansion
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            offset_x: 0.0,
            offset_y: 8.0,
            blur_radius: 20.0,
            color: Color::rgba_u8(0, 0, 0, 100),
            spread: 0.0,
        }
    }
}

pub struct ShadowEffect {
    config: ShadowConfig,
    blur_buffer: Vec<u8>,  // Reusable buffer for blur
}

impl ShadowEffect {
    pub fn new(config: ShadowConfig) -> Self;
    pub fn apply(&mut self, frame: &mut Frame, corner_radius: f32) -> FrameResult<()>;
}
```

### 7. Inset Effect (`packages/core/src/effects/inset.rs`)

**Purpose:** Apply depth/inset effect to video edges.

**Key Types:**

```rust
/// Inset configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsetConfig {
    pub enabled: bool,
    pub intensity: f32,     // 0.0-1.0
    pub style: InsetStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsetStyle {
    Light,   // Light inner shadow (pressed look)
    Dark,    // Dark inner shadow (inset look)
    Subtle,  // Very subtle depth
}

impl Default for InsetConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.3,
            style: InsetStyle::Subtle,
        }
    }
}

pub struct InsetEffect {
    config: InsetConfig,
}

impl InsetEffect {
    pub fn new(config: InsetConfig) -> Self;
    pub fn apply(&mut self, frame: &mut Frame, corner_radius: f32) -> FrameResult<()>;
}
```

### 8. Region Selection (`packages/ui-components/src/components/region_selector.rs`)

**Purpose:** Interactive overlay for selecting recording region.

**Key Types:**

```rust
/// Selection state
#[derive(Debug, Clone)]
pub struct RegionSelection {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Region selector widget
pub struct RegionSelector {
    selection: Option<RegionSelection>,
    dragging: bool,
    drag_start: Option<(i32, i32)>,
    resize_handle: Option<ResizeHandle>,
}

#[derive(Debug, Clone, Copy)]
pub enum ResizeHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Debug, Clone)]
pub enum RegionMessage {
    StartDrag { x: i32, y: i32 },
    Drag { x: i32, y: i32 },
    EndDrag,
    Confirm,
    Cancel,
}
```

### 9. Clipboard Integration (`apps/desktop/src/clipboard.rs`)

**Purpose:** Copy exported files to system clipboard.

**Dependencies:**

```toml
arboard = "3.2"
```

**Key Types:**

```rust
use arboard::Clipboard;

pub struct ClipboardManager {
    clipboard: Clipboard,
}

impl ClipboardManager {
    pub fn new() -> FrameResult<Self>;

    /// Copy file reference to clipboard (for MP4/MOV)
    pub fn copy_file(&mut self, path: &Path) -> FrameResult<()>;

    /// Copy image data to clipboard (for GIF)
    pub fn copy_image(&mut self, data: &[u8], width: u32, height: u32) -> FrameResult<()>;
}
```

## Pipeline Integration

Update `EffectsConfig` to include new effects:

```rust
// packages/core/src/effects/mod.rs

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EffectsConfig {
    // Existing
    pub zoom: ZoomConfig,
    pub keyboard: KeyboardConfig,
    pub background: Background,

    // New in Phase 5
    pub webcam: WebcamOverlayConfig,
    pub shadow: ShadowConfig,
    pub inset: InsetConfig,
    pub aspect_ratio: AspectRatio,
}
```

Update `IntegratedPipeline` to process new effects:

```rust
// packages/core/src/effects/pipeline.rs

impl IntegratedPipeline {
    pub fn process_frame(&mut self, frame: Frame, webcam_frame: Option<Frame>) -> FrameResult<ProcessedFrame> {
        let mut frame = frame;

        // 1. Apply zoom (existing)
        // 2. Apply cursor effects (existing)
        // 3. Apply shadow (new)
        if self.config.shadow.enabled {
            self.shadow_effect.apply(&mut frame, self.config.background.corner_radius)?;
        }

        // 4. Apply background (existing, modified for aspect ratio)
        // 5. Apply inset (new)
        if self.config.inset.enabled {
            self.inset_effect.apply(&mut frame, self.config.background.corner_radius)?;
        }

        // 6. Apply webcam overlay (new)
        if let Some(webcam) = webcam_frame {
            if self.config.webcam.enabled {
                self.webcam_overlay.composite(&mut frame, &webcam)?;
            }
        }

        // 7. Generate keyboard badges (existing)

        Ok(ProcessedFrame { frame, keyboard_badges })
    }
}
```

## Feature Flags

```toml
[features]
default = ["capture"]
capture = ["screencapturekit", "core-graphics", "core-foundation", "cpal", "rubato"]
encoding = ["ffmpeg-sidecar", "hound"]
encoding-libav = ["ffmpeg-next"]
webcam = ["nokhwa"]
gif = ["gifski"]
pro = []
```

## Dependency Changes

### packages/core/Cargo.toml

```toml
[dependencies]
# ... existing deps ...

# Webcam capture (optional)
nokhwa = { version = "0.10", features = ["input-avfoundation"], optional = true }

# GIF encoding (optional)
gifski = { version = "1.12", optional = true }

[features]
webcam = ["nokhwa"]
gif = ["gifski"]
```

### apps/desktop/Cargo.toml

```toml
[dependencies]
# ... existing deps ...

# Clipboard access
arboard = "3.2"
```

## File Structure

```
packages/core/src/
├── capture/
│   ├── mod.rs          # Update: add webcam re-export
│   ├── webcam.rs       # NEW: webcam capture
│   └── ...
├── effects/
│   ├── mod.rs          # Update: add new configs
│   ├── webcam_overlay.rs # NEW: webcam compositing
│   ├── aspect_ratio.rs   # NEW: aspect ratio calc
│   ├── shadow.rs         # NEW: drop shadow
│   ├── inset.rs          # NEW: inset effect
│   ├── pipeline.rs       # Update: integrate new effects
│   └── ...
├── encoder/
│   ├── mod.rs          # Update: add gif module
│   ├── gif.rs          # NEW: GIF encoder
│   └── ...
├── export_preset.rs    # NEW: preset system
└── ...

packages/ui-components/src/components/
├── mod.rs              # Update: add new components
├── webcam_settings.rs  # NEW: webcam config UI
├── region_selector.rs  # NEW: region selection overlay
├── export_dialog.rs    # Update: add preset selector
├── settings_panel.rs   # Update: add shadow/inset controls
└── ...

apps/desktop/src/
├── clipboard.rs        # NEW: clipboard integration
├── app.rs             # Update: integrate new features
└── ...
```

## Testing Strategy

### Unit Tests

Each module should have unit tests:

```rust
#[cfg(test)]
mod tests {
    // Aspect ratio calculations
    #[test]
    fn test_aspect_ratio_16x9() { ... }

    // Shadow effect
    #[test]
    fn test_shadow_bounds() { ... }

    // Export preset serialization
    #[test]
    fn test_preset_roundtrip() { ... }
}
```

### Integration Tests

```rust
// tests/phase5_integration.rs

#[test]
fn test_webcam_overlay_compositing() { ... }

#[test]
fn test_gif_export_pipeline() { ... }

#[test]
fn test_export_with_aspect_ratio() { ... }
```

## Acceptance Criteria

1. `cargo build -p frame-core --features webcam` succeeds
2. `cargo build -p frame-core --features gif` succeeds
3. `cargo build -p frame-desktop` succeeds
4. `cargo test --workspace` passes
5. `cargo clippy --workspace -- -D warnings` passes
6. Webcam overlay visible at all 4 corner positions
7. Aspect ratio export produces correct dimensions
8. GIF export produces playable GIF <10MB for 10s
9. Shadow/inset effects render correctly
10. Clipboard copy works for MP4 and GIF
