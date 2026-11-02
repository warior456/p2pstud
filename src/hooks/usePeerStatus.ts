import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { PeerInfo } from '../types';
import { useInterval } from './useInterval';

export function usePeerStatus(intervalMs = 5000) {
  const [peers, setPeers] = useState<PeerInfo[]>([]);
  const [nodeId, setNodeId] = useState<string>('');
  const [error, setError] = useState<string | null>(null);

  const fetchPeers = useCallback(async () => {
    try {
      const data = await invoke<PeerInfo[]>('list_peers');
      setPeers(data);
      setError(null);
    } catch (err: any) {
      setError(err?.toString() || 'Failed to list peers');
    }
  }, []);

  const fetchNodeId = useCallback(async () => {
    try {
      const id = await invoke<string>('get_node_id');
      setNodeId(id);
    } catch (err: any) {
      // Node may still be starting
    }
  }, []);

  useEffect(() => {
    fetchNodeId();
    fetchPeers();
  }, [fetchNodeId, fetchPeers]);

  useInterval(fetchPeers, intervalMs);

  return { peers, nodeId, error, refetch: fetchPeers };
}

