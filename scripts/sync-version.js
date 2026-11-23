/**
 * Sync version from package.json to Cargo.toml
 * Run before dev/build to ensure consistent version across frontend and backend
 */

import { readFileSync, writeFileSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const rootDir = join(__dirname, '..');

// Read version from package.json
const packageJson = JSON.parse(readFileSync(join(rootDir, 'package.json'), 'utf-8'));
const version = packageJson.version;

// Update Cargo.toml
const cargoPath = join(rootDir, 'src-tauri', 'Cargo.toml');
let cargoContent = readFileSync(cargoPath, 'utf-8');

// Replace version line in [package] section
cargoContent = cargoContent.replace(
  /^version = "[\d.]+"$/m,
  `version = "${version}"`
);

writeFileSync(cargoPath, cargoContent);

console.log(`✓ Synced version ${version} to Cargo.toml`);
