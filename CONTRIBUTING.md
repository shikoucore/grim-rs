# Contributing Guide

Thanks for your interest in contributing to grim-rs! This document describes
how we work and what is required for contributions to be accepted.

## Project Scope
- grim-rs targets Wayland compositors that support `zwlr_screencopy_manager_v1`.
- The library is Rust-first and aims for a clean public API with minimal
  dependencies and predictable behavior.

## Quick Start
- Fork the repo and create a feature branch.
- Make changes with clear commit messages.
- Add or update tests.
- Open a pull request.

## Non-Negotiable Rules
- Tests must live in `tests/` and be placed in a file that matches what you are testing.
- Any new public API must be justified (why it is needed and why it belongs in the public surface).
- Any bug fix must be tied to an Issue and include a link to it.
- Every pull request must reference the related Issue.
- For new functionality not currently present, create an Issue first and discuss before implementation.

## Issue First Policy
Before starting any work:
- Open an Issue for new features.
- Open an Issue for bug fixes, including reproduction steps and environment.
- Link the Issue in your PR description.

## Public API Policy
Changes to the public API require:
- A short justification in the PR.
- A clear explanation of alternatives considered.
- Updates to docs and examples if behavior changes.
- A migration note if the change is breaking.

## Testing Policy
- All tests must be under `tests/`.
- Place tests in the file that matches the area you are testing.
- Keep tests deterministic; avoid environment-specific assumptions.
- Run `cargo test` locally before opening a PR.

## Documentation and Changelog
- Update `README.md` when user-facing behavior changes.
- Update `CHANGELOG.md` for any notable change.
  Example:
  ```
  ### Fixed
  - **Short summary**: Clear description of what changed and why. Close #123, [@username](link).
  ```
- For breaking changes, add or update `MIGRATION.md`.

## Code Style and Quality
- Follow the existing code style and structure.
- Avoid new `unsafe` unless absolutely required; explain why in the PR.
- Prefer explicit error handling over `unwrap()` or `expect()` in production code.
- Keep functions focused and readable.

## Pull Request Checklist
- [ ] Linked Issue in the PR description.
- [ ] Tests added or updated in `tests/`.
- [ ] `cargo test` passes locally.
- [ ] Docs and changelog updated where needed.
- [ ] Public API changes justified (if applicable).

## Communication
- Keep PRs focused and small when possible.
- Be explicit about tradeoffs and alternatives.
- If in doubt, open an Issue and ask before implementing.

## Thanks
Thank you for contributing to open source and supporting grim-rs!
