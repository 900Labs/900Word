#!/usr/bin/env node
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { execFileSync } from 'node:child_process';

const outputPath = process.argv[2] ?? 'target/sbom/900word-sbom.json';
const cargoMetadata = JSON.parse(
  execFileSync('cargo', ['metadata', '--locked', '--format-version', '1'], {
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024
  })
);
const packageLock = JSON.parse(readFileSync('package-lock.json', 'utf8'));

const cargoPackages = cargoMetadata.packages.map((pkg) => ({
  ecosystem: 'cargo',
  name: pkg.name,
  version: pkg.version,
  license: pkg.license ?? null,
  repository: pkg.repository ?? null,
  source: pkg.source ?? 'workspace'
}));

const npmPackages = Object.entries(packageLock.packages ?? {})
  .filter(([path]) => path !== '')
  .map(([path, pkg]) => ({
    ecosystem: 'npm',
    name: pkg.name ?? path.replace(/^node_modules\//, ''),
    version: pkg.version ?? null,
    license: pkg.license ?? null,
    resolved: pkg.resolved ?? null
  }));

const sbom = {
  schema: '900word.bootstrap-sbom.v1',
  project: '900Word',
  source: 'generated from Cargo.lock and package-lock.json',
  package_counts: {
    cargo: cargoPackages.length,
    npm: npmPackages.length
  },
  packages: [...cargoPackages, ...npmPackages].sort((a, b) =>
    `${a.ecosystem}:${a.name}:${a.version ?? ''}`.localeCompare(
      `${b.ecosystem}:${b.name}:${b.version ?? ''}`
    )
  )
};

mkdirSync(dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${JSON.stringify(sbom, null, 2)}\n`);
console.log(`SBOM written to ${outputPath}`);
