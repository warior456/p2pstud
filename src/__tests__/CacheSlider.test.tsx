import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { CacheSlider } from '../components/Settings/CacheSlider';
import { StorageStats } from '../types';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

describe('CacheSlider Component', () => {
  const mockStats: StorageStats = {
    used_bytes: 2147483648, // 2 GB
    quota_bytes: 5368709120, // 5 GB
    pinned_bytes: 536870912, // 512 MB
    peer_count: 2,
    full_mirror: false,
  };

  it('renders quota and used storage correctly', () => {
    render(<CacheSlider stats={mockStats} />);

    expect(screen.getByText(/Host-to-Play Cache Allocation/i)).toBeInTheDocument();
    expect(screen.getByText(/2.00 GB/i)).toBeInTheDocument();
    expect(screen.getAllByText(/5 GB/i).length).toBeGreaterThan(0);
  });

  it('triggers set_cache_limit on slider adjustment', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    render(<CacheSlider stats={mockStats} />);

    const slider = screen.getByLabelText(/Cache Quota Limit/i);
    fireEvent.change(slider, { target: { value: '10' } });

    expect(invoke).toHaveBeenCalledWith('set_cache_limit', {
      bytes: 10 * 1024 * 1024 * 1024,
      fullMirror: false,
    });
  });

  it('toggles Full Mirror mode correctly', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    render(<CacheSlider stats={mockStats} />);

    const checkbox = screen.getByLabelText(/Enable Full Mirror Mode/i);
    fireEvent.click(checkbox);

    expect(invoke).toHaveBeenCalledWith('set_cache_limit', {
      bytes: null,
      fullMirror: true,
    });
  });
});
