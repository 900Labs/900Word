# Sprint 028: Accessibility And Low-Resource Mode MVP

## Status

Complete.

## Goal

Add lightweight local accessibility and low-resource controls that make 900Word easier to use on older machines and for users with accessibility needs, without adding cloud, accounts, telemetry, network behavior, heavy dependencies, file-format changes, document metadata, import changes, or export changes.

## Shipped

- Added disabled-by-default local settings for larger toolbar controls, reduced motion, and low-resource mode.
- Surfaced the settings as native checkboxes in Settings with stable labels.
- Applied the modes through desktop UI classes and data attributes.
- Increased toolbar click targets in larger-controls mode while preserving toolbar wrapping and compact layout behavior.
- Disabled app-controlled transitions and animations in reduced-motion mode.
- Made low-resource mode suppress nonessential automatic sidebar content and reduce decorative visual effects where practical.
- Preserved safety-critical recovery and warning surfaces in low-resource mode.
- Preserved explicit user-opened review, comments, notes, find, link, export, and stats surfaces.
- Kept high contrast independent from the new modes.
- Kept settings local to the desktop UI and out of documents, exports, document metadata, logs, telemetry, network calls, cloud sync, and accounts.

## Compatibility Boundary

- Low-resource mode is a UI simplification, not a measured performance guarantee.
- Larger toolbar mode changes desktop UI sizing only. It does not change document layout, page metrics, ODT content, or export output.
- Reduced motion covers app-controlled CSS transitions and animations. It does not control operating-system or WebView-level behavior outside the app shell.
- Low-resource mode suppresses automatic navigator and recent-document sidebar content. Recovery, warnings, and panels the user opens remain available.
- No new dependency, document schema, import, export, metadata, account, telemetry, network, or cloud behavior is introduced.

## Deferred

- Per-control density presets beyond the larger toolbar mode.
- Full browser-level accessibility automation for every mode combination.
- Measured low-end hardware benchmarks and hard resource thresholds.

## Verification

- `npm --workspace apps/desktop run test -- i18n` - required for this sprint.
- `npm --workspace apps/desktop run check` - required for this sprint.
- `npm --workspace apps/desktop run lint` - required for this sprint.
- `cargo test -p nine-hundred-word settings_never_enable_telemetry` - required because the Rust settings type changed.
- `git diff --check` - required for this sprint.
- Touched-file privacy scan for local paths, hostnames, private filenames, usernames, recovery locations, document text, and source image filenames - required for this sprint.

## Privacy Notes

- The settings are local desktop UI preferences only.
- The feature does not render local paths, private filenames, usernames, hostnames, recovery locations, document text, or source image filenames in new UI labels, docs, or tests.
- The feature does not write settings into saved documents, exports, document metadata, logs, telemetry, cloud sync, accounts, network calls, or external services.
