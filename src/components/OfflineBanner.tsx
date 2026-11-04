import { useState } from 'react';
import { WifiOff, RefreshCw } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

interface OfflineBannerProps {
  isOnline: boolean;
}

export function OfflineBanner({ isOnline }: OfflineBannerProps) {
  const [reconnecting, setReconnecting] = useState(false);

  if (isOnline) return null;

  const handleRetry = async () => {
    setReconnecting(true);
    try {
      await invoke('reconnect_swarm');
    } catch (e) {
      console.warn('Reconnect failed:', e);
    } finally {
      setTimeout(() => setReconnecting(false), 1000);
    }
  };

  return (
    <div className="bg-amber-50 border-b border-amber-200 px-4 py-2 flex items-center justify-between text-xs text-amber-900 shadow-2xs">
      <div className="flex items-center gap-2 font-semibold">
        <WifiOff className="w-4 h-4 text-amber-600 shrink-0" />
        <span>📡 Offline — viewing cached files only</span>
      </div>
      <button
        onClick={handleRetry}
        disabled={reconnecting}
        className="flex items-center gap-1.5 px-2.5 py-1 bg-amber-100 hover:bg-amber-200 text-amber-900 border border-amber-300 rounded text-xs font-medium transition cursor-pointer"
      >
        <RefreshCw className={`w-3.5 h-3.5 ${reconnecting ? 'animate-spin text-amber-700' : 'text-amber-700'}`} />
        <span>Retry Connection</span>
      </button>
    </div>
  );
}
