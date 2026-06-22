# ADR 009: Encryption Deferred

## Status

Accepted

## Decision

Full document encryption is deferred.

## Context

Encryption claims require key derivation, key storage, autosave, recovery, temp-file, metadata, and failure-mode design.

## Consequences

- Do not market encryption as shipped.
- Recovery and temp-file privacy must be documented before encryption work starts.
