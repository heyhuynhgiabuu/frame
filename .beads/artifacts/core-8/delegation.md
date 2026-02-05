
---
Generated: 2026-02-05T14:17:09.151Z
---

# Delegation Packet

- TASK: core-8 - Shadow effect
- EXPECTED OUTCOME: ShadowConfig struct with offset_x, offset_y, blur_radius, color; ShadowEffect with apply() method; integrates with EffectsConfig
- MUST DO:
- Follow patterns from cursor.rs and zoom.rs; use Color from effects/mod.rs; add to EffectsConfig struct; performance <2ms per frame
- MUST NOT DO:
- Don't modify pipeline.rs yet (that's integration task); keep effect self-contained
- ACCEPTANCE CHECKS:
- cargo build -p frame-core
- cargo test -p frame-core
- cargo clippy -p frame-core -- -D warnings
- CONTEXT:
Create packages/core/src/effects/shadow.rs, add mod shadow to effects/mod.rs, add shadow: ShadowConfig to EffectsConfig
