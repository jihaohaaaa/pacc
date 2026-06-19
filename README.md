# pacc

`pacc` is an Arch Linux package management assistant with a terminal user interface.
It aims to put `pacman`, `paru`, and AUR-oriented workflows into one focused TUI so
common maintenance tasks feel fast, visible, and safe.

This project is currently in an early scaffold stage. The UI shell is in place, and
the next steps are wiring real package data, package actions, and safer command
execution flows.

## Goals

- Show the system package state from one screen
- Surface upgrades from `pacman` and `paru`
- Review AUR updates without jumping between tools
- Help manage orphan packages and package cache cleanup
- Keep dangerous operations explicit and reviewable

## Current Status

The current build includes:

- A Rust TUI built with `ratatui` and `crossterm`
- Basic app state, focus management, and keyboard navigation
- Backend detection for `pacman` and `paru`
- Placeholder package and action panels for future integration

## Planned Features

- Real package inventory from `pacman -Q`
- Upgrade views for official repos and AUR packages
- Search, filtering, and per-package inspection
- Action confirmations for sync, upgrade, remove, clean, and orphan flows
- Background command execution and better error reporting

## Usage

```bash
cargo run
```

Default keys:

- `Tab`: switch focus
- `j` / `k`: move selection
- `Enter`: trigger the selected action stub
- `r`: refresh backend detection
- `q` or `Esc`: quit

## Development

```bash
cargo fmt
cargo check
```

## Publishing

The intended crates.io package name is [`pacc`](https://crates.io/crates/pacc).

## License

Licensed under either of the following, at your option:

- Apache License, Version 2.0
- MIT License

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT).
