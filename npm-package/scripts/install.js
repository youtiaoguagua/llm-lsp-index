#!/usr/bin/env node

/**
 * Post-install script: downloads lsp-index binary and LSP servers
 */

const fs = require('fs');
const path = require('path');
const https = require('https');
const { execSync, spawn } = require('child_process');
const os = require('os');

const BIN_DIR = path.join(__dirname, '..', 'bin');
const CACHE_DIR = path.join(os.homedir(), '.lsp-index');
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

// LSP server download configurations
const LSP_SERVERS = {
  java: {
    name: 'JDT Language Server',
    dir: 'jdtls',
    url: 'https://download.eclipse.org/jdtls/snapshots/jdt-language-server-latest.tar.gz',
    checkFile: path.join('plugins', 'org.eclipse.equinox.launcher.jar'),
    installCheck: () => checkCommand('java')
  },
  go: {
    name: 'gopls',
    installCommands: {
      win32: 'go install golang.org/x/tools/gopls@latest',
      default: 'go install golang.org/x/tools/gopls@latest'
    },
    checkCommand: 'gopls',
    installCheck: () => checkCommand('go')
  },
  typescript: {
    name: 'TypeScript Language Server',
    installCommands: {
      win32: 'npm install -g typescript-language-server typescript',
      default: 'npm install -g typescript-language-server typescript'
    },
    checkCommand: 'typescript-language-server',
    installCheck: () => checkCommand('npm')
  }
};

function getBinaryName() {
  const platform = process.platform;
  const arch = process.arch;
  const targetPlatform = PLATFORM_MAP[platform];
  const targetArch = ARCH_MAP[arch];

  if (!targetPlatform || !targetArch) {
    console.error(`Unsupported platform: ${platform}-${arch}`);
    process.exit(1);
  }

  const ext = platform === 'win32' ? '.exe' : '';
  return `lsp-index-${VERSION}-${targetArch}-${targetPlatform}${ext}`;
}

function getArchiveName() {
  const platform = process.platform;
  if (platform === 'win32') {
    return `lsp-index-v${VERSION}-x86_64-pc-windows-msvc.zip`;
  }
  return `lsp-index-v${VERSION}-x86_64-${PLATFORM_MAP[platform]}.tar.gz`;
}

function checkCommand(cmd) {
  try {
    execSync(`${cmd} --version`, { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
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

async function installJavaLsp() {
  const config = LSP_SERVERS.java;
  const installDir = path.join(CACHE_DIR, config.dir);
  const checkFile = path.join(installDir, config.checkFile);

  if (fs.existsSync(checkFile)) {
    console.log(`✓ ${config.name} already installed`);
    return;
  }

  if (!config.installCheck()) {
    console.log(`⚠ Java not found. Skipping ${config.name} installation.`);
    console.log('  Please install Java first: https://adoptium.net/');
    return;
  }

  console.log(`Downloading ${config.name}...`);

  if (!fs.existsSync(CACHE_DIR)) {
    fs.mkdirSync(CACHE_DIR, { recursive: true });
  }

  const tempPath = path.join(CACHE_DIR, 'jdtls.tar.gz');
  await downloadFile(config.url, tempPath);

  // Extract
  const tempExtract = path.join(CACHE_DIR, 'temp_jdtls');
  if (!fs.existsSync(tempExtract)) {
    fs.mkdirSync(tempExtract, { recursive: true });
  }

  if (process.platform === 'win32') {
    try {
      execSync(`tar -xzf "${tempPath}" -C "${tempExtract}"`);
    } catch {
      execSync(`powershell -command "tar -xzf '${tempPath}' -C '${tempExtract}'"`);
    }
  } else {
    execSync(`tar -xzf "${tempPath}" -C "${tempExtract}"`);
  }

  // Move to final location
  const extractedDir = fs.readdirSync(tempExtract).find(d => d.startsWith('jdt-language-server'));
  if (extractedDir) {
    if (fs.existsSync(installDir)) {
      fs.rmSync(installDir, { recursive: true });
    }
    fs.renameSync(path.join(tempExtract, extractedDir), installDir);
    fs.rmSync(tempExtract, { recursive: true });
  }

  fs.unlinkSync(tempPath);
  console.log(`✓ ${config.name} installed`);
}

async function installLspFromCommand(config) {
  if (checkCommand(config.checkCommand)) {
    console.log(`✓ ${config.name} already installed`);
    return;
  }

  if (!config.installCheck()) {
    console.log(`⚠ Prerequisite not found for ${config.name}. Skipping.`);
    return;
  }

  console.log(`Installing ${config.name}...`);
  const cmd = config.installCommands[process.platform] || config.installCommands.default;

  try {
    execSync(cmd, { stdio: 'inherit' });
    console.log(`✓ ${config.name} installed`);
  } catch (error) {
    console.log(`⚠ Failed to install ${config.name}: ${error.message}`);
  }
}

async function main() {
  try {
    await downloadLspIndex();

    // Install Java LSP
    await installJavaLsp();

    // Install other LSPs if their prerequisites exist
    await installLspFromCommand(LSP_SERVERS.go);
    await installLspFromCommand(LSP_SERVERS.typescript);

    console.log('\n✓ Installation complete!');
    console.log(`\nInstalled to:`);
    console.log(`  lsp-index: ${BIN_DIR}`);
    console.log(`  LSP cache: ${CACHE_DIR}`);

  } catch (error) {
    console.error('\n✗ Installation failed:', error.message);
    process.exit(1);
  }
}

main();
