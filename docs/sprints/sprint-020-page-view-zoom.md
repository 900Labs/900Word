# Sprint 020: Page View And Zoom

## Status

Complete.

## Goal

Add lightweight editor view and zoom controls from the upgrade brief while keeping page layout behavior clearly scoped to the editor viewport.

## Shipped

- Added Draft and Page Layout view mode controls to the desktop editing toolbar.
- Kept Page Layout on the existing paper-like surface and added a simpler Draft surface that uses the available editor width with less page chrome.
- Added Fit Width, 100%, and bounded custom zoom controls backed by local Svelte component state. Fit Width derives a local viewport zoom from the editor pane width without persisting it to the document.
- Added targeted viewport helper tests for zoom clamping, fit-width calculation, generated CSS viewport variables, and invalid page number normalization.
- Added a show/hide rulers toggle with simple top and left visual guides derived from the current page setup and margins.
- Kept the bottom toolbar visible and allowed the editor surface to scroll on smaller screens instead of hiding document controls.

## Deferred

- Deterministic pagination.
- Page-break preview.
- Font metrics or layout engine work.
- Print layout fidelity claims beyond the existing basic print/export behavior.
- Persisted user view preferences or any new file-format metadata.

## Verification

- `npm run check`
- `npm run lint`
- `npm run test`
- `npm run build`
- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `git diff --check`
- `./scripts/verify-public-release.sh`

## Privacy Notes

- View mode, zoom, and ruler state remain local to the running desktop component.
- No telemetry, network access, cloud accounts, stored document metadata, local paths, personal names, or remote assets are added.
