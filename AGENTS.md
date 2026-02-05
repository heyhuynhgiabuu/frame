# Frame

Open-core screen recorder for developers. Rust (iced.rs) desktop app + SolidJS web.

## Tech Stack

- **Desktop:** Rust 1.75+ with iced 0.12 (macOS native)
- **Web:** SolidJS + Tailwind (planned, `apps/web/` stub)
- **JS Tooling:** Bun 1.0+, Biome 1.5.3
- **Build:** Cargo + Justfile

## Structure

```
apps/
├── desktop/        # iced.rs app (frame-desktop)
└── web/            # SolidJS viewer (stub)
packages/
├── core/           # frame-core: capture, encoding, auto-save
├── ui-components/  # frame-ui: reusable iced widgets
└── renderer/       # frame-renderer: GPU rendering (stub)
tests/              # Integration tests
```

## Commands

**Dev:** `just dev` or `cd apps/desktop && cargo run`
**Build:** `cargo build --release`
**Test:** `cargo test --workspace`
**Lint (Rust):** `cargo clippy --workspace -- -D warnings`
**Lint (JS):** `bun run lint`
**Format:** `cargo fmt --all && bun run format`

Single test: `cargo test -p frame-core test_name`

## Code Style (Rust)

```rust
// Error handling: use FrameError + FrameResult
use crate::error::{FrameError, FrameResult};

pub fn do_work() -> FrameResult<()> {
    something().map_err(|e| FrameError::Io {
        source: e,
        context: ErrorContext::Project { name: "untitled".into() },
    })?;
    Ok(())
}
```

## Key Modules

| Module                                        | Purpose                                        |
| --------------------------------------------- | ---------------------------------------------- |
| `packages/core/src/capture/`                  | Screen/audio capture (macOS: ScreenCaptureKit) |
| `packages/core/src/capture/webcam.rs`         | Webcam capture (nokhwa)                        |
| `packages/core/src/encoder.rs`                | Video encoding (ffmpeg-sidecar)                |
| `packages/core/src/encoder/gif.rs`            | GIF encoding (gifski)                          |
| `packages/core/src/auto_save.rs`              | Auto-save & crash recovery                     |
| `packages/core/src/error.rs`                  | Typed errors with recovery actions             |
| `packages/core/src/effects/aspect_ratio.rs`   | Aspect ratio calculations                      |
| `packages/core/src/effects/shadow.rs`         | Shadow effect                                  |
| `packages/core/src/effects/inset.rs`          | Inset/depth effect                             |
| `packages/core/src/effects/webcam_overlay.rs` | Webcam overlay compositing                     |
| `packages/core/src/export_preset.rs`          | Export preset system                           |

## Boundaries

✅ **Always:** Run `cargo clippy` before commit, handle errors with `FrameResult`
⚠️ **Ask first:** New workspace deps, feature flags, platform-specific code
🚫 **Never:** `unwrap()` in production code, commit `target/`, skip error context

## Gotchas

- macOS only currently (ScreenCaptureKit requires 13.0+)
- `biome.json` is YAML format - must convert to JSON for Biome 1.x
- Tests have clippy warnings: `unnecessary_unwrap`, `ptr_arg` in core
