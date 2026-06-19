# Release Checklist

## Fast Path

Run these commands in order from repo root.

```bash
git status
git rev-parse --abbrev-ref HEAD
rg -n '^version\s*=\s*"' Cargo.toml
```

If the working tree is dirty and the user asked to release, treat that request as approval to use release-related Git cleanup steps, including stash-based workflows.

Typical cleanup sequence for unrelated local changes:

```bash
git stash push -u -m "release-prep"
# run release workflow
git stash pop
```

If the stash restore conflicts, stop and report the conflict set clearly.

Update version fields.

```bash
# edit Cargo.toml and Cargo.lock
```

Check dependency updates.

```bash
cargo upgrade --dry-run
# If accepted dependency upgrades should be included:
cargo upgrade
cargo update
git diff -- Cargo.toml Cargo.lock
```

If `cargo upgrade` is unavailable, install `cargo-edit` first or stop and report that the dependency upgrade check could not be completed.
If `cargo update` changes `Cargo.lock`, include the lockfile change in the release commit and rerun the quality checks below.

Run quality checks.

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo check
cargo test
```

Commit and push.

```bash
git add Cargo.toml Cargo.lock README.md
# adjust staged files if README was not touched
git commit -m "chore(release): bump version to X.Y.Z"
git push origin <branch>
```

If the local branch is behind remote, use normal sync operations before pushing, for example:

```bash
git fetch origin
git rebase origin/<branch>
```

If sync requires a more invasive history rewrite than a normal release flow, stop and report exactly what blocks the push.

Publish.

```bash
cargo publish
```

Create and push release tag.

```bash
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin vX.Y.Z
```

## Common Failures

- `crate ... already exists`.
  - Bump patch version and rerun publish flow.

- `working directory contains changes not yet committed`.
  - Commit release files first, then publish without `--allow-dirty`.
  - If unrelated local changes are present, stash them as part of release prep and restore them after publish/tag steps complete.

- `clippy` warnings.
  - Fix warnings before publish. Do not bypass with allow flags.

- `tag ... already exists`.
  - Verify whether remote tag points to expected release commit.
  - If tag points elsewhere, stop and ask before any force update.

## Targeted Checks For pacc

- Verify `cargo run` still launches the TUI after the version bump.
- Verify cache search, delete confirmation, and selection shortcuts still match README.
- Verify crates.io metadata in `Cargo.toml` still points to the correct repository and docs URLs.
