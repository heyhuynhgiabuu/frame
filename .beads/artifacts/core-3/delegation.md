
---
Generated: 2026-02-05T14:17:01.380Z
---

# Delegation Packet

- TASK: core-3 - Aspect ratio calculator
- EXPECTED OUTCOME: AspectRatio enum with 16:9, 9:16, 1:1, 4:3, Custom variants; dimensions() method returns (width, height); LetterboxInfo struct for letterbox/pillarbox coordinates
- MUST DO:
- Follow existing Color
- Padding patterns from effects/mod.rs; add unit tests; use serde for serialization
- MUST NOT DO:
- Don't add new dependencies; don't modify other modules yet
- ACCEPTANCE CHECKS:
- cargo build -p frame-core
- cargo test -p frame-core
- cargo clippy -p frame-core -- -D warnings
- CONTEXT:
Create packages/core/src/effects/aspect_ratio.rs, add mod aspect_ratio to effects/mod.rs
