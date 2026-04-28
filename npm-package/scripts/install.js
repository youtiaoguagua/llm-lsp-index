#!/usr/bin/env node

/**
 * Post-install script: downloads lsp-index binary
 * LSP servers (JDT LS, etc.) are now auto-downloaded by the Rust binary
 */

const fs = require('fs');
const path = require('path');
const https = require('https');
const { execSync } = require('child_process');

const BIN_DIR = path.join(__dirname, '..', 'bin');
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
    const chunks = [];
    https.get(url, (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        downloadFile(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }
      if (response.statusCode !== 200) {
        reject(new Error(`HTTP ${response.statusCode}`));
        return;
      }
      response.on('data', chunk => chunks.push(chunk));
      response.on('end', () => {
        const buffer = Buffer.concat(chunks);
        fs.writeFileSync(dest, buffer);
        resolve();
      });
    }).on('error', reject);
  });
}

async function downloadLspIndex() {
  const binaryPath = path.join(BIN_DIR, process.platform === 'win32' ? 'lsp-index.exe' : 'lsp-index');

  if (fs.existsSync(binaryPath)) {
    console.log('✓ lsp-index binary already exists');
    return;
  }

  console.log(`Downloading lsp-index ${VERSION}...`);

  if (!fs.existsSync(BIN_DIR)) {
    fs.mkdirSync(BIN_DIR, { recursive: true });
  }

  const archiveName = getArchiveName();
  const downloadUrl = `https://github.com/youtiaoguagua/llm-lsp-index/releases/download/v${VERSION}/${archiveName}`;
  const tempPath = path.join(BIN_DIR, archiveName);

  await downloadFile(downloadUrl, tempPath);

  if (process.platform === 'win32') {
    execSync(`powershell -command "Expand-Archive -Path '${tempPath}' -DestinationPath '${BIN_DIR}' -Force"`);
  } else {
    execSync(`tar -xzf "${tempPath}" -C "${BIN_DIR}"`);
  }

  fs.unlinkSync(tempPath);
  console.log('✓ lsp-index installed');
}

async function main() {
  try {
    await downloadLspIndex();
    console.log('\n✓ Installation complete!');
    console.log('\nNote: LSP servers (JDT LS, etc.) will be auto-downloaded on first use.');
  } catch (error) {
    console.error('\n✗ Installation failed:', error.message);
    process.exit(1);
  }
}

main();
