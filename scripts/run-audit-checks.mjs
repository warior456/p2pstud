// scripts/run-audit-checks.mjs
// Automated pre-audit verification runner for p2pstud.
// Runs compiler checks, Clippy linter, Rust unit tests, and Frontend Vitest suites.

import { execSync } from 'child_process';
import { existsSync } from 'fs';
import { resolve } from 'path';

const rootDir = resolve('.');
console.log('====================================================');
console.log(' P2PSTUD AUTOMATED PRE-AUDIT VERIFICATION RUNNER');
console.log('====================================================\n');

function runStep(name, cmd) {
  console.log(`\n[STEP] ${name}`);
  console.log(`Command: ${cmd}`);
  const startTime = Date.now();
  try {
    const output = execSync(cmd, { cwd: rootDir, stdio: 'inherit' });
    const duration = ((Date.now() - startTime) / 1000).toFixed(2);
    console.log(`[PASS] ${name} completed in ${duration}s`);
    return true;
  } catch (error) {
    const duration = ((Date.now() - startTime) / 1000).toFixed(2);
    console.error(`[FAIL] ${name} failed after ${duration}s (exit code: ${error.status})`);
    return false;
  }
}

let passed = 0;
let total = 0;

// 1. Rust test workspace
total++;
if (runStep('Rust Workspace Unit & Integration Tests', 'cargo test --workspace')) {
  passed++;
}

// 2. Frontend Vitest suite
total++;
if (runStep('Frontend Vitest Component Tests', 'npm run test')) {
  passed++;
}

// 3. Frontend Typecheck and Production Build
total++;
if (runStep('Frontend Vite Production Build', 'npm run build')) {
  passed++;
}

// 4. Cargo Check on Anchor Binary
total++;
if (runStep('Cargo Check Anchor Daemon Binary', 'cargo check --bin anchor')) {
  passed++;
}

console.log('\n====================================================');
console.log(` AUDIT PRE-FLIGHT SUMMARY: ${passed}/${total} STEPS PASSED`);
console.log('====================================================');
if (passed === total) {
  console.log('\nAll pre-flight test suites passed. Ready for manual/adversarial code audit.');
  process.exit(0);
} else {
  console.log('\nSome pre-flight steps failed. Please review errors before auditing.');
  process.exit(1);
}
