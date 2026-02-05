# Frame Setup Guide

Complete guide for setting up Frame development environment.

---

## Prerequisites

### Required

- **macOS 12.3+** (Monterey) - ScreenCaptureKit requires macOS 12.3 or later
- **Rust 1.75+** - Install via [rustup](https://rustup.rs/)
- **Bun 1.0+** - Install via [bun.sh](https://bun.sh/)

### Optional (for system audio)

- **BlackHole 0.5.0+** - Virtual audio driver for system audio capture

---

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/frame/frame.git
cd frame
```

### 2. Install Rust Dependencies

```bash
# Fetch all workspace dependencies
cargo fetch
```

### 3. Install JavaScript Dependencies

```bash
bun install
```

### 4. Build the Project

```bash
# Build all packages
cargo build --release

# Or use just
just build
```

### 5. Run the Desktop App

```bash
# Run in development mode
cd apps/desktop && cargo run

# Or use just
just dev
```

---

## Detailed Setup

### Rust Installation

If you don't have Rust installed:

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add to PATH
source $HOME/.cargo/env

# Verify installation
rustc --version  # Should be 1.75 or higher
cargo --version
```

### Bun Installation

If you don't have Bun installed:

```bash
# Install Bun
curl -fsSL https://bun.sh/install | bash

# Add to PATH (follow instructions in output)
# Verify installation
bun --version  # Should be 1.0 or higher
```

### ffmpeg Installation

ffmpeg is required for video encoding:

```bash
# Using Homebrew (recommended)
brew install ffmpeg

# Verify installation
ffmpeg -version
```

### BlackHole Installation (Optional)

BlackHole is required for capturing system audio (e.g., video audio, game sounds):

#### Option 1: Homebrew (Recommended)

```bash
brew install blackhole-2ch
```

#### Option 2: Manual Download

1. Download from [existential.audio/blackhole](https://existential.audio/blackhole/)
2. Open the `.pkg` file and follow installation instructions
3. Restart your Mac

#### Configure BlackHole

1. Open **Audio MIDI Setup** (search in Spotlight)
2. Click the **+** button and select **Create Multi-Output Device**
3. Check both **BlackHole 2ch** and your speakers/headphones
4. Right-click the multi-output device and select **Use This Device for Sound Output**
5. In Frame app settings, select **BlackHole 2ch** as system audio input

---

## Development Workflow

### Using Just (Task Runner)

We use `just` as our task runner. Install it:

```bash
brew install just
```

Available commands:

```bash
just dev          # Run desktop app in development mode
just build        # Build release version
just test         # Run all tests
just lint         # Run linters
just format       # Format code
just clean        # Clean build artifacts
```

### Using Cargo

```bash
# Build
cargo build --release

# Test
cargo test --workspace

# Format
cargo fmt --all

# Lint
cargo clippy --workspace -- -D warnings

# Documentation
cargo doc --workspace --open
```

### Using Bun

```bash
# Install dependencies
bun install

# Run linting
bun run lint

# Fix linting issues
bun run lint:fix

# Format code
bun run format
```

---

## IDE Setup

### VS Code

Recommended extensions:

- **rust-analyzer** - Rust language support
- **Biome** - JavaScript/TypeScript linting and formatting
- **Even Better TOML** - TOML file support
- **CodeLLDB** - Debugging support

Settings:

```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "biomejs.biome"
}
```

### RustRover / IntelliJ

1. Install the **Rust** plugin
2. Import the project as a Cargo project
3. Enable clippy inspections in settings

---

## Troubleshooting

### Build Errors

#### "ffmpeg not found"

```bash
brew install ffmpeg
```

#### "ScreenCaptureKit not available"

- Ensure you're on macOS 12.3 or later
- Check: `sw_vers -productVersion`

#### "Permission denied" when recording

1. Open **System Preferences** → **Security & Privacy** → **Privacy**
2. Select **Screen Recording**
3. Add your terminal/IDE and Frame app
4. Restart the app

### Runtime Issues

#### "No audio in recording"

1. Check microphone permissions in System Preferences
2. If using system audio, ensure BlackHole is installed and configured
3. Check audio levels in Frame settings

#### "Recording stops unexpectedly"

- Check available disk space (need at least 1GB free)
- Check Console.app for crash logs
- Try reducing recording resolution or frame rate

### Development Issues

#### "cargo check is slow"

```bash
# Enable sccache for faster builds
brew install sccache
cargo install cargo-cache
cargo cache -a  # Clean cache
```

#### "Hot reload not working"

```bash
# Install cargo-watch
cargo install cargo-watch

# Use just watch
just watch
```

---

## Project Structure

```
frame/
├── apps/
│   └── desktop/          # Main desktop application
├── packages/
│   ├── core/            # Core recording library
│   ├── ui-components/   # Reusable UI components
│   └── renderer/        # GPU rendering (future)
├── docs/                # Documentation
├── tooling/             # Build tools and configs
├── Cargo.toml          # Rust workspace
├── package.json        # Bun workspace
├── biome.json          # Biome configuration
└── Justfile            # Task runner
```

---

## Next Steps

1. Read the [API Documentation](API.md)
2. Check out [Contributing Guide](CONTRIBUTING.md)
3. Join our Discord community (coming soon)
4. Star the project on GitHub ⭐

---

## Getting Help

- **GitHub Issues**: [github.com/frame/frame/issues](https://github.com/frame/frame/issues)
- **Discussions**: [github.com/frame/frame/discussions](https://github.com/frame/frame/discussions)
- **Discord**: Coming soon

---

## License

Frame is dual-licensed under MIT and Apache-2.0. See LICENSE files for details.
