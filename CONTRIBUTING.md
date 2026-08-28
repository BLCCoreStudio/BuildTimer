# Contributing to BuildTimer

Thanks for helping improve BuildTimer.

## Scope

BuildTimer is intentionally small, Linux-first, local-only, and privacy-conscious. Changes should preserve these properties unless a future project decision explicitly changes them.

In particular:

- do not add telemetry, analytics, cloud sync, accounts, or a required server;
- do not run wrapped commands through implicit shell interpolation;
- do not persist environment snapshots or secret values;
- preserve the wrapped command's exit status behavior.

## Development setup

Install Rust 1.80 or newer, clone the repository, and create a feature branch from `main`.

```bash
git checkout -b feat/my-change
```

## Required checks

Before opening a pull request, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

All three checks must pass in CI.

## Pull requests

Keep pull requests focused. Include tests for behavior changes and update the README when user-facing behavior changes.

Please avoid unrelated refactors in the same pull request.
