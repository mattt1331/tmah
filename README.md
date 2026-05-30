# miq-v2

Like TheatreMix. TODO put something here

## Project Documentation

### Running

If native: `cargo run`
If web: `trunk serve`

### Tools

Build/check not set up yet so remember to:
- `cargo fmt` to format
- `cargo clippy` to lint
- `typos` to embarrass yourself

### Logging

Use `log::error`, `log::warn`, `log::info`, etc.

### What are these files?

`Cargo.toml, Cargo.lock`: Cargo gets dependencies and tells the compiler to do stuff.
`flake.nix, flake.lock`: Provides devshell and someday will do builds. Ignore if not using Nix package manager.
`src/`: Code
`index.html`: The base HTML file that trunk glues the compiled WASM into for web.
`assets/sw.js`: Something to do with it working as a PWA (web).
`README.md`: pretentious ramblings

