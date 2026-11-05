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
  keep_votes?: string[]; // Node IDs who voted to KEEP/DEFEND this file against deletion
  deletion_scheduled_at?: number; // Unix timestamp when file will be permanently deleted (grace period)
  seeds_count?: number; // Active peer seeds hosting this file
  item_count?: number; // For virtual folders: number of nested items
  is_quarantined?: boolean;
  quarantine_expires_at?: number | null;
  upvotes?: number;
  downvotes?: number;
  net_rating?: number;
  is_starred?: boolean;
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

export interface AppSettings {
  relay_url: string;
  cache_limit_bytes: number;
  full_mirror: boolean;
  node_id?: string;
  network_secret: string;
  onboarding_complete?: string;
  network_profile_override?: string;
}

export interface NetworkProfileInfo {
  profile: 'campus' | 'trusted' | 'unknown';
  ssid: string | null;
  lecture_throttle_active: boolean;
  override_active: string | null;
  background_cap_bps: number;
}

export interface SsidRule {
  ssid: string;
  profile: 'campus' | 'trusted';
  added_at: number;
}

export interface QuarantineStatus {
  is_quarantined: boolean;
  report_count: number;
}

export interface SoleSeedFile {
  hash: string;
  name: string | null;
  size_bytes: number;
}

export interface TransferQueueItem {
  hash: string;
  name: string | null;
  total_bytes: number;
  received_bytes: number;
  source_peer: string | null;
  created_at: number;
  updated_at: number;
}

export interface ExportPreflightInfo {
  locally_available: number;
  needs_download: number;
  total_size_bytes: number;
}

export interface DoctorTestResult {
  test: string;
  ok: boolean;
  detail: string;
}

export interface ZipImportSummary {
  files_imported: number;
  total_bytes: number;
  errors: string[];
}

export interface ImportProgress {
  import_id?: string;
  archive_name: string;
  current_entry: number;
  total_entries: number;
  bytes_processed: number;
  total_bytes: number;
  current_file: string;
  bytes_per_sec: number;
  status: 'importing' | 'completed' | 'error';
  error?: string | null;
}
