# Frame - Design Document

**Status**: Phase 1 Complete ✅ | **Last Updated**: 2026-02-05

---

## 1. Core Concept & Positioning

**Frame** is an open-core screen recorder built for developers who create product demos, tutorials, and documentation. It combines Screen Studio's "beautiful by default" philosophy with modern Rust performance and cross-platform potential.

### Key Differentiators

- **Developer-first**: Keyboard shortcuts, CLI integration, code-focused workflows
- **Open Core**: Core recording free forever; advanced features (cloud sync, team sharing, AI enhancements) in paid tier
- **Performance**: Rust + GPU-accelerated encoding for buttery-smooth 4K60 recordings
- **Extensible**: Plugin system for custom effects and export formats

### Target Audience

Developers creating:

- Product demos and walkthroughs
- Tutorial videos and courses
- Documentation with video
- Open source project showcases

---

## 2. Monorepo Structure

```
frame/
├── apps/
│   ├── desktop/              # Main iced.rs application
│   │   ├── src/
│   │   │   ├── main.rs       # Entry point
│   │   │   ├── ui/           # iced.rs UI components
│   │   │   ├── recording/    # Screen capture logic
│   │   │   ├── export/       # Video encoding & export
│   │   │   └── settings/     # Preferences & presets
│   │   └── Cargo.toml
│   │
│   └── web/                  # Optional web viewer/sharing
│       ├── src/
│       └── package.json
│
├── packages/
│   ├── core/                 # Shared Rust library
│   │   ├── src/
│   │   │   ├── capture/      # Screen/audio capture abstractions
│   │   │   ├── encoder/      # Video encoding (ffmpeg/rust-ffmpeg)
│   │   │   ├── effects/      # Zoom, cursor smoothing, etc.
│   │   │   └── project/      # Project file format
│   │   └── Cargo.toml
│   │
│   ├── ui-components/        # Reusable iced.rs components
│   │   ├── src/
│   │   └── Cargo.toml
│   │
│   └── renderer/             # GPU-accelerated rendering
│       ├── src/
│       └── Cargo.toml
│
├── plugins/                  # Plugin system
│   └── (community plugins)
│
├── tooling/
│   ├── biome/               # Shared Biome config
│   └── scripts/             # Build & release scripts
│
├── Cargo.toml               # Workspace root
├── bun.lockb                # Bun workspace lockfile
└── package.json             # Workspace root for JS tooling
```

---

## 3. Tech Stack

### Backend (Rust)

| Component        | Library                      | Rationale                                   |
| ---------------- | ---------------------------- | ------------------------------------------- |
| GUI Framework    | `iced`                       | Elm architecture, type-safe, cross-platform |
| Screen Capture   | `screencapturekit` (macOS)   | Native macOS capture, high performance      |
|                  | `x11rb` + `pipewire` (Linux) | Future Linux support                        |
|                  | `windows-capture` (Windows)  | Future Windows support                      |
| Video Encoding   | `ffmpeg-next`                | Industry standard, hardware acceleration    |
| Audio Processing | `cpal` + `rubato`            | Cross-platform audio I/O                    |
| State Management | Custom + `serde`             | Project files, settings                     |
| GPU Compute      | `wgpu`                       | Future GPU-accelerated effects              |

### Frontend (TypeScript/SolidJS) - For Web Components

| Component     | Library          | Rationale                             |
| ------------- | ---------------- | ------------------------------------- |
| Framework     | `solid-js`       | Fine-grained reactivity, small bundle |
| UI Components | `kobalte`        | Accessible, headless UI primitives    |
| Styling       | `tailwindcss` v4 | Utility-first, design system friendly |
| Validation    | `zod`            | Runtime type safety                   |
| Build Tool    | `bun`            | Fast, native TypeScript               |

### Tooling

| Tool    | Purpose                                           |
| ------- | ------------------------------------------------- |
| `biome` | Linting & formatting (replaces ESLint + Prettier) |
| `cargo` | Rust build & package management                   |
| `bun`   | JS/TS tooling, scripts                            |
| `just`  | Task runner (Makefile alternative)                |

---

## 4. Core Features

### Free Tier (Open Source)

- [ ] Screen recording (full screen, window, region)
- [ ] Webcam overlay with smart positioning
- [ ] Microphone + system audio recording
- [ ] Automatic cursor zoom & smoothing
- [ ] Basic timeline editing (trim, cut, split)
- [ ] Export to MP4 (H.264/H.265)
- [ ] Keyboard shortcut display
- [ ] Custom backgrounds & padding
- [ ] Project file format (`.frame`)

