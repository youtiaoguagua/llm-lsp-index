#!/usr/bin/env node

/**
 * Wrapper script to launch the lsp-index binary with proper arguments
 * Automatically downloads binary if not present
 */

const path = require('path');
const fs = require('fs');
const https = require('https');
const { spawn, execSync } = require('child_process');

const platform = process.platform;
const binaryName = platform === 'win32' ? 'lsp-index.exe' : 'lsp-index';
const binaryPath = path.join(__dirname, binaryName);

const VERSION = require('../package.json').version;

const PLATFORM_MAP = {
  'darwin': 'apple-darwin',
  'linux': 'unknown-linux-gnu',
  'win32': 'pc-windows-msvc'
};

const ARCH_MAP = {
  'x64': 'x86_64',
  'arm64': 'aarch64'
};

function getArchiveName() {
  const platform = process.platform;
  if (platform === 'win32') {
    return `lsp-index-v${VERSION}-x86_64-pc-windows-msvc.zip`;
  }
  return `lsp-index-v${VERSION}-x86_64-${PLATFORM_MAP[platform]}.tar.gz`;
}

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        downloadFile(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }
      if (response.statusCode !== 200) {
        reject(new Error(`HTTP ${response.statusCode}`));
        return;
      }
      response.pipe(file);
      file.on('finish', () => file.close(resolve));
    }).on('error', reject);
  });
}

async function ensureBinary() {
  if (fs.existsSync(binaryPath)) {
    return;
  }

  console.log(`Binary not found at ${binaryPath}`);
  console.log(`Downloading lsp-index ${VERSION}...`);

  const archiveName = getArchiveName();
  const downloadUrl = `https://github.com/youtiaoguagua/llm-lsp-index/releases/download/v${VERSION}/${archiveName}`;
  const tempPath = path.join(__dirname, archiveName);

  try {
    await downloadFile(downloadUrl, tempPath);
    console.log('Download complete, extracting...');

    if (process.platform === 'win32') {
      execSync(`powershell -command "Expand-Archive -Path '${tempPath}' -DestinationPath '${__dirname}' -Force"`);
    } else {
      execSync(`tar -xzf "${tempPath}" -C "${__dirname}"`);
    }

    fs.unlinkSync(tempPath);
    console.log('Binary installed successfully!');
  } catch (error) {
    throw new Error(`Failed to download binary: ${error.message}`);
  }
}

async function main() {
  try {
    // Ensure binary exists (download if needed)
    await ensureBinary();

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
      process.exit(1);
    });
  } catch (error) {
    console.error('Error:', error.message);
    console.error('\nYou can manually download the binary from:');
    console.error(`https://github.com/youtiaoguagua/llm-lsp-index/releases/tag/v${VERSION}`);
    process.exit(1);
  }
}

main();
