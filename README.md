# pacc

`pacc` is an Arch Linux package management assistant with a terminal user interface.
It aims to put `pacman`, `paru`, and AUR-oriented workflows into one focused TUI so
common maintenance tasks feel fast, visible, and safe.

This project is early but already usable for local package-maintenance review:
it can inspect `paru` clone cache entries, move selected cache directories to
trash, audit orphan packages, and remove selected orphans through an explicit
confirmation flow.

## Goals

- Show the system package state from one screen
- Surface upgrades from `pacman` and `paru`
- Review AUR updates without jumping between tools
- Help manage orphan packages and package cache cleanup
- Keep dangerous operations explicit and reviewable

## Current Status

The current build includes:

- A Rust TUI built with `ratatui` and `crossterm`
- Workbench-style navigation with overview, cache, orphan, and action panes
- Backend detection for `pacman` and `paru`
- Keyword search over local `paru` cache and clone metadata
- Cache entry inspection for PKGBUILD, git metadata, and archived package files
- Single and multi-select cache cleanup using `gio trash`
- Orphan package audit using `pacman -Qdtq`
- Single and multi-select orphan removal using `sudo -n pacman -Rns`

## Planned Features

- Real package inventory from `pacman -Q`
- Upgrade views for official repos and AUR packages
- Search, filtering, and per-package inspection
- Action confirmations for sync, upgrade, and broader package flows
- Background command execution and better error reporting

## Usage

```bash
cargo run
```

Default keys:

- `Tab`: switch focus
- `/`: search the active package workspace
- `Space`: toggle selection for the highlighted cache or orphan entry
- `d`: open delete confirmation for the selected cache or orphan entry
- `Up` / `Down`: move selection
- `Enter`: inspect the selected package entry or trigger the selected action stub
- `r`: refresh backend detection, cache index, and orphan audit
- `q` or `Esc`: quit

Cache deletion is intentionally limited to top-level directories below the
detected `paru` clone directory and moves them to trash rather than deleting
them permanently. Orphan removal is a real system package operation and uses
`sudo -n pacman -Rns -- <targets>`, so it requires non-interactive sudo access.

## Development

```bash
cargo fmt
cargo check
```

## Publishing

This crate is published on crates.io as [`pacc`](https://crates.io/crates/pacc).

## License

Licensed under either of the following, at your option:

- Apache License, Version 2.0
- MIT License

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT).
