# Sprint 073: Export And Print Polish

## Scope

Polish the first-release export and print UX in the desktop File menu while keeping the existing export backends, generated formats, native save/open workflows, telemetry behavior, network behavior, cloud sync, accounts, plugin runtime scope, backend validation, package fixtures, and Rust commands unchanged.

## Deliverables

- Reworked the File > Export panel so TXT, sanitized offline HTML, lightweight PDF snapshot, and bounded DOCX conversion options are easier to scan before opening a native save dialog.
- Added a focused frontend export workflow helper for PDF page-range options and covered it with unit tests.
- Preserved existing PDF range behavior: all-pages export sends empty options, and explicit ranges require positive integer start/end pages with end greater than or equal to start.
- Clarified the File menu print action as sanitized WebView/browser print preparation without deterministic layout or print-pagination claims.
- Polished export option buttons and PDF range controls with compact CSS-only styling, keyboard-friendly native form controls, and mobile single-column fallbacks.
- Updated public release, file-format, roadmap, and changelog documentation with conservative export and print limits.

## Validation

- `npm run check`
- `npm run lint`
- `npm run test`
- `git diff --check`
- `./scripts/verify-public-release.sh`

## Decisions

- Export buttons use operational format descriptions instead of marketing copy.
- PDF page-range validation lives in a small frontend helper because the same contract is needed for UI state and export invocation.
- Print wording names sanitized WebView/browser print preparation and avoids layout-fidelity language.
- Export status and range validation messages stay generic and do not expose paths, filenames, local directories, usernames, hostnames, or document text.
- Docs keep ODT as canonical, DOCX as conversion-only, and TXT/HTML/PDF/print as current bounded export paths.

## Follow-ups

- Smoke-test the File menu export and print flows in the desktop app before the release candidate.
- Keep broader PDF layout fidelity, deterministic print pagination, richer DOCX conversion, external-suite compatibility claims, telemetry, network behavior, cloud sync, accounts, and plugin runtime work deferred to separate scoped sprints.
