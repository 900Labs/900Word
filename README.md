<div align="center">
  <h1>900Word</h1>
  <p><strong>Offline-first word processing for low-resource environments.</strong></p>
  <p>Free. Local. Open.</p>

  <a href="https://github.com/900Labs/900Word">Repository</a> -
  <a href="#features">Features</a> -
  <a href="#build-from-source">Build From Source</a> -
  <a href="#documentation">Documentation</a> -
  <a href="#contributing">Contributing</a>
</div>

---

900Word is a local-first desktop word processor designed for communities where expensive subscriptions, constant connectivity, and high-end hardware are not realistic assumptions.

The first release track focuses on safe local document editing with OpenDocument Text (`.odt`) as the native format. DOCX support is import/export conversion only. Network sync, real-time collaboration, runtime plugins, legacy binary `.doc`, and full encryption are intentionally deferred until their security and privacy designs are documented and tested.

## Features

Current foundation:

- Tauri v2 desktop shell with a Rust backend and Svelte 5 frontend.
- ProseMirror editing surface projected from a Rust-owned document model.
- Editor, Settings, and About shell views for the desktop workspace.
- Constrained ProseMirror schema for the current `word-core` projection.
- ODT MVP package read/write for paragraphs, headings, inline marks, links, lists, tables, page breaks, metadata, named styles, and allowlisted embedded image bytes.
- Bounded DOCX import/export conversion for paragraphs, Heading 1-3, basic inline marks, safe hyperlinks, simple lists, simple tables, simple page-region headers/footers/page fields, embedded PNG/JPEG/GIF/WebP image assets, simple anchored comments, simple text-only tracked insertions/deletions, and simple footnotes/endnotes, with warnings for degraded imports.
- Local file workflow commands for new/open/save/save-as, autosave, recovery drafts, and privacy-preserving recent document tokens.
- Editing toolbar controls for undo/redo, inline marks, paragraph/heading styles, find/replace, sanitized starter templates, ODT-backed page setup metadata, and keyboard-accessible view traversal.
- Hunspell-shaped word-list spell-check loading with a generated minimal `en-US` bootstrap dictionary, user dictionary folder support, missing-dictionary fallback, and initial UI localization.
- Local TXT, sanitized HTML, lightweight paginated PDF with page-range export, simple table boxes, bounded JPEG image embedding, visible figure placeholders for unsupported image payloads, and bounded safe external URI annotations, plus minimal DOCX export-to-path workflows and a WebView print flow using sanitized print HTML.
- Rust workspace crates for document model, ODT/DOCX handling, spell-check boundaries, export, and sanitized fixtures.
- Generated JSON fixtures with multilingual sample content only.
- No telemetry by default.
- Offline startup smoke tests for the desktop boot path.
- GPL-3.0-or-later licensing.
- Public-release privacy checks for local paths, hostnames, secrets, and generated artifacts.

Planned MVP:

- Create, open, edit, and save `.odt` documents.
- Expanded ODT compatibility fixtures and broader layout fidelity.
- Expanded editing workflows.
- Hunspell affix expansion, broader dictionary compatibility, and more complete dictionary packaging.
- Accessibility, keyboard navigation, and high-contrast refinements.

## Architecture

900Word uses one durable source of truth:

1. `word-core` owns the normalized document model.
2. ProseMirror is an editing projection in the desktop UI.
3. ODT is the persisted package format for saved documents.
4. Rust import/export code sanitizes external content before it reaches the frontend.

Repository layout:

```text
900Word/
├── apps/desktop/            # Tauri v2 + Svelte 5 desktop app
├── crates/word-core/        # Document model, commands, undo/redo, stats
├── crates/word-docx/        # DOCX import/export conversion boundary
├── crates/word-odf/         # ODT package validation and read/write boundary
├── crates/word-spell/       # Spell-check dictionary boundary
├── crates/word-export/      # TXT, HTML, and lightweight PDF export adapters
├── crates/word-fixtures/    # Sanitized generated fixtures only
├── docs/                    # Public documentation, ADRs, sprint records
└── scripts/                 # Validation and release-preflight scripts
```

## Build From Source

Prerequisites:

- Rust 1.88+
- Node.js 20.19+, 22.12+, or 24+
- Tauri v2 system dependencies for your operating system

```bash
git clone https://github.com/900Labs/900Word.git
cd 900Word
npm install
npm run check
cargo test --workspace
npm run tauri:dev
```

Use the official Tauri prerequisite guide for OS-specific native packages: <https://v2.tauri.app/start/prerequisites/>.

## Validation

Run the local quality gate before opening a pull request:

```bash
./scripts/verify-local.sh
```

Run the public-release privacy gate before changing repository visibility or publishing a release:

```bash
./scripts/verify-public-release.sh
```

## Documentation

- [Documentation index](docs/README.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)
- [Quality gate](docs/QUALITY_GATE.md)
- [Public release checklist](docs/PUBLIC_RELEASE.md)
- [Threat model](docs/THREAT_MODEL.md)
- [Privacy model](docs/PRIVACY_MODEL.md)
- [File format security](docs/FILE_FORMAT_SECURITY.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Contributions must include matching documentation updates when behavior, workflows, public APIs, or contributor expectations change.

## License

900Word is licensed under GPL-3.0-or-later. See [LICENSE](LICENSE) and [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
