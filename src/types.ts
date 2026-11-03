export interface FileEntry {
  name: string;
  hash: string;
  size_bytes: number;
  is_dir: boolean;
  sync_state: 'cloud_only' | 'cached' | 'pinned' | 'downloading';
  download_progress?: number; // 0-100 when downloading
  owner_node_id?: string;
  last_accessed_at?: number;
  delete_votes?: string[]; // Node IDs who voted to mark this file for deletion
  seeds_count?: number; // Active peer seeds hosting this file
  item_count?: number; // For virtual folders: number of nested items
}

export interface UploadItem {
  id: string;
  name: string;
  size_bytes: number;
  progress: number; // 0-100
  status: 'reading' | 'uploading' | 'completed' | 'error';
  error?: string;
}

export interface StorageStats {
  used_bytes: number;
  quota_bytes: number;
  pinned_bytes: number;
  peer_count: number;
  full_mirror: boolean;
  upload_bytes_per_sec: number;
  download_bytes_per_sec: number;
}

export interface PeerInfo {
  node_id: string;
  relay_url?: string;
  latency_ms?: number;
}