### Pro Tier (Paid)

- [ ] Cloud sync & shareable links
- [ ] Team workspaces
- [ ] Advanced AI features (auto-zoom suggestions, silence removal)
- [ ] Custom export presets
- [ ] Priority support
- [ ] Commercial license

---

## 5. Data Flow Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        User Interaction                      │
│                    (iced.rs UI / Shortcuts)                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Recording Controller                      │
│         (Manages capture sessions, state machine)            │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  Screen Capture │  │  Audio Capture  │  │  Input Capture  │
│  (screencapture │  │  (cpal + system)│  │  (cursor, keys) │
│   kit / etc)    │  │                 │  │                 │
└─────────────────┘  └─────────────────┘  └─────────────────┘
              │               │               │
              └───────────────┼───────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Frame Buffer Queue                        │
│              (Ring buffer, minimal latency)                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Effects Pipeline                          │
│    (Zoom detection, cursor smoothing, motion blur)           │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Video Encoder                             │
│         (ffmpeg, hardware acceleration)                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Output File / Preview                     │
└─────────────────────────────────────────────────────────────┘
```

---

## 6. Development Phases

### Phase 1: Foundation (Weeks 1-4) ✅ COMPLETE

- [x] Project scaffolding & monorepo setup
- [x] Basic iced.rs window & UI shell
- [x] Screen capture abstraction (macOS first)
- [x] Simple recording to file

### Phase 2: Core Recording (Weeks 5-8) 🔄 IN PROGRESS

- [ ] ScreenCaptureKit integration (macOS native capture)
- [ ] Audio capture (mic + system)
- [ ] Basic timeline UI
- [ ] Export to MP4

### Phase 3: Polish & Effects (Weeks 9-12)

- [ ] Cursor zoom & smoothing
- [ ] Keyboard shortcut display
- [ ] Background customization
- [ ] Project file format

### Phase 4: Pro Features (Weeks 13-16)

- [ ] Cloud sync infrastructure (Supabase + Cloudflare)
- [ ] Shareable links
- [ ] Team workspaces
- [ ] Payment/licensing system

---

## 7. Technical Decisions

### Platform Priority

1. **macOS first** (primary target, Screen Studio's platform)
2. **Linux** (second priority, developer-friendly)
3. **Windows** (third priority, broader market)

### Audio Routing Strategy

**Decision**: Use BlackHole/virtual audio driver for system audio capture on macOS

**Rationale**: Building a custom audio driver is complex and requires kernel extensions (now deprecated on macOS). BlackHole is the industry standard and provides seamless UX.

### Cloud Infrastructure

**Decision**: Hybrid approach (A + C)

- **Supabase**: Postgres, Auth, Realtime, Storage for core data
- **Cloudflare Workers**: Edge compute for video processing, shareable links
- **Cloudflare R2**: S3-compatible storage for video files (cheaper egress)

### Plugin System

**Decision**: WASM modules (Option B)

**Rationale**: Sandboxed, cross-platform, can be written in any language that compiles to WASM. Better security than dynamic libraries.

---

## 8. Open Questions & Future Considerations

### macOS Version Support

- **ScreenCaptureKit** requires macOS 12.3+
- **CGDisplayStream** for older versions (deprecated but works)
- **Decision**: Target macOS 12.3+ only (Monterey and later)

### Video Codecs

- **H.264**: Universal compatibility
- **H.265**: Better compression, hardware accelerated
- **AV1**: Future-proof, but slow encoding
- **ProRes**: Professional editing workflow

### AI Features (Pro Tier)

- Auto-zoom suggestions based on cursor movement
- Silence detection and removal
- Transcription for captions
- Smart chapter markers

---

## 9. Success Metrics

### Technical

- [ ] 4K60 recording with <5% CPU usage on M1 Mac
- [ ] <100ms latency for preview
- [ ] Export 1-minute 1080p video in <30 seconds

### User

- [ ] Time from install to first recording: <2 minutes
- [ ] NPS score >50
- [ ] 1000+ GitHub stars in first 6 months

### Business

- [ ] 100+ Pro tier subscribers in first year
- [ ] 10+ enterprise customers
- [ ] Sustainable open-source community

---

## 10. References

- **Inspiration**: [Screen Studio](https://screen.studio/), [Kiru](https://getkiru.app/)
- **GUI Framework**: [iced.rs](https://iced.rs/)
- **Screen Capture**: [ScreenCaptureKit](https://developer.apple.com/documentation/screencapturekit)
- **Architecture**: Elm Architecture, The Rust Book

---

_This document is a living specification. Update as the project evolves._
