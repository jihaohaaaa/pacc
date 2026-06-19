---
name: release-upgrade
description: Use this skill when releasing a new pacc version, including version bump, quality checks, git push, git tag, and cargo publish. Trigger when requests mention release, publish, version bump, tag, or crates.io rollout.
---

# Release Upgrade

## Overview

This skill standardizes the release workflow for the `pacc` Rust project.
Use it to ship a new crate version with repeatable checks and minimal release mistakes.

## Workflow

1. Gather release context.
- Read `Cargo.toml`, `README.md`, and current git status.
- Confirm current branch and whether the working tree is clean.
- If there are unrelated dirty files, stop and ask before touching them.

2. Bump version.
- Update package version in `Cargo.toml`.
- Update root package version in `Cargo.lock`.
- If README contains version-sensitive release or install guidance, keep it aligned.

3. Check dependency updates.
- Run `cargo upgrade --dry-run` to inspect available direct dependency upgrades.
- If dependency upgrades should be included in the release, run `cargo upgrade` and review the `Cargo.toml` diff.
- Run `cargo update` to refresh `Cargo.lock`, then review the `Cargo.lock` diff.
- Include any accepted dependency changes in the release commit and rerun quality gates after the dependency check.

4. Run quality gates.
- Run `cargo fmt`.
- Run `cargo clippy --all-targets -- -D warnings`.
- Run `cargo check`.
- Run `cargo test`.

5. Validate release artifacts.
- Ensure `README.md` examples still match the current binary behavior and key bindings.
- Ensure the TUI still launches and the primary cache-management workflow remains intact.

6. Commit and push.
- Stage only release-related files.
- Commit with message `chore(release): bump version to X.Y.Z`.
- Push to the active remote branch.

7. Publish.
- Run `cargo publish` from project root.
- If publish fails because version already exists, bump patch version and repeat from step 2.

8. Tag release.
- Create annotated tag `vX.Y.Z` on the release commit.
- Push tag to remote with `git push origin vX.Y.Z`.
- If tag already exists and points to another commit, stop and ask before rewriting.

## Command Reference

For exact command sequences and failure handling, read [release-checklist.md](references/release-checklist.md).

## Output Contract

When finishing a release task, report only these items.
- New version.
- Commit hash.
- Push status.
- Tag status.
- Publish status.
- Any follow-up action still required.
