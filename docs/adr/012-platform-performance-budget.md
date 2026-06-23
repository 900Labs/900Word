# ADR 012: Platform And Performance Budget

## Status

Accepted

## Decision

900Word will publish measured performance and compatibility results before making broad low-resource claims.

## Consequences

- The project does not claim AbiWord-class 16 MB memory use.
- Installer size, startup time, idle memory, typing latency, and ODT open/save time become release measurements.
- Sprint 008 enforces initial frontend build-output budgets and performance smoke timing only.
- Platform-specific startup, idle memory, typing latency, package size, and runtime network evidence must be captured before binary releases.
