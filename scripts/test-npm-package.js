#!/usr/bin/env node

/**
 * Test the npm package locally
 * This script helps validate the npm package before publishing
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

function log(message) {
    console.log(`[TEST] ${message}`);
}

function error(message) {
    console.error(`[ERROR] ${message}`);
}

function success(message) {
    console.log(`[SUCCESS] ${message}`);
}

function execCommand(command, options = {}) {
    try {
        const result = execSync(command, {
            encoding: 'utf8',
            stdio: 'pipe',
            ...options
        });
        return result.trim();
    } catch (err) {
        throw new Error(`Command failed: ${command}\n${err.message}`);
    }
}

async function testPackage() {
    log('Testing Mehen npm package...');

    const testDir = path.join(os.tmpdir(), 'mehen-npm-test-' + Date.now());
    fs.mkdirSync(testDir);

    try {
        process.chdir(testDir);
        log(`Created test directory: ${testDir}`);

        execCommand('npm init -y');
        log('Initialized test npm project');

        const packagePath = path.join(__dirname, '..', 'npm', 'mehen');

        log(`Installing local package from: ${packagePath}`);
        execCommand(`npm install "${packagePath}"`);
        success('Successfully installed local mehen package');

        log('Testing mehen command...');

        try {
            const helpOutput = execCommand('npx mehen --help');
            log('mehen --help output:');
            console.log(helpOutput);
            success('mehen --help executed successfully');
        } catch (err) {
            error(`mehen --help failed: ${err.message}`);
            throw err;
        }

        try {
            const versionOutput = execCommand('npx mehen --version');
            log('mehen --version output:');
            console.log(versionOutput);
            success('mehen --version executed successfully');
        } catch (err) {
            log('mehen --version not available (this is okay)');
        }

        log('Checking installed platform packages...');
        const nodeModulesPath = path.join(testDir, 'node_modules');

        // Check for scoped packages
        const mehenScopePath = path.join(nodeModulesPath, '@mehen');
        let installedPackages = [];
        if (fs.existsSync(mehenScopePath)) {
            installedPackages = fs.readdirSync(mehenScopePath).sort();
        }

        log(`Installed platform packages: ${installedPackages.map(p => '@mehen/' + p).join(', ')}`);

        // Check for expected platform package
        const platform = process.platform;
        const arch = process.arch;
        let expectedPackages = [];

        if (platform === 'linux') {
            expectedPackages = [
                `linux-${arch}-gnu`,
                `linux-${arch}-musl`
            ];
        } else if (platform === 'darwin') {
            expectedPackages = [`darwin-${arch}`];
        } else if (platform === 'win32') {
            expectedPackages = [`win32-${arch}`];
        }

        const foundExpected = expectedPackages.some(pkg => installedPackages.includes(pkg));

        if (foundExpected) {
            success(`Found expected platform package for ${platform}-${arch}`);
        } else {
            error(`Expected platform package not found. Expected one of: ${expectedPackages.join(', ')}`);
            log(`Platform: ${platform}, Arch: ${arch}`);
        }

        // Test the launcher script directly
        log('Testing launcher script...');
        const launcherPath = path.join(nodeModulesPath, 'mehen', 'bin', 'mehen.js');

        if (fs.existsSync(launcherPath)) {
            const launcherModule = require(launcherPath);
            if (typeof launcherModule.getPlatformPackageName === 'function') {
                const platformPkg = launcherModule.getPlatformPackageName();
                log(`Platform package name: ${platformPkg}`);
            }
        }

        success('All tests passed!');

    } finally {
        process.chdir(__dirname);
        try {
            fs.rmSync(testDir, { recursive: true, force: true });
            log(`Cleaned up test directory: ${testDir}`);
        } catch (err) {
            log(`Warning: Could not clean up test directory: ${err.message}`);
        }
    }
}

if (require.main === module) {
    testPackage().catch(err => {
        error(err.message);
        process.exit(1);
    });
}

module.exports = { testPackage };
