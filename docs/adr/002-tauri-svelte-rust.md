# ADR 002: Tauri v2, Svelte 5, And Rust

## Status

Accepted

## Decision

Use Tauri v2 for the desktop shell, Svelte 5 for UI, ProseMirror for the editor surface, and Rust for document logic.

## Context

Tauri uses system WebViews and keeps the backend in Rust. This supports smaller packages than browser-bundling desktop frameworks while preserving a productive UI stack.

## Consequences

- System WebView differences must be tested.
- Rust owns security-sensitive file handling.
- The frontend is not trusted with raw imported content.
