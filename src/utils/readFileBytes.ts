import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/**
 * Read a file's bytes from the backend, transparently handling large files.
 *
 * The Rust `read_file_bytes` command returns files up to 8 MB inline. Larger
 * files are streamed over the `file-chunk` / `file-chunk-done` events instead
 * of being returned (the command returns an empty array). This helper collects
 * those chunks and assembles the complete buffer for the caller.
 */
export async function readFileBytes(hash: string): Promise<Uint8Array> {
  let total = 0;
  let done = false;
  let resolveDone: (() => void) | null = null;
  let donePromise: Promise<void> = new Promise((resolve) => {
    resolveDone = resolve;
  });

  const chunks: { offset: number; data: number[] }[] = [];

  const unlistenChunk: UnlistenFn = await listen<{
    hash: string;
    offset: number;
    total: number;
    data: number[];
  }>('file-chunk', (event) => {
    if (event.payload.hash !== hash) return;
    total = event.payload.total;
    chunks.push({ offset: event.payload.offset, data: event.payload.data });
  });

  const unlistenDone: UnlistenFn = await listen<{
    hash: string;
    total: number;
  }>('file-chunk-done', (event) => {
    if (event.payload.hash !== hash) return;
    total = event.payload.total;
    done = true;
    resolveDone?.();
  });

  try {
    // If a previous read for this hash left chunks behind, they are collected
    // too; offsets make concatenation deterministic.
    const inline = await invoke<number[]>('read_file_bytes', { hash });
    if (inline && inline.length > 0) {
      return new Uint8Array(inline);
    }

    if (!done) {
      // The invoke resolves after the backend emitted the chunks, but event
      // delivery is asynchronous. Wait for the completion signal.
      await donePromise;
    }

    if (total === 0 && chunks.length === 0) {
      return new Uint8Array(0);
    }

    const merged = new Uint8Array(total);
    let filled = 0;
    for (const chunk of chunks.sort((a, b) => a.offset - b.offset)) {
      merged.set(Uint8Array.from(chunk.data), chunk.offset);
      filled += chunk.data.length;
    }

    return filled === merged.length ? merged : merged.subarray(0, filled);
  } finally {
    unlistenChunk();
    unlistenDone();
    resolveDone = null;
  }
}
