# Frame Task Runner
# Install with: brew install just

# Default recipe - show available commands
default:
    @just --list

# Development - open in Xcode
dev:
    open apps/desktop-swift/Frame.xcodeproj

# Build the app
build:
    xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame build

# Build for release
build-release:
    xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame -configuration Release build

# Run tests
test:
    xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame test

# Lint JS files
lint:
    bun run lint

# Format JS files
format:
    bun run format

# Clean build artifacts
clean:
    xcodebuild -project apps/desktop-swift/Frame.xcodeproj -scheme Frame clean
    rm -rf ~/Library/Developer/Xcode/DerivedData/Frame-*
