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
    console.log(`[prune-target] Removing ${path.relative(process.cwd(), targetPath)}...`);
    fs.rmSync(targetPath, { recursive: true, force: true });
  }
}

export function pruneTargetIfBloated(force = false) {
  if (!fs.existsSync(TARGET_DIR)) return;

  const currentSize = getDirSizeBytes(TARGET_DIR);
  const currentSizeMB = (currentSize / (1024 * 1024)).toFixed(1);

  console.log(`[prune-target] Current ./target size: ${currentSizeMB} MB`);

  if (force || currentSize > MAX_TARGET_BYTES) {
    console.log(`[prune-target] Target size exceeds threshold or force requested. Pruning intermediate debug caches...`);

    // Prune debug folder completely (dev builds recreate in seconds)
    safeRemove(path.join(TARGET_DIR, 'debug'));

    // In release folder, remove intermediate compiler artifacts while preserving bundles & binaries
    safeRemove(path.join(TARGET_DIR, 'release', 'deps'));
    safeRemove(path.join(TARGET_DIR, 'release', 'build'));
    safeRemove(path.join(TARGET_DIR, 'release', 'incremental'));
    safeRemove(path.join(TARGET_DIR, 'release', '.fingerprint'));

    const newSize = getDirSizeBytes(TARGET_DIR);
    console.log(`[prune-target] Done. Target size reduced to ${(newSize / (1024 * 1024)).toFixed(1)} MB.`);
  } else {
    console.log(`[prune-target] Target size is healthy (within 5 GB cap). No pruning needed.`);
  }
}

// Run CLI
const isForce = process.argv.includes('--force');
pruneTargetIfBloated(isForce);
