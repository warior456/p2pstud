import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { PeerInfo } from '../types';
import { useInterval } from './useInterval';

export function usePeerStatus(intervalMs = 3000) {
  const [peers, setPeers] = useState<PeerInfo[]>([]);
  const [nodeId, setNodeId] = useState<string>('');
  const [error, setError] = useState<string | null>(null);

  const fetchNodeId = useCallback(async () => {
    try {
      const id = await invoke<string>('get_node_id');
      if (id) {
        setNodeId(id);
      }
    } catch {
      // Node may still be booting up
    }
  }, []);

  const fetchPeers = useCallback(async () => {
    try {
      const data = await invoke<PeerInfo[]>('list_peers');
      setPeers(data);
      setError(null);
    } catch (err: any) {
      setError(err?.toString() || 'Failed to list peers');
    }
  }, []);

  useEffect(() => {
    fetchNodeId();
    fetchPeers();
  }, [fetchNodeId, fetchPeers]);

  // Keep polling until nodeId is resolved AND periodically refresh peers
  useInterval(() => {
    fetchNodeId();
    fetchPeers();
  }, intervalMs);

  return { peers, nodeId, error, refetch: fetchPeers, refetchNodeId: fetchNodeId };
}
