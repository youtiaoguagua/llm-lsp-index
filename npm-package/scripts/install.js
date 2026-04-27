#!/usr/bin/env node

/**
 * Post-install script: downloads the appropriate binary for the current platform
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

function getBinaryName() {
  const platform = process.platform;
  const arch = process.arch;

  const targetPlatform = PLATFORM_MAP[platform];
  const targetArch = ARCH_MAP[arch];

  if (!targetPlatform || !targetArch) {
    console.error(`Unsupported platform: ${platform}-${arch}`);
    console.error('Supported platforms: macOS (x64, arm64), Linux (x64), Windows (x64)');
    process.exit(1);
  }

  const ext = platform === 'win32' ? '.exe' : '';
  return `lsp-index-${VERSION}-${targetArch}-${targetPlatform}${ext}`;
}

function getArchiveName(binaryName) {
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
        // Follow redirect
        downloadFile(response.headers.location, dest).then(resolve).catch(reject);
        return;
      }
      if (response.statusCode !== 200) {
        reject(new Error(`Download failed: ${response.statusCode}`));
        return;
      }
      response.pipe(file);
      file.on('finish', () => {
        file.close(resolve);
      });
    }).on('error', reject);
  });
}

async function main() {
  // Skip download if binary already exists (dev mode)
  const binaryName = getBinaryName();
  const binaryPath = path.join(BIN_DIR, process.platform === 'win32' ? 'lsp-index.exe' : 'lsp-index');

  if (fs.existsSync(binaryPath)) {
    console.log('Binary already exists, skipping download');
    return;
  }

  console.log(`Downloading lsp-index ${VERSION}...`);

  // Create bin directory
  if (!fs.existsSync(BIN_DIR)) {
    fs.mkdirSync(BIN_DIR, { recursive: true });
  }

  const archiveName = getArchiveName(binaryName);
  const downloadUrl = `https://github.com/youtiaoguagua/llm-lsp-index/releases/download/v${VERSION}/${archiveName}`;
  const tempPath = path.join(BIN_DIR, archiveName);

  try {
    console.log(`Downloading from: ${downloadUrl}`);
    await downloadFile(downloadUrl, tempPath);
    console.log('Download complete');

    // Extract
    console.log('Extracting...');
    if (process.platform === 'win32') {
      // Use PowerShell to extract on Windows
      execSync(`powershell -command "Expand-Archive -Path '${tempPath}' -DestinationPath '${BIN_DIR}' -Force"`, {
        stdio: 'inherit'
      });
    } else {
      // Use tar on Unix
      execSync(`tar -xzf "${tempPath}" -C "${BIN_DIR}"`, { stdio: 'inherit' });
    }

    // Clean up
    fs.unlinkSync(tempPath);
    console.log('Installation complete!');
  } catch (error) {
    console.error('Installation failed:', error.message);
    console.error('You can manually download the binary from:');
    console.error(`https://github.com/youtiaoguagua/llm-lsp-index/releases/tag/v${VERSION}`);
    process.exit(1);
  }
}

main();
