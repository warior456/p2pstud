import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { StorageStats, ConnectionStatus } from '../types';

export function useConnectionStatus() {
  const [isOnline, setIsOnline] = useState<boolean>(
    typeof navigator !== 'undefined' ? navigator.onLine : true
  );
  const [peerCount, setPeerCount] = useState<number>(0);
  const [relayConnected, setRelayConnected] = useState<boolean>(true);
  const [relayUrl, setRelayUrl] = useState<string | null>(null);
  const [disconnectReason, setDisconnectReason] = useState<'no_internet' | 'relay_unreachable' | null>(null);

  useEffect(() => {
    const handleOnline = () => {
      checkStatus();
    };
    const handleOffline = () => {
      setIsOnline(false);
      setDisconnectReason('no_internet');
    };

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    const checkStatus = async () => {
      if (typeof navigator !== 'undefined' && !navigator.onLine) {
        setIsOnline(false);
        setDisconnectReason('no_internet');
        return;
      }

      try {
        const connStatus = await invoke<ConnectionStatus>('get_connection_status');
        setPeerCount(connStatus.peer_count);
        setRelayConnected(connStatus.relay_connected);
        if (connStatus.relay_url) setRelayUrl(connStatus.relay_url);

        if (!connStatus.is_online) {
          setIsOnline(false);
          setDisconnectReason('relay_unreachable');
        } else {
          setIsOnline(true);
          setDisconnectReason(null);
        }
      } catch {
        // Fallback to storage stats if connection status command fails
        try {
          const stats = await invoke<StorageStats>('get_storage_stats');
          setPeerCount(stats.peer_count);
          if (stats.is_online !== undefined) {
            setIsOnline(stats.is_online);
            if (!stats.is_online) setDisconnectReason('relay_unreachable');
          }
        } catch (err) {
          console.warn('Failed to fetch storage stats connection:', err);
        }
      }
    };

    checkStatus();

    // Listen for real-time connection status events from Rust
    let unlisten: (() => void) | undefined;
    listen<{ online: boolean; peer_count: number }>('connection-status-changed', (event) => {
      setIsOnline(event.payload.online);
      setPeerCount(event.payload.peer_count);
      if (!event.payload.online) {
        setDisconnectReason('relay_unreachable');
      } else {
        setDisconnectReason(null);
      }
    }).then((cleanup) => {
      unlisten = cleanup;
    });

    const interval = setInterval(checkStatus, 3000);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
      if (unlisten) unlisten();
      clearInterval(interval);
    };
  }, []);

  return { isOnline, peerCount, relayConnected, relayUrl, disconnectReason };
}
