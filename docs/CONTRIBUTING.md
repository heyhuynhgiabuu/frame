# Contributing to Frame

Thank you for your interest in contributing to Frame! This guide will help you get started.

---

## Code of Conduct

This project adheres to a code of conduct. By participating, you are expected to uphold this code:

- Be respectful and inclusive
- Welcome newcomers
- Focus on constructive feedback
- Respect different viewpoints and experiences

---

## How to Contribute

### Reporting Bugs

Before creating a bug report:

1. Check if the issue already exists in [GitHub Issues](https://github.com/frame/frame/issues)
2. Update to the latest version to see if it's already fixed
3. Try to isolate the problem with minimal steps to reproduce

When reporting bugs, include:

- **macOS version** (e.g., 14.2.1)
- **Frame version** (e.g., 0.1.0)
- **Hardware** (e.g., MacBook Pro M1, 16GB RAM)
- **Steps to reproduce**
- **Expected behavior**
- **Actual behavior**
- **Screenshots/videos** if applicable
- **Console logs** (from Console.app or terminal)

### Suggesting Features

Feature requests are welcome! Please:

1. Check if the feature is already requested
2. Explain the use case and why it would be valuable
3. Consider if it fits Frame's scope (developer-focused screen recorder)

### Pull Requests

1. **Fork** the repository
2. **Create a branch** from `main` (e.g., `feature/my-feature` or `fix/bug-description`)
3. **Make your changes** following our coding standards
4. **Test your changes** thoroughly
5. **Update documentation** if needed
6. **Submit a pull request** with a clear description

---

## Development Setup

See [SETUP.md](SETUP.md) for detailed setup instructions.

Quick start:

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/frame.git
cd frame

# Install dependencies
cargo fetch
bun install

# Build and run
cargo build --release
cd apps/desktop && cargo run
```

---

## Coding Standards

### Rust

We follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/):

- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting (zero warnings policy)
- Write documentation comments for all public APIs
- Use meaningful variable names
- Prefer composition over inheritance
- Handle errors explicitly (no unwrap in production code)

Example:

````rust
/// Captures the screen using the provided configuration.
///
/// # Arguments
///
/// * `config` - Capture configuration specifying area, cursor, audio settings
///
/// # Returns
///
/// Returns `Ok(())` if capture started successfully, or a `FrameError` if
/// permission was denied or configuration is invalid.
///
/// # Example
///
/// ```
/// let config = CaptureConfig::default();
/// capture.start(config).await?;
/// ```
pub async fn start(&mut self, config: CaptureConfig) -> FrameResult<()> {
    // Implementation
}
````

### JavaScript/TypeScript

We use **Biome** for linting and formatting:

```bash
bun run lint     # Check for issues
bun run lint:fix # Fix auto-fixable issues
bun run format   # Format code
```

Standards:

- Use TypeScript for all new code
- Prefer `const` and `let` over `var`
- Use async/await over raw promises
- Write meaningful function and variable names
- Add JSDoc comments for public functions

### Git Commits

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types:

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, no logic change)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Build process, dependencies, etc.

Examples:

```
feat(capture): add ScreenCaptureKit integration

fix(ui): resolve recording button state issue
docs(api): update encoder documentation
refactor(core): simplify error handling
```

---

## Project Structure

```
frame/
├── apps/
│   └── desktop/          # Main application
│       ├── src/
│       │   ├── main.rs   # Entry point
│       │   ├── app.rs    # Application state
│       │   └── ui/       # UI components
│       └── Cargo.toml
├── packages/
│   ├── core/            # Core library
│   │   ├── src/
│   │   │   ├── capture/ # Screen/audio capture
│   │   │   ├── encoder/ # Video encoding
│   │   │   ├── project/ # Project management
│   │   │   └── error.rs # Error types
│   │   └── Cargo.toml
│   ├── ui-components/   # Reusable UI
│   └── renderer/        # GPU rendering
├── docs/                # Documentation
└── tooling/             # Build tools
```

---

## Testing

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run specific package tests
cargo test -p frame-core

# Run with output
cargo test --workspace -- --nocapture
```

### Writing Tests

#### Unit Tests

Place unit tests in the same file as the code:

```rust
// src/capture/mod.rs

pub fn capture_frame() -> Frame {
    // Implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_frame() {
        let frame = capture_frame();
        assert!(frame.width > 0);
        assert!(frame.height > 0);
    }
}
```

#### Integration Tests

Place integration tests in `tests/` directory:

```rust
// packages/core/tests/recording_test.rs

use frame_core::capture::*;
use frame_core::project::Project;

#[tokio::test]
async fn test_full_recording_flow() {
    let project = Project::new("Test");
    // Test full flow
}
```

### Test Coverage

Aim for:

- **Core library**: 80%+ coverage
- **UI components**: Manual testing + snapshot tests
- **Integration**: Critical user flows

---

## Documentation

### Code Documentation

- All public APIs must have doc comments
- Include examples in doc comments
- Explain "why" not just "what"

### User Documentation

- Update [SETUP.md](SETUP.md) for setup changes
- Update [API.md](API.md) for API changes
- Add to CHANGELOG.md for user-facing changes

---

## Review Process

### Before Submitting

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] Code is formatted (`cargo fmt`, `bun run format`)
- [ ] Linting passes (`cargo clippy`, `bun run lint`)
- [ ] Documentation updated
- [ ] Commit messages follow conventions

### Review Criteria

Pull requests are reviewed for:

1. **Correctness** - Does it work as intended?
2. **Code quality** - Is it maintainable?
3. **Testing** - Is it tested?
4. **Documentation** - Is it documented?
5. **Performance** - Are there performance implications?
6. **Security** - Are there security concerns?

### Response Time

- Initial review: Within 3 days
- Follow-up reviews: Within 1 day
- Simple PRs: Usually merged within a week

---

## Areas for Contribution

### Good First Issues

Look for issues labeled `good first issue` or `help wanted`:

- Documentation improvements
- Small bug fixes
- Adding tests
- Code refactoring

### Feature Areas

We're particularly interested in contributions for:

- **Platform support** - Linux and Windows implementations
- **Performance** - Optimization, hardware acceleration
- **Effects** - Cursor smoothing, zoom, motion blur
- **Export formats** - Additional codecs and formats
- **Accessibility** - Screen reader support, keyboard navigation
- **Localization** - Translations for other languages

---

## Community

### Communication Channels

- **GitHub Issues** - Bug reports and feature requests
- **GitHub Discussions** - General questions and ideas
- **Discord** - Real-time chat (coming soon)

### Recognition

Contributors will be:

- Listed in CONTRIBUTORS.md
- Mentioned in release notes
- Invited to the core team after sustained contributions

---

## License

By contributing to Frame, you agree that your contributions will be licensed under the MIT and Apache-2.0 licenses.

---

## Questions?

If you have questions:

1. Check existing [documentation](https://github.com/frame/frame/tree/main/docs)
2. Search [GitHub Discussions](https://github.com/frame/frame/discussions)
3. Ask in a new discussion thread
4. Join our Discord (coming soon)

Thank you for contributing to Frame! 🎉
