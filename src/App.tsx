import React, { useState, useEffect, useRef, useMemo } from 'react';
import { 
  FolderGit2, 
  Settings, 
  FileCode, 
  FileText, 
  Share2, 
  RefreshCw,
  Search,
  Upload,
  FileArchive,
  Wifi,
  HardDrive,
  Sparkles,
  AlertTriangle,
  X,
  Power
} from 'lucide-react';
import { 
  FileEntry, 
  UploadItem, 
  ZipImportSummary, 
  NetworkProfileInfo, 
  ImportProgress,
  UpdateInfo,
  UpdateDownloadProgress,
  ExportPreflightInfo,
  SoleSeedFile
} from './types';
import { useStorageStats } from './hooks/useStorageStats';
import { usePeerStatus } from './hooks/usePeerStatus';
import { useConnectionStatus } from './hooks/useConnectionStatus';
import { Breadcrumbs } from './components/Explorer/Breadcrumbs';
import { FileTree } from './components/Explorer/FileTree';
import { PdfViewer } from './components/Viewers/PdfViewer';
import { MarkdownEditor } from './components/Viewers/MarkdownEditor';
import { CacheSlider } from './components/Settings/CacheSlider';
import { NetworkSettings } from './components/Settings/NetworkSettings';
import { BackupSettings } from './components/Settings/BackupSettings';
import { UploadQueueDrawer } from './components/Explorer/UploadQueueDrawer';
import { TelemetryBar } from './components/Footer/TelemetryBar';
import { OfflineBanner } from './components/OfflineBanner';
import { SoleSeedBanner } from './components/SoleSeedBanner';
import { ImportingBanner } from './components/ImportingBanner';
import { OnboardingWizard } from './components/Onboarding/OnboardingWizard';
import { ZipImportModal, ZipFileItem } from './components/Explorer/ZipImportModal';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { isSameNodeId, toCanonicalHexNodeId } from './utils/nodeId';

interface SelectedFileInfo {
  path: string;
  name: string;
  size_bytes: number;
}

interface BlobRecord {
  hash: string;
  name: string | null;
  size_bytes: number;
  last_accessed_at: number;
  is_pinned: boolean;
  owner_node_id: string | null;
  delete_votes?: string[];
  keep_votes?: string[];
  deletion_scheduled_at?: number | null;
  seed_count?: number;
  is_cached?: boolean;
  is_quarantined?: boolean;
  quarantine_expires_at?: number | null;
  upvotes?: number;
  downvotes?: number;
  net_rating?: number;
  is_starred?: boolean;
}

