#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const rootDir = fileURLToPath(new URL('..', import.meta.url));

const checks = [];
const errors = [];

const versionPattern = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

const formatVersion = (value) => {
  if (typeof value !== 'string' || value.trim() === '') {
    return '<missing>';
  }

  const trimmed = value.trim();
  return /^[0-9A-Za-z][0-9A-Za-z.+-]*$/.test(trimmed) ? trimmed : '<non-version value>';
};

const readText = (relativePath) => {
  try {
    return fs.readFileSync(path.join(rootDir, relativePath), 'utf8');
  } catch (error) {
    const reason = error && typeof error === 'object' && 'code' in error ? error.code : 'read failed';
    errors.push(`unable to read ${relativePath}: ${reason}`);
    return undefined;
  }
};

const readJson = (relativePath) => {
  const text = readText(relativePath);
  if (text === undefined) {
    return undefined;
  }

  try {
    return JSON.parse(text);
  } catch (error) {
    const reason = error instanceof SyntaxError ? error.message : 'invalid JSON';
    errors.push(`unable to parse ${relativePath}: ${reason}`);
    return undefined;
  }
};

const rootPackage = readJson('package.json');
const expectedVersion = typeof rootPackage?.version === 'string' ? rootPackage.version.trim() : undefined;

const addCheck = (label, actual, source) => {
  const normalizedActual = typeof actual === 'string' ? actual.trim() : actual;
  checks.push({ label, actual: normalizedActual, source });
  if (!expectedVersion || normalizedActual !== expectedVersion) {
    errors.push(`${label} in ${source} is ${formatVersion(normalizedActual)}; expected ${formatVersion(expectedVersion)}`);
  }
};

const matchRequired = (text, pattern, label, source) => {
  if (text === undefined) {
    return undefined;
  }

  const match = text.match(pattern);
  if (!match) {
    errors.push(`${label} in ${source} was not found`);
    return undefined;
  }
  return match[1].trim();
};

if (!expectedVersion) {
  errors.push('root package version in package.json is <missing>; expected a non-empty version string');
} else if (!versionPattern.test(expectedVersion)) {
  errors.push(`root package version in package.json is ${formatVersion(expectedVersion)}; expected semver-like x.y.z`);
} else {
  addCheck('root package version', expectedVersion, 'package.json');
}

const packageLock = readJson('package-lock.json');
if (packageLock) {
  addCheck('package-lock top-level version', packageLock.version, 'package-lock.json');
  addCheck('package-lock root package version', packageLock.packages?.['']?.version, 'package-lock.json');
  addCheck(
    'package-lock desktop workspace version',
    packageLock.packages?.['apps/desktop']?.version,
    'package-lock.json'
  );
}

const desktopPackage = readJson('apps/desktop/package.json');
if (desktopPackage) {
  addCheck('desktop package version', desktopPackage.version, 'apps/desktop/package.json');
}

const tauriConfig = readJson('apps/desktop/src-tauri/tauri.conf.json');
if (tauriConfig) {
  addCheck('Tauri config version', tauriConfig.version, 'apps/desktop/src-tauri/tauri.conf.json');
}

const cargoToml = readText('Cargo.toml');
const workspacePackageSection = matchRequired(
  cargoToml,
  /\[workspace\.package\]([\s\S]*?)(?=\n\[|$)/,
  'workspace package section',
  'Cargo.toml'
);
if (workspacePackageSection) {
  const cargoWorkspaceVersion = matchRequired(
    workspacePackageSection,
    /^\s*version\s*=\s*"([^"]+)"/m,
    'workspace package version',
    'Cargo.toml'
  );
  if (cargoWorkspaceVersion) {
    addCheck('Cargo workspace package version', cargoWorkspaceVersion, 'Cargo.toml');
  }
}

const appSvelte = readText('apps/desktop/src/App.svelte');
const aboutVersionMatches = [
  ...(appSvelte?.matchAll(/<dt>\s*\{tr\((['"])version\1\)\}\s*<\/dt>\s*<dd>([^<]+)<\/dd>/g) ?? []),
];
if (aboutVersionMatches.length === 0) {
  errors.push('visible About/version string in apps/desktop/src/App.svelte was not found');
} else {
  for (const [index, match] of aboutVersionMatches.entries()) {
    addCheck(
      `visible About/version string${aboutVersionMatches.length > 1 ? ` #${index + 1}` : ''}`,
      match[2].trim(),
      'apps/desktop/src/App.svelte'
    );
  }
}

if (errors.length > 0) {
  console.error(`Version consistency failed; root package.json version is ${formatVersion(expectedVersion)}.`);
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

console.log(`Version consistency passed: ${expectedVersion} across ${checks.length} surfaces`);
