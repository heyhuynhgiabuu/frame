
---
Generated: 2026-02-05T14:17:05.288Z
---

# Delegation Packet

- TASK: core-5 - Export preset system
- EXPECTED OUTCOME: ExportPreset struct with codec, resolution, bitrate, format, fps fields; PresetManager for load/save; QualityPreset enum; serialization to JSON
- MUST DO:
- Use serde for serialization; follow error handling patterns with FrameResult; add pub mod export_preset to lib.rs
- MUST NOT DO:
- Don't implement built-in presets yet (that's core-6); don't add GIF support yet
- ACCEPTANCE CHECKS:
- cargo build -p frame-core
- cargo test -p frame-core
- cargo clippy -p frame-core -- -D warnings
- CONTEXT:
Create packages/core/src/export_preset.rs with VideoCodec, ExportOutputFormat, ExportPreset, PresetManager
