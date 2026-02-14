#!/usr/bin/env node

const { execFileSync } = require('child_process');
const { existsSync } = require('fs');
const path = require('path');

/**
 * Detect if we're running on musl libc (like Alpine Linux)
 * @returns {boolean} true if musl is detected
 */
function detectMusl() {
  try {
    if (existsSync('/lib/ld-musl-x86_64.so.1') ||
      existsSync('/lib/ld-musl-aarch64.so.1') ||
      existsSync('/usr/lib/libc.musl-x86_64.so.1') ||
      existsSync('/usr/lib/libc.musl-aarch64.so.1')) {
      return true;
    }

    if (existsSync('/proc/version')) {
      const fs = require('fs');
      const version = fs.readFileSync('/proc/version', 'utf8');
      if (version.includes('musl')) {
        return true;
      }
    }

    return false;
  } catch (error) {
    return false;
  }
}

/**
 * Get the platform-specific package name for the current system
 * @returns {string} the npm package name for this platform
 */
function getPlatformPackageName() {
  const platform = process.platform;
  const arch = process.arch;

  let pkgPlatform;
  let pkgArch;
  let suffix = '';

  switch (platform) {
    case 'linux':
      pkgPlatform = 'linux';
      suffix = detectMusl() ? '-musl' : '-gnu';
      break;
    case 'darwin':
      pkgPlatform = 'darwin';
      break;
    case 'win32':
      pkgPlatform = 'win32';
      break;
    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }

  switch (arch) {
    case 'x64':
      pkgArch = 'x64';
      break;
    case 'arm64':
      pkgArch = 'arm64';
      break;
    default:
      throw new Error(`Unsupported architecture: ${arch}`);
  }

  return `@mehen/${pkgPlatform}-${pkgArch}${suffix}`;
}

/**
 * Find and execute the platform-specific mehen binary
 */
function main() {
  try {
    const pkgName = getPlatformPackageName();
    const binName = process.platform === 'win32' ? 'mehen.exe' : 'mehen';

    let binPath;
    try {
      binPath = require.resolve(`${pkgName}/bin/${binName}`);
    } catch (resolveError) {
      console.error(`Error: Could not find mehen binary for your platform (${pkgName}).`);
      console.error('');
      console.error('This usually means:');
      console.error('1. Optional dependencies were disabled during installation');
      console.error('2. Your platform is not supported');
      console.error('');
      console.error('To fix this:');
      console.error('1. Reinstall with optional dependencies enabled:');
      console.error('   npm install mehen');
      console.error('   # or');
      console.error('   yarn add mehen');
      console.error('');
      console.error('2. If you disabled optional dependencies, re-enable them:');
      console.error('   npm install --include=optional');
      console.error('');
      console.error(`Expected package: ${pkgName}`);
      console.error(`Platform: ${process.platform} ${process.arch}`);
      process.exit(1);
    }

    if (!existsSync(binPath)) {
      console.error(`Error: Binary not found at ${binPath}`);
      console.error('The platform package was installed but the binary is missing.');
      console.error('Please try reinstalling mehen.');
      process.exit(1);
    }

    const args = process.argv.slice(2);

    try {
      execFileSync(binPath, args, {
        stdio: 'inherit',
        windowsHide: false
      });
    } catch (execError) {
      if (execError.status !== undefined) {
        process.exit(execError.status);
      }
      console.error(`Error executing mehen binary: ${execError.message}`);
      process.exit(1);
    }

  } catch (error) {
    console.error(`Error: ${error.message}`);
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}

module.exports = { getPlatformPackageName, detectMusl };
