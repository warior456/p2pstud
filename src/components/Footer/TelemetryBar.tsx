import { Users, Database, Shield, Wifi, PackageOpen } from 'lucide-react';
import { StorageStats, NetworkProfileInfo, ImportProgress } from '../../types';

interface TelemetryBarProps {
  stats: StorageStats | null;
  networkProfile?: NetworkProfileInfo | null;
  importProgress?: ImportProgress | null;
  onOpenSettings?: () => void;
  onOpenNetworkSettings?: () => void;
  onOpenBandwidthSettings?: () => void;
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

export const TelemetryBar: React.FC<TelemetryBarProps> = ({ 
  stats, 
  networkProfile,
  importProgress,
  onOpenSettings,
  onOpenNetworkSettings,
  onOpenBandwidthSettings 
}) => {
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

  const isCampus = networkProfile?.profile === 'campus';
  const ssidName = networkProfile?.ssid || 'Unknown SSID';

  return (
    <footer className="h-9 bg-gray-900 text-gray-300 text-xs px-4 flex items-center justify-between border-t border-gray-800 select-none">
      {/* Left: Peer Connection status & Network Profile Badge */}
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
          <span
            className="text-[10px] bg-gray-800 text-emerald-300 px-1.5 py-0.5 rounded font-mono border border-emerald-900/50"
            title={`Application Build Version: v${stats?.current_version || '0.1.45'}`}
          >
            v{stats?.current_version || '0.1.45'}
          </span>
          {stats?.swarm_median_version && stats.swarm_median_version !== stats.current_version && (
            <span
              className="text-[10px] bg-gray-800 text-blue-300 px-1.5 py-0.5 rounded font-mono border border-blue-900/50"
              title={`Swarm Median Version: v${stats.swarm_median_version}`}
            >
              Swarm: v{stats.swarm_median_version}
            </span>
          )}
        </div>

        {/* Network Stealth / Campus Profile Badge */}
        {networkProfile && (
          <>
            <div className="h-3 w-px bg-gray-700" />
            <button
              onClick={onOpenNetworkSettings}
              className={`flex items-center gap-1.5 px-2 py-0.5 rounded text-[11px] font-medium transition-colors cursor-pointer border ${
                isCampus
                  ? 'bg-amber-950/60 text-amber-300 border-amber-800/80 hover:bg-amber-900/60'
                  : 'bg-emerald-950/60 text-emerald-300 border-emerald-800/80 hover:bg-emerald-900/60'
              }`}
              title={`Network: ${ssidName} (${isCampus ? 'Campus Stealth Mode - Relay only' : 'Trusted Network - LAN mDNS active'}). Click to manage SSID rules.`}
            >
              {isCampus ? (
                <>
                  <Shield className="w-3 h-3 text-amber-400" />
                  <span>Campus Stealth ({ssidName})</span>
                  {networkProfile.lecture_throttle_active && (
                    <span className="text-[9px] bg-amber-500/30 text-amber-200 px-1 rounded">Lecture Hrs</span>
                  )}
                </>
              ) : (
                <>
                  <Wifi className="w-3 h-3 text-emerald-400" />
                  <span>Trusted ({ssidName})</span>
                </>
              )}
            </button>
          </>
        )}
      </div>

      {/* Center: Live dynamic network speeds & active importing badge */}
      <div className="flex items-center gap-4 font-mono text-xs">
        <button
          onClick={onOpenBandwidthSettings}
          className="flex items-center gap-4 bg-gray-950/70 hover:bg-gray-800/80 px-2.5 py-1 rounded border border-gray-800/80 transition-colors cursor-pointer"
          title="Live P2P Network Speeds. Click to configure Bandwidth & Battery Defense limits."
        >
          <div className="flex items-center gap-1.5 text-blue-400 font-medium">
            <span className="text-sm">↑</span>
            <span>{upSpeed}</span>
          </div>
          <div className="flex items-center gap-1.5 text-emerald-400 font-medium">
            <span className="text-sm">↓</span>
            <span>{downSpeed}</span>
          </div>
        </button>

        {importProgress && importProgress.status === 'importing' && (
          <div
            className="flex items-center gap-2 px-2.5 py-1 bg-indigo-950/90 border border-indigo-700/70 rounded text-indigo-300 text-[11px] animate-pulse"
            title={`Importing: ${importProgress.archive_name} (${formatSpeed(importProgress.bytes_per_sec)})`}
          >
            <PackageOpen className="w-3.5 h-3.5 text-indigo-400 shrink-0" />
            <span className="font-semibold text-indigo-200">Importing:</span>
            <span className="truncate max-w-[120px]" title={importProgress.archive_name}>
              {importProgress.archive_name}
            </span>
            <span className="font-mono text-indigo-300">
              {formatSpeed(importProgress.bytes_per_sec)}
            </span>
          </div>
        )}
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
