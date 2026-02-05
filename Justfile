# Frame Task Runner
# Install with: cargo install just

# Default recipe - show available commands
default:
    @just --list

# Development commands
dev:
    cd apps/desktop && cargo run

# Build commands
build:
    cargo build --release

build-desktop:
    cd apps/desktop && cargo build --release

# Testing
test:
    cargo test --workspace

test-core:
    cargo test -p frame-core

# Linting and formatting
lint:
    cargo clippy --workspace -- -D warnings
    cd .. && bun run lint

format:
    cargo fmt --all
    cd .. && bun run format

# Cleanup
clean:
    cargo clean
    rm -rf target/

# Install dependencies
install:
    cargo fetch
    cd .. && bun install

# Release build for distribution
release-macos:
    cargo build --release --target x86_64-apple-darwin
    cargo build --release --target aarch64-apple-darwin
    # TODO: Create universal binary and .app bundle

# Development helpers
watch:
    cargo watch -x "run --package frame-desktop"

# Documentation
docs:
    cargo doc --workspace --open
