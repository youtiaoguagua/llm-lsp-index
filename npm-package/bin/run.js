#!/usr/bin/env node

/**
 * Wrapper script to launch the lsp-index binary with proper arguments
 */

const path = require('path');
const { spawn } = require('child_process');

const platform = process.platform;
const binaryName = platform === 'win32' ? 'lsp-index.exe' : 'lsp-index';
const binaryPath = path.join(__dirname, binaryName);

// Forward all arguments to the binary
const args = process.argv.slice(2);

const child = spawn(binaryPath, args, {
  stdio: ['pipe', 'pipe', 'pipe'],
  env: process.env
});

// Pipe stdin/stdout/stderr
process.stdin.pipe(child.stdin);
child.stdout.pipe(process.stdout);
child.stderr.pipe(process.stderr);

child.on('exit', (code) => {
  process.exit(code ?? 0);
});

child.on('error', (err) => {
  console.error('Failed to start lsp-index:', err.message);
  console.error('Make sure the binary is installed (run npm install)');
  process.exit(1);
});
