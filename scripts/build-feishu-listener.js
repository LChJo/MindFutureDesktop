#!/usr/bin/env node
/**
 * Build script for Feishu Event Listener
 *
 * Bundles event-listener.cjs with all npm dependencies into a single file
 * using esbuild, eliminating the need for node_modules at runtime.
 *
 * Source:  sidecar/feishu/event-listener.cjs
 * Output:  src-tauri/resources/feishu/event-listener.cjs
 */

import { existsSync, mkdirSync } from 'fs'
import { join, dirname } from 'path'
import { fileURLToPath } from 'url'
import { execSync } from 'child_process'
import esbuild from 'esbuild'

const __dirname = dirname(fileURLToPath(import.meta.url))
const rootDir = join(__dirname, '..')
const sidecarDir = join(rootDir, 'sidecar', 'feishu')
const resourcesDir = join(rootDir, 'src-tauri', 'resources', 'feishu')
const entryFile = join(sidecarDir, 'event-listener.cjs')
const outFile = join(resourcesDir, 'event-listener.cjs')

console.log('Building Feishu Event Listener...')

// Install dependencies if needed
if (!existsSync(join(sidecarDir, 'node_modules'))) {
  console.log('Installing dependencies...')
  execSync('npm install', { cwd: sidecarDir, stdio: 'inherit' })
}

// Ensure output directory exists
mkdirSync(resourcesDir, { recursive: true })

// Bundle with esbuild
console.log('Bundling with esbuild...')
const result = await esbuild.build({
  entryPoints: [entryFile],
  bundle: true,
  platform: 'node',
  target: 'node18',
  format: 'cjs',
  outfile: outFile,
  // Tree-shake unused code
  treeShaking: true,
  // Keep the code readable for debugging
  minify: false,
  // Handle optional native addons that ws/protobufjs try to require
  // These are optional and the SDK works without them
  external: [
    'bufferutil',
    'utf-8-validate',
  ],
  // Suppress warnings about dynamic requires in optional deps
  logLevel: 'warning',
})

if (result.errors.length > 0) {
  console.error('Build failed:', result.errors)
  process.exit(1)
}

if (result.warnings.length > 0) {
  console.warn('Build warnings:', result.warnings)
}

console.log(`Feishu Event Listener bundled successfully: ${outFile}`)
