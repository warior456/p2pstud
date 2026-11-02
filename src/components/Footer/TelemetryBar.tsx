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

export const TelemetryBar: React.FC<TelemetryBarProps> = ({ stats, onOpenSettings }) => {
  const isConnected = stats ? stats.peer_count > 0 : false;
  const peerCount = stats?.peer_count ?? 0;

  const usedGb = stats ? formatGigabytes(stats.used_bytes) : '0.0';
  const quotaDisplay = stats
    ? stats.full_mirror
      ? 'Mirror'
      : `${formatGigabytes(stats.quota_bytes)} GB`
    : '5.0 GB';

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

      {/* Center: Live speeds (swarming simulation) */}
      <div className="flex items-center gap-4 text-gray-400 font-mono">
        <div className="flex items-center gap-1">
          <ArrowUp className="w-3.5 h-3.5 text-blue-400" />
          <span>{isConnected ? '1.2 MB/s' : '0.0 KB/s'}</span>
        </div>
        <div className="flex items-center gap-1">
          <ArrowDown className="w-3.5 h-3.5 text-emerald-400" />
          <span>{isConnected ? '4.5 MB/s' : '0.0 KB/s'}</span>
        </div>
      </div>

      {/* Right: Cache footprint & settings shortcut */}
      <div className="flex items-center gap-3">
        <button
          onClick={onOpenSettings}
          className="flex items-center gap-1.5 text-gray-300 hover:text-white transition-colors"
          title="Click to adjust cache quota slider"
        >
          <Database className="w-3.5 h-3.5 text-sky-400" />
          <span>
            Cache: {usedGb} / {quotaDisplay}
          </span>
        </button>
      </div>
    </footer>
  );
};

