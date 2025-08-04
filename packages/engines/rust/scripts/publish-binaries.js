#!/usr/bin/env node

/**
 * Script to publish platform-specific binary packages
 * Usage: node scripts/publish-binaries.js [version] [--dry-run] [--retry]
 */

import { execSync } from 'child_process';
import { readFileSync, writeFileSync, existsSync, statSync } from 'fs';
import { join, dirname, basename } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const rustEngineDir = join(__dirname, '..');

const platforms = [
  { name: 'darwin-arm64', binary: 'mdx-hybrid-engine-rust.darwin-arm64.node' },
  { name: 'darwin-x64', binary: 'mdx-hybrid-engine-rust.darwin-x64.node' },
  { name: 'win32-x64-msvc', binary: 'mdx-hybrid-engine-rust.win32-x64-msvc.node' },
  { name: 'linux-x64-gnu', binary: 'mdx-hybrid-engine-rust.linux-x64-gnu.node' },
  { name: 'linux-x64-musl', binary: 'mdx-hybrid-engine-rust.linux-x64-musl.node' },
];

// Color codes for terminal output
const colors = {
  reset: '\x1b[0m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
};

function log(message, color = 'reset') {
  console.log(`${colors[color]}${message}${colors.reset}`);
}

function updatePackageVersion(packagePath, version) {
  try {
    const pkg = JSON.parse(readFileSync(packagePath, 'utf8'));
    pkg.version = version;
    writeFileSync(packagePath, JSON.stringify(pkg, null, 2) + '\n');
    return true;
  } catch (error) {
    log(`❌ Failed to update version in ${packagePath}: ${error.message}`, 'red');
    return false;
  }
}

function verifyBinary(binaryPath) {
  if (!existsSync(binaryPath)) {
    return { valid: false, error: 'Binary file not found' };
  }
  
  try {
    const stats = statSync(binaryPath);
    const sizeInMB = stats.size / (1024 * 1024);
    
    if (stats.size < 1000000) {
      return { valid: false, error: `Binary too small: ${sizeInMB.toFixed(2)} MB` };
    }
    
    return { valid: true, size: sizeInMB.toFixed(2) };
  } catch (error) {
    return { valid: false, error: error.message };
  }
}

async function publishPackage(packageDir, dryRun, retryCount = 3) {
  const packageName = basename(packageDir);
  const command = dryRun
    ? 'npm publish --dry-run --access public'
    : 'npm publish --access public';
  
  for (let attempt = 1; attempt <= retryCount; attempt++) {
    try {
      log(`📦 Publishing ${packageName} (attempt ${attempt}/${retryCount})...`, 'blue');
      execSync(command, { cwd: packageDir, stdio: 'inherit' });
      log(`✅ Successfully published ${packageName}`, 'green');
      return true;
    } catch (error) {
      if (attempt === retryCount) {
        log(`❌ Failed to publish ${packageName} after ${retryCount} attempts`, 'red');
        log(`   Error: ${error.message}`, 'red');
        return false;
      }
      log(`⚠️  Attempt ${attempt} failed, retrying...`, 'yellow');
      // Wait before retry
      await new Promise(resolve => setTimeout(resolve, 2000 * attempt));
    }
  }
}

async function main() {
  const args = process.argv.slice(2);
  const versionArg = args.find(arg => !arg.startsWith('--'));
  const version = versionArg || JSON.parse(readFileSync(join(rustEngineDir, 'package.json'), 'utf8')).version;
  const dryRun = args.includes('--dry-run');
  const enableRetry = args.includes('--retry');

  log(`\n🚀 Publishing binary packages for version ${version}`, 'blue');
  if (dryRun) {
    log('   DRY RUN MODE - No packages will be actually published\n', 'yellow');
  }

  const results = { success: [], skipped: [], failed: [] };

  // Check if binaries exist and publish
  for (const platform of platforms) {
    const binaryPath = join(rustEngineDir, platform.binary);
    const npmDir = join(rustEngineDir, 'npm', platform.name);
    
    log(`\n📦 Processing ${platform.name}...`, 'blue');
    
    // Verify binary exists
    const verification = verifyBinary(binaryPath);
    if (!verification.valid) {
      log(`   ⚠️  Skipping: ${verification.error}`, 'yellow');
      results.skipped.push({ platform: platform.name, reason: verification.error });
      continue;
    }
    
    log(`   ✓ Binary verified: ${verification.size} MB`, 'green');

    // Update version in platform package.json
    const packageJsonPath = join(npmDir, 'package.json');
    if (!updatePackageVersion(packageJsonPath, version)) {
      results.failed.push({ platform: platform.name, reason: 'Failed to update package.json' });
      continue;
    }

    // Copy binary to npm directory
    const targetBinaryPath = join(npmDir, platform.binary);
    try {
      log(`   📋 Copying binary to npm directory...`);
      execSync(`cp "${binaryPath}" "${targetBinaryPath}"`);
    } catch (error) {
      log(`   ❌ Failed to copy binary: ${error.message}`, 'red');
      results.failed.push({ platform: platform.name, reason: 'Failed to copy binary' });
      continue;
    }

    // Publish the package
    const published = await publishPackage(npmDir, dryRun, enableRetry ? 3 : 1);
    if (published) {
      results.success.push(platform.name);
    } else {
      results.failed.push({ platform: platform.name, reason: 'Publishing failed' });
      if (!dryRun && !enableRetry) {
        log('\n💡 Tip: Use --retry flag to enable automatic retries', 'yellow');
      }
    }
  }

  // Update main package optional dependencies
  if (!dryRun && results.success.length > 0) {
    log('\n📝 Updating main package optional dependencies...', 'blue');
    const mainPackagePath = join(rustEngineDir, 'package.json');
    
    try {
      const mainPackage = JSON.parse(readFileSync(mainPackagePath, 'utf8'));
      
      if (mainPackage.optionalDependencies) {
        Object.keys(mainPackage.optionalDependencies).forEach(dep => {
          if (dep.startsWith('@jp-knj/mdx-hybrid-engine-rust-')) {
            mainPackage.optionalDependencies[dep] = version;
          }
        });
        
        writeFileSync(mainPackagePath, JSON.stringify(mainPackage, null, 2) + '\n');
        log('   ✓ Updated optional dependencies in main package.json', 'green');
      }
    } catch (error) {
      log(`   ❌ Failed to update main package.json: ${error.message}`, 'red');
    }
  }

  // Print summary
  log('\n' + '='.repeat(60), 'blue');
  log('📊 Publishing Summary', 'blue');
  log('='.repeat(60), 'blue');
  
  if (results.success.length > 0) {
    log(`\n✅ Successfully published (${results.success.length}):`, 'green');
    results.success.forEach(p => log(`   • ${p}`, 'green'));
  }
  
  if (results.skipped.length > 0) {
    log(`\n⚠️  Skipped (${results.skipped.length}):`, 'yellow');
    results.skipped.forEach(({ platform, reason }) => 
      log(`   • ${platform}: ${reason}`, 'yellow'));
  }
  
  if (results.failed.length > 0) {
    log(`\n❌ Failed (${results.failed.length}):`, 'red');
    results.failed.forEach(({ platform, reason }) => 
      log(`   • ${platform}: ${reason}`, 'red'));
  }
  
  log('\n' + '='.repeat(60) + '\n', 'blue');
  
  // Exit with error if any failures in non-dry-run mode
  if (!dryRun && results.failed.length > 0) {
    process.exit(1);
  }
}

main().catch(error => {
  console.error('Error:', error);
  process.exit(1);
});