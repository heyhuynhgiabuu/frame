
---
Generated: 2026-02-05T14:17:11.888Z
---

# Delegation Packet

- TASK: core-9 - Inset effect
- EXPECTED OUTCOME: InsetConfig struct with intensity, style (Light/Dark); InsetEffect with apply() method; works with rounded corners
- MUST DO:
- Follow patterns from shadow.rs; add to EffectsConfig struct; coordinate with shadow for edge effects
- MUST NOT DO:
- Don't modify pipeline.rs yet; keep effect self-contained
- ACCEPTANCE CHECKS:
- cargo build -p frame-core
- cargo test -p frame-core
- cargo clippy -p frame-core -- -D warnings
- CONTEXT:
Create packages/core/src/effects/inset.rs, add mod inset to effects/mod.rs, add inset: InsetConfig to EffectsConfig
