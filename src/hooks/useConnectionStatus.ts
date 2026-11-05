import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { StorageStats } from '../types';

export function useConnectionStatus() {
  const [isOnline, setIsOnline] = useState<boolean>(
    typeof navigator !== 'undefined' ? navigator.onLine : true
  );
  const [peerCount, setPeerCount] = useState<number>(0);

  useEffect(() => {
    const handleOnline = () => setIsOnline(true);
    const handleOffline = () => setIsOnline(false);

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    // Initial fetch from storage stats
    const fetchInitial = async () => {
      try {
        const stats = await invoke<StorageStats>('get_storage_stats');
        setPeerCount(stats.peer_count);
        // Do NOT treat 0 peers as offline if network interface is online.
        // Distinguish "searching for peers" from true offline.
        if (typeof navigator !== 'undefined' && !navigator.onLine) {
          setIsOnline(false);
        } else {
          setIsOnline(true);
        }
      } catch (err) {
        console.warn('Failed to fetch initial peer count:', err);
      }
    };
    fetchInitial();

    // Listen for real-time connection status events from Rust
    let unlisten: (() => void) | undefined;
    listen<{ online: boolean; peer_count: number }>('connection-status-changed', (event) => {
      setIsOnline(event.payload.online);
      setPeerCount(event.payload.peer_count);
    }).then((cleanup) => {
      unlisten = cleanup;
    });

    const interval = setInterval(fetchInitial, 3000);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
      if (unlisten) unlisten();
      clearInterval(interval);
    };
  }, []);

  return { isOnline, peerCount };
}

