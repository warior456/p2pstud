import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { StorageStats } from '../types';
import { useInterval } from './useInterval';

export function useStorageStats(intervalMs = 1000) {
  const [stats, setStats] = useState<StorageStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  const fetchStats = useCallback(async () => {
    try {
      const data = await invoke<StorageStats>('get_storage_stats');
      setStats(data);
      setError(null);
    } catch (err: any) {
      setError(err?.toString() || 'Failed to fetch storage stats');
    }
  }, []);

  useEffect(() => {
    fetchStats();
  }, [fetchStats]);

  useInterval(fetchStats, intervalMs);

  return { stats, error, refetch: fetchStats };
}

