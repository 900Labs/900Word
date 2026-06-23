#!/usr/bin/env node
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { basename, join } from 'node:path';
import { hostname } from 'node:os';

const roots = process.argv.slice(2);
const defaultRoots = [
  'apps/desktop/src-tauri/target/release/bundle',
  'apps/desktop/dist'
];
const scanRoots = roots.length > 0 ? roots : defaultRoots;
const requireArtifacts = process.env.REQUIRE_PACKAGE_ARTIFACTS === '1';
const maxFileBytes = Number(process.env.PACKAGE_SCAN_MAX_FILE_BYTES ?? 128 * 1024 * 1024);

const blockedNames = new Set(['.DS_Store', 'Thumbs.db', 'desktop.ini']);
const blockedExtensions = ['.pdb', '.dSYM', '.map', '.debug'];
const blockedRegexes = [
  { name: 'local user path', regex: /\/Users\/[A-Za-z0-9._-]+|\/home\/[A-Za-z0-9._-]+|C:\\Users\\[^\\\r\n]+|Desktop\/[A-Za-z0-9._-]+/ },
  { name: 'private key material', regex: /-----BEGIN (RSA |OPENSSH |)PRIVATE KEY-----/ },
  { name: 'development server URL', regex: /http:\/\/localhost:5173|http:\/\/127\.0\.0\.1:5173/ },
  { name: 'debug source root', regex: /\/target\/debug\/|\\target\\debug\\/ }
];
const localHost = hostname().split('.')[0];
if (localHost.length > 1) {
  blockedRegexes.push({
    name: 'local hostname',
    regex: new RegExp(escapeRegExp(localHost))
  });
}

const existingRoots = scanRoots.filter((root) => exists(root));
if (existingRoots.length === 0) {
  if (requireArtifacts) {
    console.error(`ERROR: no package artifact roots found: ${scanRoots.join(', ')}`);
    process.exit(1);
  }
  console.log('No package artifact roots found; package scan skipped');
  process.exit(0);
}

let scannedFiles = 0;
const findings = [];

for (const root of existingRoots) {
  walk(root, (file) => {
    scannedFiles += 1;
    const name = basename(file);
    if (blockedNames.has(name) || blockedExtensions.some((ext) => name.endsWith(ext))) {
      findings.push(`${file}: blocked generated/debug file`);
      return;
    }

    const stats = statSync(file);
    if (stats.size > maxFileBytes) {
      findings.push(`${file}: file exceeds package scan limit (${stats.size} bytes)`);
      return;
    }

    const text = readFileSync(file).toString('latin1');
    for (const { name: patternName, regex } of blockedRegexes) {
      if (regex.test(text)) {
        findings.push(`${file}: ${patternName}`);
      }
    }
  });
}

if (findings.length > 0) {
  for (const finding of findings) {
    console.error(finding);
  }
  console.error(`ERROR: package artifact scan failed with ${findings.length} finding(s)`);
  process.exit(1);
}

if (requireArtifacts && scannedFiles === 0) {
  console.error('ERROR: package artifact roots contained no files');
  process.exit(1);
}

console.log(`Package artifact scan passed (${scannedFiles} file(s))`);

function walk(root, onFile) {
  const stats = statSync(root);
  if (stats.isFile()) {
    onFile(root);
    return;
  }
  if (!stats.isDirectory()) {
    return;
  }
  for (const entry of readdirSync(root)) {
    walk(join(root, entry), onFile);
  }
}

function exists(path) {
  try {
    statSync(path);
    return true;
  } catch {
    return false;
  }
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
