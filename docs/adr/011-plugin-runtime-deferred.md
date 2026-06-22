# ADR 011: Plugin Runtime Deferred

## Status

Accepted

## Decision

Runtime plugins are deferred.

## Consequences

- No plugin loading in the bootstrap.
- Future plugin work requires sandboxing, signing, permissions, GPL compatibility, and threat-model updates.
