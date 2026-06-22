# Contributing to 900Word

900Word is an open-source desktop word processor for offline-first, low-resource environments. Contributions are welcome when they preserve the project goals: local ownership, privacy, accessibility, careful file-format handling, and clear documentation.

## Setup

```bash
git clone https://github.com/YOUR_GITHUB_USERNAME/900Word.git
cd 900Word
npm install
./scripts/verify-local.sh
```

Install Tauri system dependencies for your operating system from the official guide: <https://v2.tauri.app/start/prerequisites/>.

## Development Rules

- Keep `word-core` as the durable document model source of truth.
- Treat imported documents and assets as untrusted input.
- Do not add telemetry, network calls, external converters, plugins, or sync behavior without an accepted ADR.
- Keep runtime plugins, cloud sync, binary `.doc`, deterministic pagination, and full encryption deferred until their designs are documented and tested.
- Add documentation updates with code changes that affect behavior, public APIs, workflows, validation, security, privacy, or contributor expectations.
- Use generated fixtures only. Do not commit real personal documents.

## Validation

Run before opening a pull request:

```bash
./scripts/verify-local.sh
```

For public-release or visibility changes, also run:

```bash
./scripts/verify-public-release.sh
```

## Pull Requests

Every pull request must include:

- Summary of changes.
- Problem statement.
- Scope in and out.
- Validation commands and results.
- Documentation impact.
- Link to a sprint record in `docs/sprints/`.

Use squash merge for `main`.
