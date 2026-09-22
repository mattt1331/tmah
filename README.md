# tmah

Like TheatreMix, but made at home. TODO put something here

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

`src/`: Code

`index.html`: The base HTML file that trunk glues the compiled WASM into for web.

`assets/sw.js`: Something to do with it working as a PWA (web).

`flake.nix, flake.lock`: Provides devshell and someday will do builds. Ignore if not using Nix package manager.

`README.md`: pretentious ramblings

### Approximate Architecture

The code is structured more or less like this:

`main.rs`: Starts eframe's event loop

`crate::app`: Contains the main state struct (`State`) and a few methods

`crate::app::ui`: Contains most of the ui code

`crate::app::cues`: Contains state-modifying methods used by the ui of the cues screen (idea is to decouple logic from ui)

`crate::app::file`: Methods and code related to saving/loading files, state-modifying methods used by file ui

`crate::app::board`: The big one. Contains all the code for interacting with the board. The rest of the program interfaces with `board` through the enum `Connections` which is a list of all the boards we can connect to and the trait `Connectable` which defines a common interface for each board.
