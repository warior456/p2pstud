import fs from 'node:fs';
import path from 'node:path';

const TARGET_DIR = path.resolve(process.cwd(), 'target');
const MAX_TARGET_BYTES = 5 * 1024 * 1024 * 1024; // 5 GB safety cap

function getDirSizeBytes(dirPath) {
  if (!fs.existsSync(dirPath)) return 0;
  let total = 0;
  try {
    const entries = fs.readdirSync(dirPath, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dirPath, entry.name);
      if (entry.isDirectory()) {
        total += getDirSizeBytes(fullPath);
      } else if (entry.isFile()) {
        try {
          total += fs.statSync(fullPath).size;
        } catch {}
      }
    }
  } catch {}
  return total;
}

function safeRemove(targetPath) {
  if (fs.existsSync(targetPath)) {
    try {
      console.log(`[prune-target] Removing ${path.relative(process.cwd(), targetPath)}...`);
      fs.rmSync(targetPath, { recursive: true, force: true });
    } catch (err) {
      console.warn(`[prune-target] Notice: Could not remove ${path.relative(process.cwd(), targetPath)} (${err.message}). Skipping.`);
    }
  }
}

export function pruneTargetIfBloated(force = false) {
  if (!fs.existsSync(TARGET_DIR)) return;

  const currentSize = getDirSizeBytes(TARGET_DIR);
  const currentSizeMB = (currentSize / (1024 * 1024)).toFixed(1);

  console.log(`[prune-target] Current ./target size: ${currentSizeMB} MB`);

  if (force) {
    console.log(`[prune-target] Force requested. Pruning intermediate debug and release compiler caches...`);
    safeRemove(path.join(TARGET_DIR, 'debug'));
    safeRemove(path.join(TARGET_DIR, 'release', 'deps'));
    safeRemove(path.join(TARGET_DIR, 'release', 'build'));
    safeRemove(path.join(TARGET_DIR, 'release', 'incremental'));
    safeRemove(path.join(TARGET_DIR, 'release', '.fingerprint'));
  } else if (currentSize > MAX_TARGET_BYTES) {
    console.log(`[prune-target] Target size exceeds threshold (${MAX_TARGET_BYTES / (1024 * 1024 * 1024)} GB). Pruning intermediate debug cache only to preserve fast incremental builds...`);
    safeRemove(path.join(TARGET_DIR, 'debug'));
  } else {
    console.log(`[prune-target] Target size is healthy. No pruning needed.`);
  }
}

// Run CLI
const isForce = process.argv.includes('--force');
pruneTargetIfBloated(isForce);

