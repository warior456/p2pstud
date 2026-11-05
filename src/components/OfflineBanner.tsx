import { useState } from 'react';
import { WifiOff, AlertTriangle, RefreshCw } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

interface OfflineBannerProps {
  isOnline: boolean;
  relayConnected?: boolean;
  disconnectReason?: 'no_internet' | 'relay_unreachable' | null;
  relayUrl?: string | null;
  onOpenSettings?: () => void;
}

export function OfflineBanner({
  isOnline,
  disconnectReason,
  relayUrl,
  onOpenSettings,
}: OfflineBannerProps) {
  const [reconnecting, setReconnecting] = useState(false);

  if (isOnline) return null;

  const handleRetry = async () => {
    setReconnecting(true);
    try {
      await invoke('reconnect_swarm');
    } catch (e) {
      console.warn('Reconnect failed:', e);
    } finally {
      setTimeout(() => setReconnecting(false), 1200);
    }
  };

  const isRelayIssue = disconnectReason === 'relay_unreachable';

  return (
    <div
      className={`border-b px-4 py-2.5 flex items-center justify-between text-xs shadow-2xs transition-all ${
        isRelayIssue
          ? 'bg-rose-50 border-rose-200 text-rose-950'
          : 'bg-amber-50 border-amber-200 text-amber-950'
      }`}
    >
      <div className="flex items-center gap-2.5 font-medium min-w-0">
        {isRelayIssue ? (
          <AlertTriangle className="w-4 h-4 text-rose-600 shrink-0" />
        ) : (
          <WifiOff className="w-4 h-4 text-amber-600 shrink-0" />
        )}
        <div className="truncate">
          <span className="font-bold">
            {isRelayIssue
              ? 'P2P Swarm Disconnected (Relay Unreachable)'
              : '📡 Offline — viewing cached files only'}
          </span>
          <span className="ml-1.5 opacity-90 hidden sm:inline">
            {isRelayIssue
              ? `Cannot reach relay server ${relayUrl ? `(${relayUrl})` : ''}. Port 443 may be blocked or network has no route.`
              : 'Viewing cached files only. Reconnecting when network restores.'}
          </span>
        </div>
      </div>
      <div className="flex items-center gap-2 shrink-0">
        {onOpenSettings && (
          <button
            onClick={onOpenSettings}
            className="px-2 py-1 text-xs text-gray-700 hover:text-gray-900 hover:bg-black/5 rounded transition cursor-pointer font-medium"
          >
            Relay Settings
          </button>
        )}
        <button
          onClick={handleRetry}
          disabled={reconnecting}
          className={`flex items-center gap-1.5 px-3 py-1 rounded text-xs font-semibold transition cursor-pointer shadow-2xs ${
            isRelayIssue
              ? 'bg-rose-100 hover:bg-rose-200 text-rose-900 border border-rose-300'
              : 'bg-amber-100 hover:bg-amber-200 text-amber-900 border border-amber-300'
          }`}
        >
          <RefreshCw
            className={`w-3.5 h-3.5 ${
              reconnecting
                ? 'animate-spin ' + (isRelayIssue ? 'text-rose-700' : 'text-amber-700')
                : isRelayIssue
                ? 'text-rose-700'
                : 'text-amber-700'
            }`}
          />
          <span>{reconnecting ? 'Reconnecting...' : 'Retry Connection'}</span>
        </button>
      </div>
    </div>
  );
}
