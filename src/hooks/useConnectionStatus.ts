import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { StorageStats } from '../types';

export function useConnectionStatus() {
  const [isOnline, setIsOnline] = useState<boolean>(true);
  const [peerCount, setPeerCount] = useState<number>(0);

  useEffect(() => {
    // Initial fetch from storage stats
    const fetchInitial = async () => {
      try {
        const stats = await invoke<StorageStats>('get_storage_stats');
        setPeerCount(stats.peer_count);
        setIsOnline(stats.peer_count > 0);
      } catch (err) {
        console.warn('Failed to fetch initial peer count:', err);
      }
    };
    fetchInitial();

    // Listen for real-time connection status events
    let unlisten: (() => void) | undefined;
    listen<{ online: boolean; peer_count: number }>('connection-status-changed', (event) => {
      setIsOnline(event.payload.online);
      setPeerCount(event.payload.peer_count);
    }).then((cleanup) => {
      unlisten = cleanup;
    });

    const interval = setInterval(fetchInitial, 3000);

    return () => {
      if (unlisten) unlisten();
      clearInterval(interval);
    };
  }, []);

  return { isOnline, peerCount };
}

