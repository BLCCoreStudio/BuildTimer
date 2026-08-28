# BuildTimer

**Measure command durations and compare them with previous local runs.**

BuildTimer is a small, Linux-first Rust CLI for measuring how long build, test, and other terminal commands take and comparing the result with the previous local run of the same command.

It is local-only: no telemetry, cloud service, account, server, or network backend is involved.

## Status

**BuildTimer v0.1.0 is available as the first public release.**

A prebuilt Linux x86_64 binary and SHA-256 checksum are available on the [GitHub Releases page](https://github.com/BLCCoreStudio/BuildTimer/releases/tag/v0.1.0).

## What it does

```bash
buildtimer -- cargo build --release
```

BuildTimer starts the wrapped executable directly, waits for it to finish, prints the elapsed time, stores a local history entry, and compares the result with the previous matching command.

Example output:

```text
BuildTimer: 12.438 s
Previous: 13.102 s -> 12.438 s (664 ms faster, 5.1%)
```

The wrapped process exit code is returned by BuildTimer unchanged. If the process is terminated by a Unix signal, BuildTimer uses the conventional `128 + signal` process exit status.

## Install on Linux x86_64

Download these files from the [v0.1.0 release](https://github.com/BLCCoreStudio/BuildTimer/releases/tag/v0.1.0):

- `buildtimer-linux-x86_64`
- `buildtimer-linux-x86_64.sha256`

Verify and run:

```bash
sha256sum -c buildtimer-linux-x86_64.sha256
chmod +x buildtimer-linux-x86_64
./buildtimer-linux-x86_64 --version
```

You can optionally rename the binary to `buildtimer` and place it somewhere on your `PATH`, such as `~/.local/bin`.

## Commands

Run and time a command:

```bash
buildtimer -- cargo test
buildtimer -- cargo build --release
buildtimer -- make -j8
```

Show recent history:

```bash
buildtimer history
```

Clear local history:

```bash
buildtimer clear
```

## Direct process execution

BuildTimer does **not** execute wrapped commands through `sh -c`, `bash -c`, or another shell. The executable and argument vector are passed directly through Rust's `std::process::Command` API.

That means shell interpolation is intentionally not performed by BuildTimer. For example, `$TOKEN`, pipes, redirections, globs, and command substitution are not expanded unless you explicitly choose a shell as the wrapped executable yourself.

## Local history and privacy

History is stored locally at:

```text
$XDG_DATA_HOME/buildtimer/history.json
```

or, when `XDG_DATA_HOME` is unset:

```text
~/.local/share/buildtimer/history.json
```

The file contains only BuildTimer history metadata such as the sanitized command display, duration, and Unix timestamp. BuildTimer never enumerates or stores the process environment.

Before persistence, BuildTimer redacts common secret-bearing arguments such as:

```text
TOKEN=value          -> TOKEN=<redacted>
--password value     -> --password <redacted>
--api-key=value      -> --api-key=<redacted>
https://user:pass@... -> https://<redacted>@...
```

Environment-style `NAME=value` arguments are always persisted with the value redacted. History is capped at the most recent 500 entries. On Unix, the BuildTimer data directory is set to mode `0700` and the history file to mode `0600`.

## Command start failures

If a wrapped executable cannot be started, BuildTimer reports the operating-system error and does not write a history entry.

Common Unix-compatible return codes are used:

- `127` when the executable cannot be found
- `126` when permission is denied
- `1` for other process-start errors

## Build from source

Requirements:

- Rust 1.80 or newer
- Linux is the primary supported platform for v0.1.0

```bash
git clone https://github.com/BLCCoreStudio/BuildTimer.git
cd BuildTimer
cargo build --release --locked
./target/release/buildtimer --version
```

For local installation during development:

```bash
cargo install --path . --locked
```

## Development checks

The CI workflow runs the same required checks on pull requests and on `main`:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
```

See [CONTRIBUTING.md](CONTRIBUTING.md) before sending changes and [SECURITY.md](SECURITY.md) for vulnerability reporting guidance.

## License

BuildTimer is licensed under the [MIT License](LICENSE).

Built by **BLC Core Studio**.