export default function App() {
  const [onboardingComplete, setOnboardingComplete] = useState<boolean | null>(null);
  const [paths, setPaths] = useState<string[]>(['Shared Cloud']);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [customFolders, setCustomFolders] = useState<string[]>([]);
  const [uploadQueue, setUploadQueue] = useState<UploadItem[]>([]);
  const [selectedHash, setSelectedHash] = useState<string | null>(null);
  const [currentView, setCurrentView] = useState<'explorer' | 'pdf' | 'markdown' | 'settings'>('explorer');
  const [settingsTab, setSettingsTab] = useState<'cache' | 'network' | 'backup'>('cache');
  const [searchQuery, setSearchQuery] = useState('');
  const [pdfData, setPdfData] = useState<Uint8Array | null>(null);
  const [networkProfile, setNetworkProfile] = useState<NetworkProfileInfo | null>(null);
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  const fileInputRef = useRef<HTMLInputElement>(null);
  const zipInputRef = useRef<HTMLInputElement>(null);
  const [importingZip, setImportingZip] = useState<boolean>(false);
  const [pendingZipFile, setPendingZipFile] = useState<ZipFileItem | null>(null);
  const [showZipModal, setShowZipModal] = useState<boolean>(false);
  const [selectedHashes, setSelectedHashes] = useState<string[]>([]);
  const [importProgress, setImportProgress] = useState<ImportProgress | null>(null);

  // Auto-updater, network classification, and quit modal states
  const [availableUpdate, setAvailableUpdate] = useState<UpdateInfo | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<UpdateDownloadProgress | null>(null);
  const [isUpdatePending, setIsUpdatePending] = useState<boolean>(false);
  const [isUpdatingNow, setIsUpdatingNow] = useState<boolean>(false);
  const [dismissedNetworkPrompt, setDismissedNetworkPrompt] = useState<boolean>(false);
  const [showQuitModal, setShowQuitModal] = useState<boolean>(false);
  const [soleSeedsOnQuit, setSoleSeedsOnQuit] = useState<any[]>([]);

  const { stats, refetch: refetchStats } = useStorageStats();
  const { nodeId } = usePeerStatus();
  const { isOnline, relayConnected, relayUrl, disconnectReason } = useConnectionStatus();

  // Listen for backend import progress events
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<ImportProgress>('import-progress', (event) => {
      setImportProgress(event.payload);
      if (event.payload.status === 'completed') {
        loadWorkspaceFiles();
        loadWorkspaceFolders();
        refetchStats();
        setTimeout(() => {
          setImportProgress((curr) => (curr?.status === 'completed' ? null : curr));
        }, 3500);
      }
    })
      .then((unsub) => {
        unlisten = unsub;
      })
      .catch((err) => {
        console.warn('Could not register import-progress listener:', err);
      });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // Check onboarding & pending update status on startup
  useEffect(() => {
    invoke<string | null>('get_setting', { key: 'onboarding_complete' })
      .then((val) => {
        setOnboardingComplete(val === 'true');
      })
      .catch((err) => {
        console.warn('Could not read onboarding status:', err);
        setOnboardingComplete(true); // default to true on error so app stays usable
      });

    invoke<string | null>('get_setting', { key: 'update_pending' })
      .then((val) => {
        if (val === 'true') setIsUpdatePending(true);
      })
      .catch(() => {});
  }, []);

  // Fetch network profile
  const refreshNetworkProfile = async () => {
    try {
      const profile = await invoke<NetworkProfileInfo>('get_network_profile');
      setNetworkProfile(profile);
    } catch (err) {
      console.warn('Failed to load network profile:', err);
    }
  };

  useEffect(() => {
    refreshNetworkProfile();
    // Low-frequency safety poll (60s) and immediate refresh on window focus
    const interval = setInterval(refreshNetworkProfile, 60000);
    const onFocus = () => refreshNetworkProfile();
    window.addEventListener('focus', onFocus);
    return () => {
      clearInterval(interval);
      window.removeEventListener('focus', onFocus);
    };
  }, []);

  // Listen to background Tauri events (network profile change events, new SSIDs, scarcity replication, quarantine)
  useEffect(() => {
    let unlistenProfile: (() => void) | undefined;
    let unlistenSsid: (() => void) | undefined;
    let unlistenScarcity: (() => void) | undefined;
    let unlistenQuarantine: (() => void) | undefined;

    listen<NetworkProfileInfo>('network-profile-changed', (e) => {
      setNetworkProfile(e.payload);
    }).then((un) => {
      unlistenProfile = un;
    });

    listen<{ ssid: string }>('network-new-ssid', (e) => {
      refreshNetworkProfile();
      setToastMessage(`Connected to new network: ${e.payload.ssid}`);
      setTimeout(() => setToastMessage(null), 5000);
    }).then((un) => {
      unlistenSsid = un;
    });

    listen<{ hash: string }>('scarcity-replicated', () => {
      setToastMessage(`Defended rare file! Cached endangered sole-seed file.`);
      setTimeout(() => setToastMessage(null), 5000);
      loadWorkspaceFiles();
      refetchStats();
    }).then((un) => {
      unlistenScarcity = un;
    });

    listen<{ hash: string; reason: string }>('file-quarantined', (e) => {
      setToastMessage(`Security notice: File ${e.payload.hash.slice(0, 8)}... was placed on community quarantine hold.`);
      setTimeout(() => setToastMessage(null), 7000);
      loadWorkspaceFiles();
    }).then((un) => {
      unlistenQuarantine = un;
    });

    let unlistenFolderCreated: (() => void) | undefined;
    let unlistenFolderDeleted: (() => void) | undefined;

    listen<string>('folder-created', (e) => {
      setCustomFolders((prev) => (prev.includes(e.payload) ? prev : [...prev, e.payload]));
    }).then((un) => {
      unlistenFolderCreated = un;
    });

    listen<string>('folder-deleted', (e) => {
      setCustomFolders((prev) => prev.filter((f) => f !== e.payload && !f.startsWith(`${e.payload}/`)));
    }).then((un) => {
      unlistenFolderDeleted = un;
    });

    let unlistenUpdate: (() => void) | undefined;
    let unlistenUpdateDownloaded: (() => void) | undefined;
    let unlistenDownloadProgress: (() => void) | undefined;
    let unlistenQuitRequested: (() => void) | undefined;
    let unlistenDbSwitched: (() => void) | undefined;

    listen('network-database-switched', () => {
      loadWorkspaceFiles();
      loadWorkspaceFolders();
      refetchStats();
      refreshNetworkProfile();
      setToastMessage('Switched active network database');
      setTimeout(() => setToastMessage(null), 4000);
    }).then((un) => {
      unlistenDbSwitched = un;
    });

    listen<UpdateInfo>('update-available', (e) => {
      setAvailableUpdate(e.payload);
    }).then((un) => {
      unlistenUpdate = un;
    });

    listen<UpdateDownloadProgress>('update-download-progress', (e) => {
      setDownloadProgress(e.payload);
    }).then((un) => {
      unlistenDownloadProgress = un;
    });

    listen('update-downloaded', () => {
      setIsUpdatePending(true);
      setDownloadProgress(null);
    }).then((un) => {
      unlistenUpdateDownloaded = un;
    });

    listen('app-quit-requested', async () => {
      try {
        const soleSeeds = await invoke<any[]>('get_sole_seed_files');
        const pending = await invoke<string | null>('get_setting', { key: 'update_pending' });
        const isPending = pending === 'true';

        if ((!soleSeeds || soleSeeds.length === 0) && !isPending) {
          // Not the only one seeding and no update pending: cleanly quit
          await invoke('confirm_app_exit', { installUpdate: false });
        } else {
          // Only one seeding or update pending: open warning popup
          setSoleSeedsOnQuit(soleSeeds || []);
          setIsUpdatePending(isPending);
          setShowQuitModal(true);
        }
      } catch (_) {
        setShowQuitModal(true);
      }
    }).then((un) => {
      unlistenQuitRequested = un;
    });

    return () => {
      if (unlistenProfile) unlistenProfile();
      if (unlistenSsid) unlistenSsid();
      if (unlistenScarcity) unlistenScarcity();
      if (unlistenQuarantine) unlistenQuarantine();
      if (unlistenFolderCreated) unlistenFolderCreated();
      if (unlistenFolderDeleted) unlistenFolderDeleted();
      if (unlistenUpdate) unlistenUpdate();
      if (unlistenUpdateDownloaded) unlistenUpdateDownloaded();
      if (unlistenDownloadProgress) unlistenDownloadProgress();
      if (unlistenQuitRequested) unlistenQuitRequested();
      if (unlistenDbSwitched) unlistenDbSwitched();
    };
  }, []);

  const loadWorkspaceFiles = async () => {
    try {
      const records = await invoke<BlobRecord[]>('list_files');
      if (records) {
        setFiles(
          records.map((r) => ({
            name: r.name || `Document-${r.hash.slice(0, 8)}.bin`,
            hash: r.hash,
            size_bytes: r.size_bytes,
            is_dir: false,
            sync_state: r.is_pinned ? 'pinned' : r.is_cached ? 'cached' : 'cloud_only',
            owner_node_id: r.owner_node_id ? (toCanonicalHexNodeId(r.owner_node_id) || r.owner_node_id) : undefined,
            last_accessed_at: r.last_accessed_at,
            delete_votes: r.delete_votes || [],
            keep_votes: r.keep_votes || [],
            deletion_scheduled_at: r.deletion_scheduled_at ?? undefined,
            seeds_count: r.seed_count ?? 1,
            is_quarantined: r.is_quarantined ?? false,
            quarantine_expires_at: r.quarantine_expires_at ?? null,
            upvotes: r.upvotes ?? 0,
            downvotes: r.downvotes ?? 0,
            net_rating: r.net_rating ?? 0,
            is_starred: r.is_starred ?? false,
          }))
        );
      }
    } catch (err) {
      console.warn('Failed to fetch workspace files:', err);
    }
  };

  const loadWorkspaceFolders = async () => {
    try {
      const folders = await invoke<string[]>('list_folders');
      if (folders && Array.isArray(folders)) {
        setCustomFolders(folders);
      }
    } catch (err) {
      console.warn('Failed to fetch workspace folders:', err);
    }
  };

  useEffect(() => {
    loadWorkspaceFiles();
    loadWorkspaceFolders();
    const interval = setInterval(() => {
      loadWorkspaceFiles();
      loadWorkspaceFolders();
    }, 2000);
    return () => clearInterval(interval);
  }, []);

  const handleTriggerUpload = async () => {
    try {
      const selected = await invoke<SelectedFileInfo[] | null>('pick_files_dialog');
      if (selected && selected.length > 0) {
        await handleUploadSelectedFiles(selected);
        return;
      }
    } catch (err) {
      console.warn('Native file picker dialog error, falling back to input:', err);
    }
    fileInputRef.current?.click();
  };

  const handleUploadSelectedFiles = async (selected: SelectedFileInfo[]) => {
    if (!selected || selected.length === 0) return;

    const newItems: UploadItem[] = selected.map((f) => ({
      id: `${f.name}-${Date.now()}-${Math.random()}`,
      name: currentPrefix ? `${currentPrefix}${f.name}` : f.name,
      size_bytes: f.size_bytes,
      progress: 10,
      status: 'reading',
    }));

    setUploadQueue((prev) => [...prev, ...newItems]);

    for (let i = 0; i < selected.length; i++) {
      const f = selected[i];
      const item = newItems[i];
      const targetName = item.name;

      if (f.name.toLowerCase().endsWith('.zip')) {
        setPendingZipFile({
          name: f.name,
          size_bytes: f.size_bytes,
          path: f.path,
        });
        setShowZipModal(true);
        continue;
      }

      try {
        setUploadQueue((prev) =>
          prev.map((q) => (q.id === item.id ? { ...q, progress: 65, status: 'uploading' } : q))
        );
        await invoke('upload_file_path', { filePath: f.path, targetName });
        setUploadQueue((prev) =>
          prev.map((q) => (q.id === item.id ? { ...q, progress: 100, status: 'completed' } : q))
        );
      } catch (err) {
        console.error('Failed to upload file by path:', err);
        setUploadQueue((prev) =>
          prev.map((q) =>
            q.id === item.id ? { ...q, progress: 100, status: 'error', error: String(err) } : q
          )
        );
      }
    }

    await loadWorkspaceFiles();
    refetchStats();
  };

  const currentPrefix = paths.length > 1 ? paths.slice(1).join('/') + '/' : '';

  const handleUploadFiles = async (selectedFiles: FileList | File[]) => {
    if (!selectedFiles || selectedFiles.length === 0) return;

    const fileList = Array.from(selectedFiles);
    const newItems: UploadItem[] = fileList.map((f) => ({
      id: `${f.name}-${Date.now()}-${Math.random()}`,
      name: currentPrefix ? `${currentPrefix}${f.name}` : f.name,
      size_bytes: f.size,
      progress: 10,
      status: 'reading',
    }));

    setUploadQueue((prev) => [...prev, ...newItems]);

    for (let i = 0; i < fileList.length; i++) {
      const file = fileList[i];
      const item = newItems[i];
      const targetName = item.name;

      // If user uploaded a ZIP file, prompt with security & extraction warning modal
      if (file.name.toLowerCase().endsWith('.zip')) {
        const filePath = (file as any).path as string | undefined;
        let data: Uint8Array | undefined = undefined;
        if (!filePath) {
          try {
            const ab = await file.arrayBuffer();
            data = new Uint8Array(ab);
          } catch (err) {
            console.warn('Could not read zip buffer in advance:', err);
          }
        }
        setPendingZipFile({
          name: file.name,
          size_bytes: file.size,
          path: filePath,
          file,
          data,
        });
        setShowZipModal(true);
        continue;
      }

      try {
        setUploadQueue((prev) =>
          prev.map((q) => (q.id === item.id ? { ...q, progress: 30, status: 'reading' } : q))
        );
        const filePath = (file as any).path as string | undefined;
        if (filePath) {
          setUploadQueue((prev) =>
            prev.map((q) => (q.id === item.id ? { ...q, progress: 65, status: 'uploading' } : q))
          );
          await invoke('upload_file_path', { filePath, targetName });
        } else {
          const buffer = await file.arrayBuffer();
          const bytes = Array.from(new Uint8Array(buffer));
          setUploadQueue((prev) =>
            prev.map((q) => (q.id === item.id ? { ...q, progress: 65, status: 'uploading' } : q))
          );
          await invoke('upload_file_bytes', { name: targetName, data: bytes });
        }

        setUploadQueue((prev) =>
          prev.map((q) => (q.id === item.id ? { ...q, progress: 100, status: 'completed' } : q))
        );
      } catch (err) {
        console.error('Failed to upload file:', err);
        setUploadQueue((prev) =>
          prev.map((q) =>
            q.id === item.id ? { ...q, progress: 100, status: 'error', error: String(err) } : q
          )
        );
      }
    }

    if (fileInputRef.current && !fileList.some((f) => f.name.toLowerCase().endsWith('.zip'))) {
      fileInputRef.current.value = '';
    }

    await loadWorkspaceFiles();
    refetchStats();
  };

  const handleFileInputChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files.length > 0) {
      await handleUploadFiles(e.target.files);
    }
  };

  const handleTriggerZipImport = async () => {
    try {
      const selected = await invoke<SelectedFileInfo | null>('pick_zip_dialog');
      if (selected) {
        setPendingZipFile({
          name: selected.name,
          size_bytes: selected.size_bytes,
          path: selected.path,
        });
        setShowZipModal(true);
      }
    } catch (err) {
      console.warn('Native ZIP dialog error, falling back to input:', err);
      zipInputRef.current?.click();
    }
  };

  const handleZipInputChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const filePath = (file as any).path as string | undefined;
    let data: Uint8Array | undefined = undefined;
    if (!filePath) {
      try {
        const ab = await file.arrayBuffer();
        data = new Uint8Array(ab);
      } catch (err) {
        console.warn('Could not read zip buffer in handleZipInputChange:', err);
      }
    }
    setPendingZipFile({
      name: file.name,
      size_bytes: file.size,
      path: filePath,
      file,
      data,
    });
    setShowZipModal(true);
  };

  const handleExecuteZipUnpack = async () => {
    if (!pendingZipFile) return;
    const target = pendingZipFile;
    setShowZipModal(false);
    setImportingZip(true);
    setImportProgress({
      archive_name: target.name,
      current_entry: 0,
      total_entries: 0,
      bytes_processed: 0,
      total_bytes: target.size_bytes || 0,
      current_file: 'Reading archive entries...',
      bytes_per_sec: 0,
      status: 'importing',
    });

    try {
      const targetFolder = currentPrefix.replace(/\/$/, '') || null;
      let summary: ZipImportSummary;

      const resolvedPath = target.path || (target.file ? (target.file as any).path : undefined);
      if (resolvedPath) {
        // High-performance streaming disk import for multi-GB ZIPs (BufReader, zero webview RAM)
        summary = await invoke<ZipImportSummary>('import_zip_file', {
          filePath: resolvedPath,
          targetFolder,
        });
      } else if (target.data) {
        const bytes = Array.from(target.data);
        summary = await invoke<ZipImportSummary>('import_zip_bytes', {
          data: bytes,
          targetFolder,
        });
      } else if (target.file) {
        const buffer = await target.file.arrayBuffer();
        const bytes = Array.from(new Uint8Array(buffer));
        summary = await invoke<ZipImportSummary>('import_zip_bytes', {
          data: bytes,
          targetFolder,
        });
      } else {
        throw new Error('No valid ZIP file target provided');
      }

      const mb = (summary.total_bytes / (1024 * 1024)).toFixed(2);
      let msg = `Successfully imported ${summary.files_imported} file(s) (${mb} MB) from ${target.name}.`;
      if (summary.errors && summary.errors.length > 0) {
        msg += `\n\n${summary.errors.length} file(s) skipped:\n` + summary.errors.slice(0, 3).join('\n');
      }
      setToastMessage(msg);
      setImportProgress({
        archive_name: target.name,
        current_entry: summary.files_imported,
        total_entries: summary.files_imported,
        bytes_processed: summary.total_bytes,
        total_bytes: summary.total_bytes,
        current_file: 'Completed',
        bytes_per_sec: 0,
        status: 'completed',
      });
      await loadWorkspaceFiles();
      await loadWorkspaceFolders();
      refetchStats();
      setTimeout(() => {
        setImportProgress((curr) => (curr?.status === 'completed' ? null : curr));
      }, 3500);
    } catch (err) {
      setImportProgress({
        archive_name: target.name,
        current_entry: 0,
        total_entries: 0,
        bytes_processed: 0,
        total_bytes: 0,
        current_file: '',
        bytes_per_sec: 0,
        status: 'error',
        error: String(err),
      });
      setTimeout(() => {
        setImportProgress((curr) => (curr?.status === 'error' ? null : curr));
      }, 5000);
      alert(`ZIP import failed: ${err}`);
    } finally {
      setImportingZip(false);
      setPendingZipFile(null);
      if (zipInputRef.current) {
        zipInputRef.current.value = '';
      }
      if (fileInputRef.current) {
        fileInputRef.current.value = '';
      }
    }
  };

  const handleExecuteUploadAsZip = async () => {
    if (!pendingZipFile) return;
    const target = pendingZipFile;
    setShowZipModal(false);
    const targetName = currentPrefix ? `${currentPrefix}${target.name}` : target.name;
    const resolvedPath = target.path || (target.file ? (target.file as any).path : undefined);
    try {
      if (resolvedPath) {
        await invoke('upload_file_path', { filePath: resolvedPath, targetName });
      } else if (target.data) {
        const bytes = Array.from(target.data);
        await invoke('upload_file_bytes', { name: targetName, data: bytes });
      } else if (target.file) {
        const buffer = await target.file.arrayBuffer();
        const bytes = Array.from(new Uint8Array(buffer));
        await invoke('upload_file_bytes', { name: targetName, data: bytes });
      }
      setToastMessage(`Uploaded archive "${target.name}" as single file.`);
      setImportProgress({
        archive_name: target.name,
        current_entry: 1,
        total_entries: 1,
        bytes_processed: target.size_bytes || 0,
        total_bytes: target.size_bytes || 0,
        current_file: 'Completed',
        bytes_per_sec: 0,
        status: 'completed',
      });
      await loadWorkspaceFiles();
      refetchStats();
      setTimeout(() => {
        setImportProgress((curr) => (curr?.status === 'completed' ? null : curr));
      }, 3500);
    } catch (err) {
      alert(`Upload failed: ${err}`);
    } finally {
      setPendingZipFile(null);
      if (zipInputRef.current) {
        zipInputRef.current.value = '';
      }
      if (fileInputRef.current) {
        fileInputRef.current.value = '';
      }
    }
  };

  const handleExportFolderZip = async (folderNameOrPath: string) => {
    try {
      const virtualPath = currentPrefix ? `${currentPrefix}${folderNameOrPath}` : folderNameOrPath;
      const preflight = await invoke<ExportPreflightInfo>('export_folder_zip_preflight', {
        virtualPath,
      });

      if (preflight.locally_available === 0 && preflight.needs_download === 0) {
        alert(`Folder "${folderNameOrPath}" contains no files to export.`);
        return;
      }

      if (preflight.needs_download > 0) {
        const total = preflight.locally_available + preflight.needs_download;
        const proceed = window.confirm(
          `Export folder "${folderNameOrPath}" (${total} files total):\n\n` +
          `• ${preflight.locally_available} file(s) are cached locally.\n` +
          `• ${preflight.needs_download} file(s) must be downloaded from swarm peers.\n\n` +
          `Do you want to proceed with downloading and exporting?`
        );
        if (!proceed) return;
      }

      const defaultName = `${folderNameOrPath.replace(/[\/\\]/g, '_')}_export.zip`;
      const savePath = await invoke<string | null>('pick_folder_export_save_dialog', {
        defaultName,
      });

      if (!savePath) return;

      setToastMessage(`Exporting "${folderNameOrPath}" to ZIP archive...`);
      await invoke('export_folder_zip', {
        virtualPath,
        destPath: savePath,
      });
      setToastMessage(`Successfully exported folder to:\n${savePath}`);
    } catch (err) {
      console.error('Failed to export folder ZIP:', err);
      alert(`Folder export failed: ${err}`);
    }
  };

  const handleClassifyNetwork = async (ssid: string, profile: 'campus' | 'trusted') => {
    try {
      await invoke('add_ssid_rule', { ssid, profile });
      await refreshNetworkProfile();
      setToastMessage(`Network "${ssid}" classified as ${profile === 'campus' ? 'Campus Stealth' : 'Trusted'}.`);
    } catch (err) {
      alert(`Failed to save network rule: ${err}`);
    }
  };

  const handleApplyUpdateOnRestart = async () => {
    try {
      setToastMessage('Downloading update package...');
      await invoke('download_app_update');
      setIsUpdatePending(true);
      setAvailableUpdate(null);
      setDownloadProgress(null);
      setToastMessage('Update downloaded! It will install when you restart or update.');
    } catch (err) {
      setDownloadProgress(null);
      alert(`Failed to stage update: ${err}`);
    }
  };

  const handleUpdateAndRestartNow = async () => {
    try {
      setIsUpdatingNow(true);
      setToastMessage('Downloading update package...');
      await invoke('download_app_update');
      setToastMessage('Download verified! Applying update...');
      setIsUpdatePending(true);
      setDownloadProgress(null);
      await invoke('confirm_app_exit', { installUpdate: true });
    } catch (err) {
      setIsUpdatingNow(false);
      setDownloadProgress(null);
      alert(`Failed to apply update: ${err}`);
    }
  };

  const handleDismissUpdateToast = async () => {
    setAvailableUpdate(null);
    try {
      await invoke('dismiss_app_update');
    } catch (_) {}
  };

  const handleRequestQuit = async () => {
    try {
      const soleSeeds = await invoke<any[]>('get_sole_seed_files');
      const pending = await invoke<string | null>('get_setting', { key: 'update_pending' });
      const isPending = pending === 'true';

      if ((!soleSeeds || soleSeeds.length === 0) && !isPending) {
        // Not sole seeder: cleanly exit immediately
        await invoke('confirm_app_exit', { installUpdate: false });
      } else {
        // Only one seeding: display warning confirmation popup
        setSoleSeedsOnQuit(soleSeeds || []);
        setIsUpdatePending(isPending);
        setShowQuitModal(true);
      }
    } catch (_) {
      await invoke('confirm_app_exit', { installUpdate: false });
    }
  };

  const handleMinimizeToTray = async () => {
    setShowQuitModal(false);
    try {
      await invoke('hide_main_window');
    } catch (_) {}
  };

  const handleConfirmExit = async (installUpdate: boolean) => {
    setShowQuitModal(false);
    try {
      await invoke('confirm_app_exit', { installUpdate });
    } catch (_) {}
  };

  const handleCancelZipModal = () => {
    setShowZipModal(false);
    setPendingZipFile(null);
    if (zipInputRef.current) {
      zipInputRef.current.value = '';
    }
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const handleCreateFolder = async () => {
    const folderName = prompt('Enter folder name:');
    if (!folderName || !folderName.trim()) return;
    const cleanName = folderName.trim().replace(/[\/\\]/g, '-');
    const fullFolderPath = currentPrefix ? `${currentPrefix}${cleanName}` : cleanName;

    try {
      await invoke('create_folder', { folderPath: fullFolderPath });
      setCustomFolders((prev) => (prev.includes(fullFolderPath) ? prev : [...prev, fullFolderPath]));
      await loadWorkspaceFiles();
    } catch (err) {
      console.error('Failed to create folder:', err);
      alert(`Failed to create folder: ${err}`);
    }
  };

  const handleDeleteFolder = async (folderName: string) => {
    const targetFolder = currentPrefix ? `${currentPrefix}${folderName}` : folderName;
    if (!confirm(`Are you sure you want to delete folder "${folderName}"?`)) return;

    try {
      await invoke('delete_folder', { folderPath: targetFolder });
      setCustomFolders((prev) => prev.filter((f) => f !== targetFolder && !f.startsWith(`${targetFolder}/`)));
      if (paths.length > 1 && paths[paths.length - 1] === folderName) {
        setPaths((prev) => prev.slice(0, -1));
      }
      await loadWorkspaceFiles();
    } catch (err) {
      alert(`Failed to delete folder: ${err}`);
    }
  };

  const handleNavigateFolder = (folderName: string) => {
    setPaths((prev) => [...prev, folderName]);
    setSelectedHash(null);
  };

  const handleNavigateBreadcrumbs = (index: number) => {
    if (index === -1) {
      setPaths(['Shared Cloud']);
    } else {
      setPaths(paths.slice(0, index + 1));
    }
    setSelectedHash(null);
    setCurrentView('explorer');
  };

  const handleNavigateUp = () => {
    if (paths.length > 1) {
      setPaths((prev) => prev.slice(0, -1));
      setSelectedHash(null);
    }
  };

  const handleDismissUpload = (id: string) => {
    setUploadQueue((prev) => prev.filter((item) => item.id !== id));
  };

  const handleClearUploadQueue = () => {
    setUploadQueue([]);
  };

  const handleToggleSelectHash = (hash: string) => {
    setSelectedHashes((prev) =>
      prev.includes(hash) ? prev.filter((h) => h !== hash) : [...prev, hash]
    );
  };

  const handleClearSelection = () => {
    setSelectedHashes([]);
  };

  const handleBatchDelete = async (hashes: string[]) => {
    if (!hashes || hashes.length === 0) return;
    const selectedList = files.filter((f) => hashes.includes(f.hash));
    const owned = selectedList.filter((f) => isSameNodeId(f.owner_node_id, nodeId));
    const community = selectedList.filter((f) => !isSameNodeId(f.owner_node_id, nodeId));

    let confirmMsg = '';
    if (owned.length > 0 && community.length > 0) {
      confirmMsg = `You selected ${hashes.length} file(s):\n- ${owned.length} owned file(s) will be PERMANENTLY DELETED.\n- ${community.length} community file(s) will receive your DELETE VOTE.\n\nProceed?`;
    } else if (owned.length > 0) {
      confirmMsg = `Are you sure you want to delete ${owned.length} file(s) owned by you?\n\nThis will remove them from the workspace.`;
    } else {
      confirmMsg = `Vote to mark ${community.length} file(s) for deletion?\n\nThis records your vote for community consensus.`;
    }

    if (!confirm(confirmMsg)) return;

    try {
      const deleted = await invoke<string[]>('batch_delete_files', { hashes });
      setToastMessage(`Processed deletion/votes for ${deleted.length} file(s).`);
      setSelectedHashes([]);
      await loadWorkspaceFiles();
      refetchStats();
    } catch (err) {
      alert(`Batch delete error: ${err}`);
    }
  };

  const handleBatchMove = async (hashes: string[]) => {
    if (!hashes || hashes.length === 0) return;
    const knownFolders = new Set<string>();
    for (const f of files) {
      if (f.name.includes('/')) {
        const parts = f.name.split('/');
        parts.pop();
        let acc = '';
        for (const p of parts) {
          acc = acc ? `${acc}/${p}` : p;
          knownFolders.add(acc);
        }
      }
    }
    for (const cf of customFolders) {
      knownFolders.add(cf);
    }

    const folderList = Array.from(knownFolders).sort();
    const folderHelp = folderList.length > 0 
      ? `\n\nAvailable folders:\n- (leave empty for Root)\n` + folderList.map((f) => `- ${f}`).join('\n')
      : '\n\nEnter destination folder name (or leave empty for Root)';

    const destination = prompt(
      `Move ${hashes.length} selected file(s) to folder:${folderHelp}`,
      currentPrefix.replace(/\/$/, '')
    );

    if (destination === null) return;

    try {
      const moved = await invoke<string[]>('batch_move_files', {
        hashes,
        targetFolder: destination.trim(),
      });
      setToastMessage(`Moved ${moved.length} file(s) to "${destination.trim() || 'Root'}".`);
      setSelectedHashes([]);
      await loadWorkspaceFiles();
      await loadWorkspaceFolders();
    } catch (err) {
      alert(`Batch move error: ${err}`);
    }
  };

  const handleBatchDownload = async (hashes: string[]) => {
    if (!hashes || hashes.length === 0) return;
    setToastMessage(`Starting download of ${hashes.length} file(s)...`);
    for (const hash of hashes) {
      await handleDownload(hash);
    }
  };

  const handleDelete = async (hash: string) => {
    const file = files.find((f) => f.hash === hash);
    const fileName = file?.name || hash.slice(0, 8);
    if (!confirm(`Are you sure you want to delete your file "${fileName}"?\n\nThis will remove it from the workspace.`)) {
      return;
    }

    try {
      await invoke('delete_file', { hash });
      setFiles((prev) => prev.filter((f) => f.hash !== hash));
      refetchStats();
    } catch (err) {
      console.error('Delete error:', err);
      alert(`Delete failed: ${err}`);
    }
  };

  const handleVoteDelete = async (hash: string) => {
    const file = files.find((f) => f.hash === hash);
    const fileName = file?.name || hash.slice(0, 8);
    if (
      !confirm(
        `Vote to mark "${fileName}" for deletion?\n\nThis records your vote. Deletion requires at least 3 delete votes and an 86% community consensus ratio. Once reached, a 24-hour grace period allows peers to defend/keep the file before permanent removal.`
      )
    ) {
      return;
    }

    try {
      await invoke<number>('vote_delete_file', { hash });
      await loadWorkspaceFiles();
      refetchStats();
    } catch (err) {
      console.error('Vote delete error:', err);
      alert(`Vote failed: ${err}`);
    }
  };

  const handleUnvoteDelete = async (hash: string) => {
    try {
      await invoke<number>('unvote_delete_file', { hash });
      await loadWorkspaceFiles();
      refetchStats();
    } catch (err) {
      console.error('Unvote error:', err);
      alert(`Unvote failed: ${err}`);
    }
  };

  const handleVoteKeep = async (hash: string) => {
    try {
      await invoke<number>('vote_keep_file', { hash });
      await loadWorkspaceFiles();
      refetchStats();
    } catch (err) {
      console.error('Vote keep error:', err);
      alert(`Vote to keep failed: ${err}`);
    }
  };

  const handleUnvoteKeep = async (hash: string) => {
    try {
      await invoke<number>('unvote_keep_file', { hash });
      await loadWorkspaceFiles();
      refetchStats();
    } catch (err) {
      console.error('Unvote keep error:', err);
      alert(`Unvote keep failed: ${err}`);
    }
  };

  const handleEvictLocal = async (hash: string) => {
    const file = files.find((f) => f.hash === hash);
    if (file && (file.seeds_count ?? 1) <= 1) {
      alert(
        `Cannot remove "${file.name}" from local disk: You are currently the ONLY active seed in the swarm!\n\nIf you delete your copy, this file will be permanently lost for all students. Wait until at least one other peer caches it, or vote to delete the file if it is no longer needed.`
      );
      return;
    }

    try {
      await invoke('evict_local_file', { hash });
      setFiles((prev) =>
        prev.map((f) => (f.hash === hash ? { ...f, sync_state: 'cloud_only' } : f))
      );
      refetchStats();
    } catch (err) {
      alert(`Failed to free local space: ${err}`);
    }
  };

  const handleMoveFile = async (hash: string, targetFolderPath: string) => {
    const file = files.find((f) => f.hash === hash);
    if (!file) return;

    const basename = file.name.split('/').pop() || file.name;
    const cleanFolder = targetFolderPath.trim().replace(/^\/+|\/+$/g, '');
    const newPath = cleanFolder ? `${cleanFolder}/${basename}` : basename;

    if (newPath === file.name) return;

    try {
      await invoke('move_file', { hash, newPath });
      await loadWorkspaceFiles();
    } catch (err) {
      console.error('Move file error:', err);
      alert(`Failed to move file: ${err}`);
    }
  };

  const handlePromptMove = async (hash: string) => {
    const file = files.find((f) => f.hash === hash);
    if (!file) return;

    const knownFolders = new Set<string>();
    for (const f of files) {
      if (f.name.includes('/')) {
        const parts = f.name.split('/');
        parts.pop();
        let acc = '';
        for (const p of parts) {
          acc = acc ? `${acc}/${p}` : p;
          knownFolders.add(acc);
        }
      }
    }
    for (const cf of customFolders) {
      knownFolders.add(cf);
    }

    const folderList = Array.from(knownFolders).sort();
    const folderHelp = folderList.length > 0 
      ? `\n\nAvailable folders:\n- (leave empty for Root)\n` + folderList.map((f) => `- ${f}`).join('\n')
      : '\n\nEnter folder name (or leave empty for Root)';

    const destination = prompt(
      `Move "${file.name}" to folder:${folderHelp}`,
      currentPrefix.replace(/\/$/, '')
    );

    if (destination === null) return;
    await handleMoveFile(hash, destination.trim());
  };

  const handleMoveToFolder = async (sourceHash: string, targetFolderName: string) => {
    const targetPath = currentPrefix ? `${currentPrefix}${targetFolderName}` : targetFolderName;
    await handleMoveFile(sourceHash, targetPath);
  };

  const handleSelectFile = async (file: FileEntry) => {
    setSelectedHash(file.hash);
    if (file.name.endsWith('.pdf')) {
      setCurrentView('pdf');
      try {
        const bytes = await invoke<number[]>('read_file_bytes', { hash: file.hash });
        if (bytes && bytes.length > 0) {
          setPdfData(new Uint8Array(bytes));
        } else {
          setPdfData(null);
        }
      } catch {
        setPdfData(null);
      }
    } else if (file.name.endsWith('.md') || file.name.endsWith('.txt')) {
      setCurrentView('markdown');
    }
  };

  const handlePin = async (hash: string) => {
    try {
      await invoke('pin_file', { hash });
      setFiles((prev) =>
        prev.map((f) => (f.hash === hash ? { ...f, sync_state: 'pinned' } : f))
      );
      refetchStats();
    } catch (err) {
      console.error(err);
    }
  };

  const handleUnpin = async (hash: string) => {
    try {
      await invoke('unpin_file', { hash });
      setFiles((prev) =>
        prev.map((f) => (f.hash === hash ? { ...f, sync_state: 'cached' } : f))
      );
      refetchStats();
    } catch (err) {
      console.error(err);
    }
  };

  const handleDownload = async (hash: string) => {
    const file = files.find((f) => f.hash === hash);
    if (!file) return;

    setFiles((prev) =>
      prev.map((f) =>
        f.hash === hash ? { ...f, sync_state: 'downloading', download_progress: 25 } : f
      )
    );

    try {
      const bytes = await invoke<number[]>('read_file_bytes', { hash });
      if (bytes && bytes.length > 0) {
        const blob = new Blob([new Uint8Array(bytes)], { type: 'application/octet-stream' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = file.name;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
      }

      setFiles((prev) =>
        prev.map((f) => (f.hash === hash ? { ...f, sync_state: 'cached' } : f))
      );
      refetchStats();
    } catch (err) {
      console.error('Download error:', err);
      alert(`Download failed: ${err}`);
      setFiles((prev) =>
        prev.map((f) => (f.hash === hash ? { ...f, sync_state: 'cloud_only' } : f))
      );
    }
  };

  const handleRate = async (hash: string, rating: 1 | -1) => {
    try {
      await invoke('rate_file', { hash, rating });
      await loadWorkspaceFiles();
    } catch (err) {
      console.warn('Failed to rate file:', err);
    }
  };

  const handleToggleStar = async (hash: string) => {
    try {
      await invoke('toggle_star', { hash });
      await loadWorkspaceFiles();
    } catch (err) {
      console.warn('Failed to toggle star:', err);
    }
  };

  const handleReportDangerous = async (hash: string, reason: string) => {
    try {
      await invoke('report_dangerous', { hash, reason });
      alert(`Security report submitted for file. Thank you for protecting the swarm!`);
      await loadWorkspaceFiles();
    } catch (err) {
      alert(`Failed to submit report: ${err}`);
    }
  };

  const currentDisplayFiles = useMemo(() => {
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      return files.filter(
        (f) => f.name.toLowerCase().includes(q) || f.hash.toLowerCase().includes(q)
      );
    }

    const dirMap = new Map<string, { size_bytes: number; count: number }>();
    const fileEntries: FileEntry[] = [];

    for (const folder of customFolders) {
      if (folder.startsWith(currentPrefix)) {
        const rel = folder.slice(currentPrefix.length);
        const parts = rel.split('/').filter(Boolean);
        if (parts.length > 0) {
          const directDir = parts[0];
          if (!dirMap.has(directDir)) {
            dirMap.set(directDir, { size_bytes: 0, count: 0 });
          }
        }
      }
    }

    for (const file of files) {
      const fileName = file.name;
      if (currentPrefix === '') {
        if (fileName.includes('/')) {
          const dirName = fileName.split('/')[0];
          const cur = dirMap.get(dirName) || { size_bytes: 0, count: 0 };
          dirMap.set(dirName, {
            size_bytes: cur.size_bytes + file.size_bytes,
            count: cur.count + 1,
          });
        } else {
          fileEntries.push(file);
        }
      } else {
        if (fileName.startsWith(currentPrefix)) {
          const rel = fileName.slice(currentPrefix.length);
          if (rel.includes('/')) {
            const dirName = rel.split('/')[0];
            const cur = dirMap.get(dirName) || { size_bytes: 0, count: 0 };
            dirMap.set(dirName, {
              size_bytes: cur.size_bytes + file.size_bytes,
              count: cur.count + 1,
            });
          } else {
            fileEntries.push({
              ...file,
              name: rel,
            });
          }
        }
      }
    }

    const folderEntries: FileEntry[] = Array.from(dirMap.entries()).map(([dirName, info]) => ({
      name: dirName,
      hash: `folder-${currentPrefix}${dirName}`,
      size_bytes: info.size_bytes,
      is_dir: true,
      sync_state: 'cached',
      item_count: info.count,
    }));

    folderEntries.sort((a, b) => a.name.localeCompare(b.name));
    fileEntries.sort((a, b) => a.name.localeCompare(b.name));

    return [...folderEntries, ...fileEntries];
  }, [files, customFolders, currentPrefix, searchQuery]);

  // If first launch and onboarding is not complete, show full wizard
  if (onboardingComplete === false) {
    return <OnboardingWizard onComplete={() => setOnboardingComplete(true)} />;
  }

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-gray-50 text-gray-900 select-none">
      <input
        type="file"
        ref={fileInputRef}
        onChange={handleFileInputChange}
        className="hidden"
      />
      <input
        type="file"
        ref={zipInputRef}
        accept=".zip"
        onChange={handleZipInputChange}
        className="hidden"
      />

      {/* Top Application Header */}
      <header className="h-14 bg-white border-b border-gray-200 px-4 flex items-center justify-between z-10">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-blue-600 rounded-lg text-white shadow-xs">
            <FolderGit2 className="w-5 h-5" />
          </div>
          <div>
            <h1 className="text-sm font-bold tracking-tight text-gray-900">
              P2P Shared Cloud
            </h1>
            <button
              onClick={() => {
                if (nodeId) {
                  navigator.clipboard.writeText(nodeId);
                  setToastMessage(`Copied Node ID to clipboard: ${nodeId.slice(0, 16)}...`);
                }
              }}
              className="text-[10px] font-mono text-gray-400 hover:text-blue-600 truncate max-w-[200px] text-left cursor-pointer transition-colors block"
              title={nodeId ? `Click to copy full Node ID: ${nodeId}` : 'Connecting to Iroh Swarm...'}
            >
              {nodeId ? `Node: ${nodeId.slice(0, 16)}...` : 'Connecting to Iroh Swarm...'}
            </button>
          </div>
        </div>

        {/* Search Bar */}
        <div className="flex-1 max-w-md mx-8 relative">
          <Search className="w-4 h-4 absolute left-3 top-2.5 text-gray-400" />
          <input
            type="text"
            placeholder="Search workspace files & hashes..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-9 pr-4 py-1.5 bg-gray-100 hover:bg-gray-100/80 focus:bg-white border border-transparent focus:border-blue-500 rounded-lg text-xs transition-all outline-hidden"
          />
        </div>

        {/* Global Action Buttons */}
        <div className="flex items-center gap-2">
          <button
            onClick={handleTriggerZipImport}
            disabled={importingZip}
            className="px-3 py-1.5 bg-white border border-gray-300 hover:bg-gray-50 text-gray-700 rounded-lg text-xs font-semibold flex items-center gap-1.5 shadow-2xs transition-colors disabled:opacity-50 cursor-pointer"
            title="Import and unpack a ZIP archive of course files directly into the active folder"
          >
            <FileArchive className="w-3.5 h-3.5 text-amber-600" />
            <span>{importingZip ? 'Importing ZIP...' : 'Import ZIP'}</span>
          </button>
          <button
            onClick={handleTriggerUpload}
            className="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-semibold flex items-center gap-1.5 shadow-xs transition-colors cursor-pointer"
          >
            <Upload className="w-3.5 h-3.5" />
            Upload File
          </button>
          <button
            onClick={() => {
              loadWorkspaceFiles();
              refetchStats();
              refreshNetworkProfile();
            }}
            className="p-1.5 hover:bg-gray-100 rounded-lg text-gray-500 hover:text-gray-700 transition-colors"
            title="Refresh Swarm Catalog"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
          <button
            onClick={() => setCurrentView('settings')}
            className={`p-1.5 rounded-lg transition-colors ${
              currentView === 'settings'
                ? 'bg-blue-100 text-blue-700'
                : 'hover:bg-gray-100 text-gray-500 hover:text-gray-700'
            }`}
            title="Workspace Settings"
          >
            <Settings className="w-4 h-4" />
          </button>
          <button
            onClick={handleRequestQuit}
            className="p-1.5 hover:bg-rose-50 text-gray-500 hover:text-rose-600 rounded-lg transition-colors cursor-pointer"
            title="Quit or Minimize P2P Workspace"
          >
            <Power className="w-4 h-4" />
          </button>
        </div>
      </header>

      {/* Toast Notification */}
      {toastMessage && (
        <div className="bg-blue-600 text-white text-xs px-4 py-2 font-medium flex items-center justify-between shadow-md z-30 transition-all">
          <span>{toastMessage}</span>
          <button
            onClick={() => setToastMessage(null)}
            className="ml-4 text-blue-200 hover:text-white"
          >
            ✕
          </button>
        </div>
      )}

      {/* Main Body */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Sidebar */}
        <aside className="w-56 bg-gray-50 border-r border-gray-200 flex flex-col justify-between py-4 select-none shrink-0">
          <div className="space-y-6">
            <div>
              <span className="px-3 text-[11px] font-semibold tracking-wider text-gray-400 uppercase">
                Workspaces
              </span>
              <div className="mt-1.5 space-y-0.5">
                <button
                  onClick={() => handleNavigateBreadcrumbs(0)}
                  className={`w-full flex items-center gap-2.5 px-3 py-1.5 text-xs font-medium rounded-lg transition-colors ${
                    currentView === 'explorer'
                      ? 'bg-blue-50 text-blue-700'
                      : 'text-gray-600 hover:bg-gray-100'
                  }`}
                >
                  <Share2 className="w-4 h-4 text-blue-500" />
                  Shared Cloud Folder
                </button>
              </div>
            </div>

            <div>
              <span className="px-3 text-[11px] font-semibold tracking-wider text-gray-400 uppercase">
                Quick Filters
              </span>
              <div className="mt-1.5 space-y-0.5">
                <button
                  onClick={() => setSearchQuery((prev) => (prev === '.pdf' ? '' : '.pdf'))}
                  className={`w-full flex items-center gap-2.5 px-3 py-1.5 text-xs rounded-lg transition-colors cursor-pointer ${
                    searchQuery === '.pdf'
                      ? 'bg-red-50 text-red-700 font-semibold'
                      : 'text-gray-600 hover:bg-gray-100'
                  }`}
                  title={searchQuery === '.pdf' ? 'Click to clear filter' : 'Filter files ending with .pdf'}
                >
                  <FileText className="w-3.5 h-3.5 text-red-500" />
                  Documents (.pdf)
                </button>
                <button
                  onClick={() => setSearchQuery((prev) => (prev === '.md' ? '' : '.md'))}
                  className={`w-full flex items-center gap-2.5 px-3 py-1.5 text-xs rounded-lg transition-colors cursor-pointer ${
                    searchQuery === '.md'
                      ? 'bg-blue-50 text-blue-700 font-semibold'
                      : 'text-gray-600 hover:bg-gray-100'
                  }`}
                  title={searchQuery === '.md' ? 'Click to clear filter' : 'Filter files ending with .md or .txt'}
                >
                  <FileCode className="w-3.5 h-3.5 text-blue-500" />
                  Notes (.md, .txt)
                </button>
              </div>
            </div>

            <div>
              <span className="px-3 text-[11px] font-semibold tracking-wider text-gray-400 uppercase">
                Configuration
              </span>
              <div className="mt-1.5 space-y-0.5">
                <button
                  onClick={() => {
                    setCurrentView('settings');
                    setSettingsTab('network');
                  }}
                  className="w-full flex items-center gap-2.5 px-3 py-1.5 text-xs text-gray-600 rounded-lg hover:bg-gray-100"
                >
                  <Wifi className="w-3.5 h-3.5 text-emerald-500" />
                  Network & Stealth
                </button>
                <button
                  onClick={() => {
                    setCurrentView('settings');
                    setSettingsTab('backup');
                  }}
                  className="w-full flex items-center gap-2.5 px-3 py-1.5 text-xs text-gray-600 rounded-lg hover:bg-gray-100"
                >
                  <HardDrive className="w-3.5 h-3.5 text-purple-500" />
                  Backups & Export
                </button>
              </div>
            </div>
          </div>

          <div className="px-3 space-y-2">
            <button
              onClick={() => {
                setCurrentView('settings');
                setSettingsTab('cache');
              }}
              className="w-full flex items-center justify-between p-2 rounded-lg border border-gray-200 hover:bg-gray-50 transition-colors"
            >
              <div className="flex items-center gap-2 text-xs font-semibold text-gray-700">
                <Settings className="w-3.5 h-3.5 text-gray-500" />
                Cache Limits
              </div>
              <span className="text-[10px] font-bold text-blue-600 bg-blue-50 px-1.5 py-0.5 rounded">
                {stats ? `${Math.round(stats.quota_bytes / (1024 * 1024 * 1024))}GB` : '5GB'}
              </span>
            </button>
          </div>
        </aside>

        {/* Center / Content Area */}
        <main className="flex-1 flex flex-col bg-white overflow-hidden">
          {/* Top Banners */}
          <SoleSeedBanner
            onShowSoleSeeds={async () => {
              try {
                const soleSeeds = await invoke<SoleSeedFile[]>('get_sole_seed_files');
                if (soleSeeds && soleSeeds.length > 0) {
                  setPaths(['Shared Cloud']);
                  const hashes = soleSeeds.map((s) => s.hash);
                  setSelectedHashes(hashes);
                  setToastMessage(`Highlighted ${soleSeeds.length} rare/sole-seeded file(s) across your workspace.`);
                }
              } catch (err) {
                console.warn('Failed to display sole seeds:', err);
              }
            }}
          />
          <OfflineBanner
            isOnline={isOnline}
            relayConnected={relayConnected}
            disconnectReason={disconnectReason}
            relayUrl={relayUrl}
            onOpenSettings={() => {
              setCurrentView('settings');
              setSettingsTab('network');
            }}
          />
          <ImportingBanner progress={importProgress} onDismiss={() => setImportProgress(null)} />

          {/* Unrecognized Network Prompt (R17.4) */}
          {networkProfile?.ssid && !networkProfile?.is_known_network && !dismissedNetworkPrompt && (
            <div className="bg-gradient-to-r from-blue-900 via-indigo-900 to-blue-950 text-white px-4 py-2.5 flex items-center justify-between text-xs border-b border-indigo-700/60 shadow-md animate-in fade-in">
              <div className="flex items-center gap-2.5 min-w-0">
                <Wifi className="w-4 h-4 text-cyan-400 shrink-0" />
                <span className="truncate">
                  New Wi-Fi detected: <strong className="text-white font-mono">{networkProfile.ssid}</strong>. How should P2PStud classify this network?
                </span>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                <button
                  onClick={() => handleClassifyNetwork(networkProfile.ssid!, 'campus')}
                  className="px-2.5 py-1 bg-amber-500 hover:bg-amber-400 text-slate-950 font-semibold rounded text-xs transition cursor-pointer shadow-2xs"
                  title="Limit background sync speeds to 512 KB/s and enable lecture stealth"
                >
                  Campus Stealth
                </button>
                <button
                  onClick={() => handleClassifyNetwork(networkProfile.ssid!, 'trusted')}
                  className="px-2.5 py-1 bg-emerald-600 hover:bg-emerald-500 text-white font-semibold rounded text-xs transition cursor-pointer shadow-2xs"
                  title="Enable unrestricted high-speed sync for home or lab Wi-Fi"
                >
                  Trusted Network
                </button>
                <button
                  onClick={() => setDismissedNetworkPrompt(true)}
                  className="text-gray-400 hover:text-white text-xs px-1 cursor-pointer"
                  title="Dismiss prompt for this session"
                >
                  ✕
                </button>
              </div>
            </div>
          )}

          {currentView === 'explorer' && (
            <>
              <Breadcrumbs paths={paths} onNavigate={handleNavigateBreadcrumbs} />
              <FileTree
                files={currentDisplayFiles}
                selectedHash={selectedHash}
                selectedHashes={selectedHashes}
                onToggleSelectHash={handleToggleSelectHash}
                onClearSelection={handleClearSelection}
                onBatchDelete={handleBatchDelete}
                onBatchDownload={handleBatchDownload}
                onBatchMove={handleBatchMove}
                currentUserId={nodeId}
                uploadQueue={uploadQueue}
                isRoot={paths.length <= 1}
                currentFolderName={paths.length > 1 ? paths[paths.length - 1] : undefined}
                onSelectFile={handleSelectFile}
                onNavigateFolder={handleNavigateFolder}
                onNavigateUp={handleNavigateUp}
                onCreateFolder={handleCreateFolder}
                onDeleteFolder={handleDeleteFolder}
                onUploadFiles={handleUploadFiles}
                onExportFolderZip={handleExportFolderZip}
                onPin={handlePin}
                onUnpin={handleUnpin}
                onDownload={handleDownload}
                onPreview={(hash) => {
                  const f = files.find((item) => item.hash === hash);
                  if (f) handleSelectFile(f);
                }}
                onDelete={handleDelete}
                onVoteDelete={handleVoteDelete}
                onUnvoteDelete={handleUnvoteDelete}
                onVoteKeep={handleVoteKeep}
                onUnvoteKeep={handleUnvoteKeep}
                onEvictLocal={handleEvictLocal}
                onMove={handlePromptMove}
                onMoveToFolder={handleMoveToFolder}
                onRate={handleRate}
                onToggleStar={handleToggleStar}
                onReportDangerous={handleReportDangerous}
                onFileUploaded={() => {
                  loadWorkspaceFiles();
                  refetchStats();
                }}
                onTriggerUpload={handleTriggerUpload}
              />
            </>
          )}

          {currentView === 'pdf' && (
            <PdfViewer
              data={pdfData}
              onClose={() => setCurrentView('explorer')}
            />
          )}

          {currentView === 'markdown' && (
            <MarkdownEditor
              initialContent="# Notes & Markdown Preview\n\nEdit notes here or preview documents."
              onClose={() => setCurrentView('explorer')}
            />
          )}

          {currentView === 'settings' && (
            <div className="flex-1 p-8 bg-gray-50 overflow-y-auto">
              <div className="max-w-3xl mx-auto space-y-6">
                <div className="flex items-center justify-between">
                  <div>
                    <h2 className="text-xl font-bold text-gray-900">Application Settings</h2>
                    <p className="text-xs text-gray-500">Configure cache, stealth network rules, diagnostics, and backups.</p>
                  </div>
                  <div className="flex items-center gap-3">
                    <button
                      onClick={() => setOnboardingComplete(false)}
                      className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-semibold text-blue-700 bg-blue-50 border border-blue-200 rounded-lg hover:bg-blue-100 transition shadow-sm"
                      title="Run the initial setup wizard again"
                    >
                      <Sparkles className="w-3.5 h-3.5 text-blue-600" />
                      Re-run Onboarding
                    </button>
                    <button
                      onClick={() => setCurrentView('explorer')}
                      className="text-xs text-blue-600 hover:underline font-semibold"
                    >
                      ← Back to Files
                    </button>
                  </div>
                </div>

                {/* Settings Navigation Tabs */}
                <div className="flex border-b border-gray-200 space-x-4">
                  <button
                    onClick={() => setSettingsTab('cache')}
                    className={`pb-2 text-xs font-semibold border-b-2 transition-colors ${
                      settingsTab === 'cache'
                        ? 'border-blue-600 text-blue-600'
                        : 'border-transparent text-gray-500 hover:text-gray-700'
                    }`}
                  >
                    Cache & Quota
                  </button>
                  <button
                    onClick={() => setSettingsTab('network')}
                    className={`pb-2 text-xs font-semibold border-b-2 transition-colors ${
                      settingsTab === 'network'
                        ? 'border-blue-600 text-blue-600'
                        : 'border-transparent text-gray-500 hover:text-gray-700'
                    }`}
                  >
                    Network & Swarm
                  </button>
                  <button
                    onClick={() => setSettingsTab('backup')}
                    className={`pb-2 text-xs font-semibold border-b-2 transition-colors ${
                      settingsTab === 'backup'
                        ? 'border-blue-600 text-blue-600'
                        : 'border-transparent text-gray-500 hover:text-gray-700'
                    }`}
                  >
                    Backups & Disaster Recovery
                  </button>
                </div>

                {settingsTab === 'cache' && (
                  <CacheSlider stats={stats} onStatsUpdated={refetchStats} />
                )}

                {settingsTab === 'network' && (
                  <NetworkSettings />
                )}

                {settingsTab === 'backup' && (
                  <BackupSettings />
                )}
              </div>
            </div>
          )}
        </main>
      </div>

      {/* Floating Upload Queue Notification Drawer */}
      <UploadQueueDrawer
        queue={uploadQueue}
        onDismiss={handleDismissUpload}
        onClearAll={handleClearUploadQueue}
      />

      {/* Persistent Bottom Status & Telemetry Bar */}
      <TelemetryBar 
        stats={stats} 
        networkProfile={networkProfile}
        importProgress={importProgress}
        onOpenSettings={() => {
          setCurrentView('settings');
          setSettingsTab('cache');
        }}
        onOpenNetworkSettings={() => {
          setCurrentView('settings');
          setSettingsTab('network');
        }}
      />

      {/* ZIP Archive Import Security & Extraction Warning Modal */}
      <ZipImportModal
        isOpen={showZipModal}
        file={pendingZipFile}
        targetFolder={currentPrefix ? `Shared Cloud / ${currentPrefix}` : 'Shared Cloud'}
        onUnpack={handleExecuteZipUnpack}
        onUploadAsZip={handleExecuteUploadAsZip}
        onCancel={handleCancelZipModal}
      />

      {/* Non-intrusive Update Available Toast (R23.3) */}
      {availableUpdate && (
        <div className="fixed bottom-12 right-6 bg-slate-900 text-white p-4 rounded-xl shadow-2xl border border-slate-700 z-50 max-w-sm animate-in fade-in slide-in-from-bottom-4">
          <div className="flex items-start gap-3">
            <Sparkles className="w-5 h-5 text-blue-400 shrink-0 mt-0.5" />
            <div className="flex-1 min-w-0">
              <div className="font-semibold text-xs text-white">Update Available: v{availableUpdate.version}</div>
              <p className="text-[11px] text-slate-300 mt-1 line-clamp-2">{availableUpdate.notes}</p>
              {downloadProgress && (
                <div className="mt-2.5 p-2 bg-slate-800/80 rounded-lg space-y-1">
                  <div className="flex justify-between text-[10px] text-blue-300 font-mono">
                    <span>Downloading: {(downloadProgress.downloaded_bytes / (1024 * 1024)).toFixed(1)} / {(downloadProgress.total_bytes / (1024 * 1024)).toFixed(1)} MB</span>
                    <span>{downloadProgress.percentage.toFixed(0)}%</span>
                  </div>
                  <div className="w-full bg-slate-700 rounded-full h-1.5 overflow-hidden">
                    <div
                      className="bg-blue-400 h-1.5 rounded-full transition-all duration-150"
                      style={{ width: `${downloadProgress.percentage}%` }}
                    />
                  </div>
                </div>
              )}
              <div className="flex flex-wrap items-center gap-2 mt-3">
                <button
                  onClick={handleUpdateAndRestartNow}
                  disabled={isUpdatingNow}
                  className="px-3 py-1 bg-blue-600 hover:bg-blue-500 text-white rounded text-xs font-semibold cursor-pointer shadow-xs disabled:opacity-50 flex items-center gap-1.5"
                >
                  {isUpdatingNow && <RefreshCw className="w-3.5 h-3.5 animate-spin" />}
                  <span>{isUpdatingNow ? 'Downloading & Restarting...' : 'Update & Restart Now'}</span>
                </button>
                <button
                  onClick={handleApplyUpdateOnRestart}
                  disabled={isUpdatingNow}
                  className="px-2.5 py-1 bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-600 rounded text-xs font-medium cursor-pointer disabled:opacity-50"
                >
                  On Next Restart
                </button>
                <button
                  onClick={handleDismissUpdateToast}
                  disabled={isUpdatingNow}
                  className="px-2 py-1 text-slate-400 hover:text-white text-xs cursor-pointer disabled:opacity-50"
                >
                  Later
                </button>
              </div>
            </div>
            <button onClick={handleDismissUpdateToast} className="text-slate-400 hover:text-white cursor-pointer">
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* Quit Intercept / Safe Shutdown Confirmation Modal (R16.2/3, R23.3) */}
      {showQuitModal && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-xs flex items-center justify-center z-50 p-4">
          <div className="bg-white rounded-xl shadow-2xl max-w-md w-full border border-gray-200 p-6 space-y-4">
            <div className="flex items-start gap-3">
              <div className="w-10 h-10 rounded-full bg-amber-50 text-amber-600 flex items-center justify-center border border-amber-200 shrink-0">
                <AlertTriangle className="w-5 h-5" />
              </div>
              <div>
                <h3 className="text-sm font-bold text-gray-900">
                  {isUpdatePending
                    ? 'Update Ready to Install'
                    : soleSeedsOnQuit.length > 0
                    ? 'Sole Seed Shutdown Warning'
                    : 'Exit P2P Workspace'}
                </h3>
                <p className="text-xs text-gray-600 mt-1 leading-relaxed">
                  {isUpdatePending
                    ? 'A downloaded update is ready to install. Do you want to apply it now and restart, or close without installing?'
                    : soleSeedsOnQuit.length > 0
                    ? `Warning: You are currently the ONLY active online seed for ${soleSeedsOnQuit.length} course file(s). If you close the app now, these files will be unavailable to your peers until you return.`
                    : 'Are you sure you want to exit P2PStud? Background seeding will stop.'}
                </p>
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-gray-100">
              <button
                onClick={() => setShowQuitModal(false)}
                className="px-3 py-1.5 text-xs text-gray-600 hover:bg-gray-100 rounded-lg font-medium cursor-pointer"
              >
                Cancel
              </button>
              <button
                onClick={handleMinimizeToTray}
                className="px-3 py-1.5 text-xs bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg font-medium cursor-pointer"
                title="Keep seeding in background while hiding the window"
              >
                Minimize to Tray
              </button>
              {isUpdatePending ? (
                <>
                  <button
                    onClick={() => handleConfirmExit(false)}
                    className="px-3 py-1.5 text-xs bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg font-medium cursor-pointer"
                  >
                    Quit Without Installing
                  </button>
                  <button
                    onClick={() => handleConfirmExit(true)}
                    className="px-4 py-1.5 text-xs bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-semibold shadow-xs cursor-pointer"
                  >
                    Install & Restart
                  </button>
                </>
              ) : (
                <button
                  onClick={() => handleConfirmExit(false)}
                  className={`px-4 py-1.5 text-xs text-white rounded-lg font-semibold shadow-xs cursor-pointer ${
                    soleSeedsOnQuit.length > 0 ? 'bg-red-600 hover:bg-red-700' : 'bg-gray-900 hover:bg-gray-800'
                  }`}
                >
                  {soleSeedsOnQuit.length > 0 ? 'Quit Anyway' : 'Quit'}
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
