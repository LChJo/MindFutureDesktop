#!/usr/bin/env node
const { runClaude } = require('../lib/index.js');

// Pass command line arguments to claude
const args = process.argv.slice(2);
runClaude(args).catch(err => {
  console.error('[mcd] Error:', err.message);
  process.exit(1);
});
