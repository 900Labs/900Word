# Privacy Model

## Local Data

900Word uses user-selected document paths for saved files. Future autosave and recovery files must use `{APP_DATA_DIR}` and must be documented before release.

## Logs

Logs may include high-level operation names and error categories. Logs must not include document text, private filenames, local paths, or recovered content.

## Metadata

Exporters must avoid adding local usernames, hostnames, absolute paths, or private build metadata to ODT, HTML, TXT, PDF, or EPUB outputs.

## Network

Core editing workflows must run offline. Any future network feature must be opt-in and documented in an ADR.
