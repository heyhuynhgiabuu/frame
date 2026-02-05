# Frame

An open-core screen recorder built for developers. Beautiful by default, extensible by design.

## Architecture

- **Desktop App**: Rust + iced.rs for native performance
- **Web Components**: SolidJS + Tailwind + Kobalte for web UI
- **Backend**: Hybrid cloud (Supabase + Cloudflare)

## Quick Start

```bash
# Install dependencies
bun install

# Run desktop app
cd apps/desktop && cargo run

# Run web app
cd apps/web && bun dev
```

## Project Structure

```
frame/
├── apps/
│   ├── desktop/          # Main iced.rs application
│   └── web/              # Web viewer/sharing
├── packages/
│   ├── core/             # Shared Rust library
│   ├── ui-components/    # Reusable iced.rs components
│   └── renderer/         # GPU-accelerated rendering
├── plugins/              # Plugin system
└── tooling/              # Build & config
```

## License

MIT/Apache-2.0 for core, commercial license for Pro features
