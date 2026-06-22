# ADR 006: PDF Export Strategy

## Status

Accepted

## Decision

The bootstrap provides basic PDF export for smoke testing. Deterministic pagination and full layout fidelity are deferred until a layout-engine ADR is accepted.

## Consequences

- README must not claim full desktop-publishing PDF fidelity.
- PDF tests begin with metadata and text-presence checks.
