#!/usr/bin/env node
/**
 * Build script for Coworks Sidecar
 *
 * Compiles the TypeScript sidecar and copies it to the resources directory
 * for bundling with the Tauri application.
 */

import { execSync } from 'child_process'
import { existsSync, mkdirSync, cpSync, rmSync } from 'fs'
import { join, dirname } from 'path'
import { fileURLToPath } from 'url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const rootDir = join(__dirname, '..')
const sidecarDir = join(rootDir, 'sidecar', 'coworks')
const resourcesDir = join(rootDir, 'src-tauri', 'resources', 'coworks-sidecar')

console.log('Building Coworks Sidecar...')

// Change to sidecar directory
process.chdir(sidecarDir)

// Install dependencies if needed
if (!existsSync(join(sidecarDir, 'node_modules'))) {
  console.log('Installing dependencies...')
  execSync('npm install', { stdio: 'inherit' })
}

// Build TypeScript
console.log('Compiling TypeScript...')
execSync('npm run build', { stdio: 'inherit' })

// Create resources directory
console.log('Copying to resources...')
if (existsSync(resourcesDir)) {
  rmSync(resourcesDir, { recursive: true })
}
mkdirSync(resourcesDir, { recursive: true })

// Copy dist contents directly into resourcesDir (flat, Rust expects coworks-sidecar/index.js)
cpSync(join(sidecarDir, 'dist'), resourcesDir, { recursive: true })

// Copy package.json (needed for module resolution)
cpSync(join(sidecarDir, 'package.json'), join(resourcesDir, 'package.json'))

// Copy node_modules (production only)
console.log('Copying node_modules...')
cpSync(join(sidecarDir, 'node_modules'), join(resourcesDir, 'node_modules'), { recursive: true })

console.log('Coworks Sidecar build complete!')
