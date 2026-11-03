import React from 'react';
import { ArrowUp, ArrowDown, Users, Database } from 'lucide-react';
import { StorageStats } from '../../types';

interface TelemetryBarProps {
  stats: StorageStats | null;
  onOpenSettings?: () => void;
}

function formatGigabytes(bytes: number): string {
  return (bytes / (1024 * 1024 * 1024)).toFixed(1);
}

function formatSpeed(bytesPerSec: number): string {
  if (!bytesPerSec || bytesPerSec <= 0) return '0.0 KB/s';
  if (bytesPerSec < 1024 * 1024) {
    return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  }
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
}

export const TelemetryBar: React.FC<TelemetryBarProps> = ({ stats, onOpenSettings }) => {
  const isConnected = stats ? stats.peer_count > 0 : false;
  const peerCount = stats?.peer_count ?? 0;

  const usedGb = stats ? formatGigabytes(stats.used_bytes) : '0.0';
  const quotaDisplay = stats
    ? stats.full_mirror
      ? 'Mirror'
      : `${formatGigabytes(stats.quota_bytes)} GB`
    : '5.0 GB';

  const upSpeed = formatSpeed(stats?.upload_bytes_per_sec ?? 0);
  const downSpeed = formatSpeed(stats?.download_bytes_per_sec ?? 0);

  return (
    <footer className="h-9 bg-gray-900 text-gray-300 text-xs px-4 flex items-center justify-between border-t border-gray-800 select-none">
      {/* Peer Connection status */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-1.5 font-medium">
          <span
            className={`w-2 h-2 rounded-full ${
              isConnected ? 'bg-emerald-400 animate-pulse' : 'bg-gray-500'
            }`}
          />
          <span className={isConnected ? 'text-gray-200' : 'text-gray-400'}>
            {isConnected ? 'Swarm Connected' : 'Seeking Peers'}
          </span>
        </div>

        <div className="h-3 w-px bg-gray-700" />

        <div className="flex items-center gap-1.5 text-gray-400">
          <Users className="w-3.5 h-3.5" />
          <span>Peers: {peerCount}</span>
        </div>
      </div>

      {/* Center: Live dynamic network speeds */}
      <div className="flex items-center gap-4 text-gray-400 font-mono">
        <div className="flex items-center gap-1">
          <ArrowUp className="w-3.5 h-3.5 text-blue-400" />
          <span>{upSpeed}</span>
        </div>
        <div className="flex items-center gap-1">
          <ArrowDown className="w-3.5 h-3.5 text-emerald-400" />
          <span>{downSpeed}</span>
        </div>
      </div>

      {/* Right: Cache footprint & settings shortcut */}
      <div className="flex items-center gap-3">
        <button
          onClick={onOpenSettings}
          className="flex items-center gap-1.5 text-gray-400 hover:text-gray-200 transition-colors cursor-pointer"
          title="Click to view host cache settings"
        >
          <Database className="w-3.5 h-3.5 text-blue-400" />
          <span>Cache: {usedGb} / {quotaDisplay}</span>
        </button>
      </div>
    </footer>
  );
};
