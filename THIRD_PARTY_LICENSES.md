# Third-Party Licenses

900Word is licensed under GPL-3.0-or-later.

Dependency, font, and dictionary licenses must be tracked before public release. The current bootstrap allows common permissive and GPL-compatible licenses through `deny.toml`, but maintainers must verify:

- Rust crate licenses with `cargo deny check`.
- npm package licenses before binary distribution.
- Hunspell dictionary licenses before bundling.
- Font licenses before bundling.
- Any generated SBOM before publishing release artifacts.

No third-party document fixtures from real users may be committed.

## Bundled Dictionaries

| Component | Location | License | Notes |
| --- | --- | --- | --- |
| English bootstrap Hunspell dictionary | `crates/word-spell/dictionaries/en_US/` | GPL-3.0-or-later | Generated specifically for 900Word. Minimal word list for offline bootstrap behavior and tests; not a complete English dictionary. |

User dictionaries loaded from `{APP_DATA_DIR}/dictionaries` are user-provided content. They are not redistributed by this project and must not be added to release artifacts unless their license is reviewed and recorded here.
