import { render, screen } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { TelemetryBar } from '../components/Footer/TelemetryBar';
import { StorageStats } from '../types';

describe('TelemetryBar Component', () => {
  it('renders correctly when disconnected (peers = 0)', () => {
    const emptyStats: StorageStats = {
      used_bytes: 0,
      quota_bytes: 5368709120, // 5 GB
      pinned_bytes: 0,
      peer_count: 0,
      full_mirror: false,
      upload_bytes_per_sec: 0,
      download_bytes_per_sec: 0,
    };

    render(<TelemetryBar stats={emptyStats} />);

    expect(screen.getByText(/Peers: 0/i)).toBeInTheDocument();
    expect(screen.getByText(/Seeking Peers/i)).toBeInTheDocument();
    expect(screen.getByText(/0.0 \/ 5.0 GB/i)).toBeInTheDocument();
  });

  it('renders active swarm with connected peers and dynamic bandwidth speeds', () => {
    const activeStats: StorageStats = {
      used_bytes: 2470000000,
      quota_bytes: 5368709120,
      pinned_bytes: 500000000,
      peer_count: 3,
      full_mirror: false,
      upload_bytes_per_sec: 1250000, // 1.2 MB/s
      download_bytes_per_sec: 4718592, // 4.5 MB/s
    };

    render(<TelemetryBar stats={activeStats} />);

    expect(screen.getByText(/Peers: 3/i)).toBeInTheDocument();
    expect(screen.getByText(/Swarm Connected/i)).toBeInTheDocument();
    expect(screen.getByText(/1.2 MB\/s/i)).toBeInTheDocument();
    expect(screen.getByText(/4.5 MB\/s/i)).toBeInTheDocument();
  });

  it('displays Mirror indicator when full_mirror is true', () => {
    const mirrorStats: StorageStats = {
      used_bytes: 8589934592,
      quota_bytes: Number.MAX_SAFE_INTEGER,
      pinned_bytes: 1000000000,
      peer_count: 5,
      full_mirror: true,
      upload_bytes_per_sec: 0,
      download_bytes_per_sec: 0,
    };

    render(<TelemetryBar stats={mirrorStats} />);

    expect(screen.getByText(/Mirror/i)).toBeInTheDocument();
  });
});
