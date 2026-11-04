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
  Sparkles
} from 'lucide-react';
import { FileEntry, UploadItem, ZipImportSummary, NetworkProfileInfo } from './types';
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
import { OnboardingWizard } from './components/Onboarding/OnboardingWizard';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

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

  const { stats, refetch: refetchStats } = useStorageStats();
  const { nodeId } = usePeerStatus();
  const { isOnline } = useConnectionStatus();

  // Check onboarding status on startup
  useEffect(() => {
    invoke<string | null>('get_setting', { key: 'onboarding_complete' })
      .then((val) => {
        setOnboardingComplete(val === 'true');
      })
      .catch((err) => {
        console.warn('Could not read onboarding status:', err);
        setOnboardingComplete(true); // default to true on error so app stays usable
      });
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

    return () => {
      if (unlistenProfile) unlistenProfile();
      if (unlistenSsid) unlistenSsid();
      if (unlistenScarcity) unlistenScarcity();
      if (unlistenQuarantine) unlistenQuarantine();
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
            owner_node_id: r.owner_node_id || undefined,
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

  useEffect(() => {
    loadWorkspaceFiles();
    const interval = setInterval(loadWorkspaceFiles, 2000);
    return () => clearInterval(interval);
  }, []);

  const handleTriggerUpload = () => {
    fileInputRef.current?.click();
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

      try {
        setUploadQueue((prev) =>
          prev.map((q) => (q.id === item.id ? { ...q, progress: 30, status: 'reading' } : q))
        );
        const buffer = await file.arrayBuffer();
        const bytes = Array.from(new Uint8Array(buffer));

        setUploadQueue((prev) =>
          prev.map((q) => (q.id === item.id ? { ...q, progress: 65, status: 'uploading' } : q))
        );

        await invoke('upload_file_bytes', { name: targetName, data: bytes });

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

    if (fileInputRef.current) {
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

  const handleTriggerZipImport = () => {
    zipInputRef.current?.click();
  };

  const handleZipInputChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    setImportingZip(true);
    try {
      const buffer = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buffer));
      const targetFolder = currentPrefix.replace(/\/$/, '') || null;
      const summary = await invoke<ZipImportSummary>('import_zip_bytes', {
        data: bytes,
        targetFolder,
      });

      const mb = (summary.total_bytes / (1024 * 1024)).toFixed(2);
      let msg = `Successfully imported ${summary.files_imported} file(s) (${mb} MB) from ${file.name}.`;
      if (summary.errors && summary.errors.length > 0) {
        msg += `\n\n${summary.errors.length} file(s) failed or were skipped:\n` + summary.errors.slice(0, 3).join('\n');
      }
      alert(msg);
      await loadWorkspaceFiles();
      refetchStats();
    } catch (err) {
      alert(`ZIP import failed: ${err}`);
    } finally {
      setImportingZip(false);
      if (zipInputRef.current) {
        zipInputRef.current.value = '';
      }
    }
  };

  const handleCreateFolder = () => {
    const folderName = prompt('Enter folder name:');
    if (!folderName || !folderName.trim()) return;
    const cleanName = folderName.trim().replace(/[\/\\]/g, '-');
    const fullFolderPath = currentPrefix ? `${currentPrefix}${cleanName}` : cleanName;

    if (!customFolders.includes(fullFolderPath)) {
      setCustomFolders((prev) => [...prev, fullFolderPath]);
    }
  };

  const handleDeleteFolder = async (folderName: string) => {
    const targetFolder = currentPrefix ? `${currentPrefix}${folderName}` : folderName;
    if (!confirm(`Are you sure you want to delete folder "${folderName}"?`)) return;

    try {
      await invoke('delete_folder', { folderPath: targetFolder });
      setCustomFolders((prev) => prev.filter((f) => f !== targetFolder));
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
            <p className="text-[10px] font-mono text-gray-400 truncate max-w-[200px]">
              {nodeId ? `Node: ${nodeId.slice(0, 16)}...` : 'Connecting to Iroh Swarm...'}
            </p>
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
          {import.meta.env.DEV && (
            <button
              onClick={handleTriggerZipImport}
              disabled={importingZip}
              className="px-3 py-1.5 bg-amber-500 hover:bg-amber-600 text-white rounded-lg text-xs font-semibold flex items-center gap-1.5 shadow-xs transition-colors disabled:opacity-50"
              title="Developer tool: Import a ZIP archive of files into the active folder"
            >
              <FileArchive className="w-3.5 h-3.5" />
              {importingZip ? 'Importing ZIP...' : 'Import ZIP (Dev)'}
            </button>
          )}
          <button
            onClick={handleTriggerUpload}
            className="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-semibold flex items-center gap-1.5 shadow-xs transition-colors"
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
                  onClick={() => setSearchQuery('.pdf')}
                  className="w-full flex items-center gap-2.5 px-3 py-1.5 text-xs text-gray-600 rounded-lg hover:bg-gray-100"
                >
                  <FileText className="w-3.5 h-3.5 text-red-500" />
                  Documents (.pdf)
                </button>
                <button
                  onClick={() => setSearchQuery('.md')}
                  className="w-full flex items-center gap-2.5 px-3 py-1.5 text-xs text-gray-600 rounded-lg hover:bg-gray-100"
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
          <SoleSeedBanner />
          <OfflineBanner isOnline={isOnline} />

          {currentView === 'explorer' && (
            <>
              <Breadcrumbs paths={paths} onNavigate={handleNavigateBreadcrumbs} />
              <FileTree
                files={currentDisplayFiles}
                selectedHash={selectedHash}
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
                    Network Stealth & Diagnostics
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
        onOpenSettings={() => {
          setCurrentView('settings');
          setSettingsTab('cache');
        }}
        onOpenNetworkSettings={() => {
          setCurrentView('settings');
          setSettingsTab('network');
        }}
      />
    </div>
  );
}
