export interface FileEntry {
  name: string;
  hash: string;
  size_bytes: number;
  is_dir: boolean;
  sync_state: 'cloud_only' | 'cached' | 'pinned' | 'downloading';
  download_progress?: number; // 0-100 when downloading
}

export interface StorageStats {
  used_bytes: number;
  quota_bytes: number;
  pinned_bytes: number;
  peer_count: number;
  full_mirror: boolean;
}

export interface PeerInfo {
  node_id: string;
  relay_url?: string;
  latency_ms?: number;
}

