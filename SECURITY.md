# Security Policy

## Reporting a Vulnerability

Do not report security vulnerabilities through public GitHub Issues.

Email `security@900labs.com` with:

- A clear description of the issue.
- Reproduction steps.
- A proof of concept, if safe to share.
- Affected version or commit.
- Operating system and version.
- Suggested fix, if available.

## Scope

In scope:

- Unsafe ODT, DOCX, HTML, TXT, image, font, or dictionary handling.
- Path traversal or filesystem access outside intended locations.
- Tauri IPC commands that fail to validate inputs.
- Cross-site scripting or script execution through imported content.
- Privacy leaks through logs, autosave, recovery, exports, metadata, package artifacts, or runtime network calls.
- Silent document corruption or unsafe save behavior.

Out of scope:

- Social engineering.
- Physical access attacks.
- Vulnerabilities in unmodified dependencies that already have public advisories, unless 900Word needs a project-specific mitigation.

## Security Posture

900Word is local-first and does not include telemetry by default. The primary attack surface is hostile document input, local file operations, Tauri IPC, generated exports, and future optional extension points.

Runtime plugins, cloud sync, external converters, and full encryption are deferred until their security models are accepted.
