# Changelog

All notable changes to Frame will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial project structure and monorepo setup
- Basic iced.rs desktop application with state management
- Core library with capture abstraction, encoder stubs, and project management
- UI component library with reusable iced.rs components
- GPU renderer placeholder with wgpu
- Project file format with auto-save support
- Error handling framework with custom FrameError type
- Development tooling: Biome, Just, Bun workspace
- Documentation: API docs, setup guide, contributing guide

### Technical

- Set up Rust workspace with 4 crates (desktop, core, ui, renderer)
- Configured Biome for JavaScript/TypeScript linting and formatting
- Configured cargo fmt for Rust formatting
- Created design document for Phase 2 planning
- Set up beads task tracking for Phase 2 implementation

## [0.1.0] - TBD

### Added

- Screen recording with ScreenCaptureKit (macOS 12.3+)
- Audio capture (microphone + system audio via BlackHole)
- Basic timeline UI for reviewing recordings
- MP4 export with H.264/H.265 encoding
- Recording controls (start, stop)
- Real-time preview during recording
- Project auto-save during recording

## Future Releases

### [0.2.0] - Phase 3: Polish & Effects

- Cursor zoom and smoothing
- Webcam overlay
- Advanced timeline editing (trim, cut, split)
- Keyboard shortcut display
- Background customization
- Performance optimizations

### [0.3.0] - Phase 4: Pro Features

- Cloud sync and shareable links
- Team workspaces
- AI-powered features (auto-zoom, silence removal)
- Advanced export presets
- Commercial licensing

### [1.0.0] - Stable Release

- Windows and Linux support
- Plugin system
- Hardware acceleration
- Professional editing features
- Enterprise features

---

## Legend

- **Added** - New features
- **Changed** - Changes to existing functionality
- **Deprecated** - Soon-to-be removed features
- **Removed** - Removed features
- **Fixed** - Bug fixes
- **Security** - Security improvements
- **Technical** - Internal changes not visible to users
