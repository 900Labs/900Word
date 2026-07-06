# Sprint 072: Creation Quality Polish

## Scope

Improve first-release document creation quality and visual polish in the desktop Start Center and Settings page setup controls. This sprint stays in the existing Svelte/CSS frontend surface and does not change file formats, the backend document model, template storage, document import/export behavior, telemetry, network behavior, cloud behavior, accounts, or plugin runtime behavior.

## Deliverables

- Polished the Start Center as a usable creation workspace with document actions, recent and recovery access, template browsing, and a compact page setup summary that links to Settings.
- Added deterministic CSS-only template preview metadata derived from template id, name, source, and description.
- Added unit coverage for template preview classification, bounded deterministic metadata, and no raw path-like metadata echoing.
- Reworked Settings page setup controls so A4, A3, A5, Letter, Legal, and Tabloid are visible as clickable page-format presets while custom width and height fields remain available.
- Reworked margin presets into visible controls while custom top, right, bottom, and left numeric fields remain available.
- Updated the changelog and roadmap with public-safe release notes for this sprint.

## Validation

- `npm run check`
- `npm run lint`
- `npm run test`
- `git diff --check`
- `./scripts/verify-public-release.sh`

## Decisions

- Template previews stay CSS/HTML-only and do not fetch, generate, embed, or reference image assets.
- Template preview metadata is a bounded frontend presentation layer and does not inspect backend document contents.
- A4 remains the default page setup behavior through the existing default page setup path.
- Settings keeps numeric custom fields for dimensions and margins instead of hiding advanced page setup behind presets.
- Start Center copy remains operational and avoids marketing or documentation-style explanations.

## Follow-ups

- Smoke-test the polished Start Center in the running desktop app before the release candidate.
- Consider keyboard focus order and screen-reader wording as part of the release accessibility pass.
- Keep broader template browsing, downloadable template packs, cloud templates, and plugin-backed template sources deferred.
