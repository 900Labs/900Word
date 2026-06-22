# Public Release Checklist

900Word stays private until this checklist passes on `main`.

## Source Scan

```bash
./scripts/verify-public-release.sh
git status --short --branch
```

Expected result:

1. No local user paths.
2. No hostnames.
3. No secrets or private keys.
4. No OS metadata files.
5. No real user documents.
6. No generated package artifacts.

## Package Scan

Before publishing binary artifacts:

1. Build package artifacts manually.
2. Unpack each artifact.
3. Search unpacked files for local paths, hostnames, secrets, debug files, and private sample content.
4. Confirm no telemetry or unsolicited network calls during runtime smoke.

## Documentation Scan

Docs must use placeholders such as:

- `YOUR_GITHUB_USERNAME`
- `{REPO_ROOT}`
- `{APP_DATA_DIR}`

Do not include local computer paths or personal names.
