# Security Policy

## Supported versions

BuildTimer is currently pre-release software. Security fixes target the latest code on `main` until the first stable release policy is defined.

## Reporting a vulnerability

Please do not publish exploit details in a public issue before maintainers have had a reasonable chance to investigate and fix the problem.

Use GitHub's private vulnerability reporting feature for this repository when available. If private reporting is unavailable, open a minimal public issue that does not include secrets, exploit payloads, or sensitive reproduction data and ask for a private contact path.

Include:

- affected revision or version;
- operating system and relevant environment details that are safe to share;
- a concise description of the impact;
- reproduction steps that do not expose real credentials or private data.

## Security design notes

BuildTimer is local-only and has no telemetry, cloud backend, account system, or server component.

Wrapped commands are started directly with `std::process::Command`; BuildTimer does not implicitly invoke a shell or interpolate shell syntax.

BuildTimer does not enumerate or persist environment variables. Before writing command history, it redacts environment-style `NAME=value` arguments, common secret-bearing flags, authorization-like inline values, and URL credentials. On Unix, the local data directory and history file are restricted to owner access where supported.

Users should still avoid passing sensitive material in unusual positional command-line arguments because command lines can be observable elsewhere on many operating systems independently of BuildTimer.
