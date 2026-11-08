import { describe, it, expect, vi, beforeEach } from 'vitest';
import { listen } from '@tauri-apps/api/event';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

import { readFileBytes } from '../utils/readFileBytes';

type Handler = (event: { payload: any }) => void;

const handlers: Record<string, Handler> = {};

beforeEach(() => {
  for (const key of Object.keys(handlers)) {
    delete handlers[key];
  }
  invokeMock.mockReset();
  vi.mocked(listen).mockImplementation((async (name: string, cb: Handler) => {
    handlers[name] = cb;
    return () => {};
  }) as any);
});

describe('readFileBytes', () => {
  it('returns inline bytes for small files', async () => {
    invokeMock.mockResolvedValue([1, 2, 3]);

    const result = await readFileBytes('small-hash');

    expect(Array.from(result)).toEqual([1, 2, 3]);
    expect(invokeMock).toHaveBeenCalledWith('read_file_bytes', { hash: 'small-hash' });
  });

  it('assembles streamed chunks for files larger than the inline limit', async () => {
    invokeMock.mockImplementation(async () => {
      // The backend emits all chunks before the command resolves.
      handlers['file-chunk']({
        payload: { hash: 'big-hash', offset: 0, total: 6, data: [1, 2, 3] },
      });
      handlers['file-chunk']({
        payload: { hash: 'big-hash', offset: 3, total: 6, data: [4, 5, 6] },
      });
      handlers['file-chunk-done']({ payload: { hash: 'big-hash', total: 6 } });
      return [];
    });

    const result = await readFileBytes('big-hash');

    expect(Array.from(result)).toEqual([1, 2, 3, 4, 5, 6]);
  });

  it('ignores chunks belonging to a different file', async () => {
    invokeMock.mockImplementation(async () => {
      handlers['file-chunk']({
        payload: { hash: 'other-hash', offset: 0, total: 2, data: [9, 9] },
      });
      handlers['file-chunk']({
        payload: { hash: 'target-hash', offset: 0, total: 2, data: [7, 8] },
      });
      handlers['file-chunk-done']({ payload: { hash: 'target-hash', total: 2 } });
      return [];
    });

    const result = await readFileBytes('target-hash');

    expect(Array.from(result)).toEqual([7, 8]);
  });
});
