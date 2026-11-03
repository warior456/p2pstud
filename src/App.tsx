import React, { useState, useEffect, useRef, useMemo } from 'react';
import { 
  FolderGit2, 
  Settings, 
  FileCode, 
  FileText, 
  Share2, 
  RefreshCw,
  Search,
  Upload
} from 'lucide-react';
import { FileEntry, UploadItem } from './types';
import { useStorageStats } from './hooks/useStorageStats';
import { usePeerStatus } from './hooks/usePeerStatus';
import { Breadcrumbs } from './components/Explorer/Breadcrumbs';
import { FileTree } from './components/Explorer/FileTree';
import { PdfViewer } from './components/Viewers/PdfViewer';
import { MarkdownEditor } from './components/Viewers/MarkdownEditor';
import { CacheSlider } from './components/Settings/CacheSlider';
import { UploadQueueDrawer } from './components/Explorer/UploadQueueDrawer';
import { TelemetryBar } from './components/Footer/TelemetryBar';
import { invoke } from '@tauri-apps/api/core';

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
}

export default function App() {
  const [paths, setPaths] = useState<string[]>(['Shared Cloud']);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [customFolders, setCustomFolders] = useState<string[]>([]);
  const [uploadQueue, setUploadQueue] = useState<UploadItem[]>([]);
  const [selectedHash, setSelectedHash] = useState<string | null>(null);
  const [currentView, setCurrentView] = useState<'explorer' | 'pdf' | 'markdown' | 'settings'>('explorer');
  const [searchQuery, setSearchQuery] = useState('');
  const [pdfData, setPdfData] = useState<Uint8Array | null>(null);

  const fileInputRef = useRef<HTMLInputElement>(null);

  const { stats, refetch: refetchStats } = useStorageStats();
  const { nodeId } = usePeerStatus();

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
          }))
        );
      }
    } catch (err) {
      console.warn('Failed to fetch workspace files:', err);
    }
  };

  useEffect(() => {
    loadWorkspaceFiles();
    // Periodically poll catalog so peers' uploads appear automatically
    const interval = setInterval(loadWorkspaceFiles, 2000);
    return () => clearInterval(interval);
  }, []);

  const handleTriggerUpload = () => {
    fileInputRef.current?.click();
  };

  // Current directory relative prefix within workspace, e.g. "Math101/"
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

  const handleCreateFolder = () => {
    const folderName = prompt('Enter folder name:');
    if (!folderName || !folderName.trim()) return;
    const cleanName = folderName.trim().replace(/[\/\\]/g, '-');
    const fullFolderPath = currentPrefix ? `${currentPrefix}${cleanName}` : cleanName;

    if (!customFolders.includes(fullFolderPath)) {
      setCustomFolders((prev) => [...prev, fullFolderPath]);
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

    // Extract basename from existing name
    const basename = file.name.split('/').pop() || file.name;
    const cleanFolder = targetFolderPath.trim().replace(/^\/+|\/+$/g, '');
    const newPath = cleanFolder ? `${cleanFolder}/${basename}` : basename;

    if (newPath === file.name) return; // Unchanged

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

    // Collect all existing directory paths across all files + custom folders
    const knownFolders = new Set<string>();
    for (const f of files) {
      if (f.name.includes('/')) {
        const parts = f.name.split('/');
        parts.pop(); // remove filename
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

    if (destination === null) return; // User cancelled
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

  const currentDisplayFiles = useMemo(() => {
    // If user is searching globally, search across all workspace files
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      return files.filter(
        (f) => f.name.toLowerCase().includes(q) || f.hash.toLowerCase().includes(q)
      );
    }

    const dirMap = new Map<string, { size_bytes: number; count: number }>();
    const fileEntries: FileEntry[] = [];

    // Custom folders under current directory
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

    // Process files under current directory
    for (const file of files) {
      const fileName = file.name;
      if (currentPrefix === '') {
        // Root workspace
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
        // Inside subfolder
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
              name: rel, // Basename within this folder
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

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-gray-50 text-gray-900 select-none">
      <input
        type="file"
        ref={fileInputRef}
        onChange={handleFileInputChange}
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
            }}
            className="p-1.5 hover:bg-gray-100 rounded-lg text-gray-500 hover:text-gray-700 transition-colors"
            title="Refresh Swarm Catalog"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
          <button
            onClick={() => setCurrentView('settings')}
            className="p-1.5 hover:bg-gray-100 rounded-lg text-gray-500 hover:text-gray-700 transition-colors"
            title="Workspace Settings"
          >
            <Settings className="w-4 h-4" />
          </button>
        </div>
      </header>

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
          </div>

          <div className="px-3">
            <button
              onClick={() => setCurrentView('settings')}
              className="w-full flex items-center justify-between p-2 rounded-lg border border-gray-200 hover:bg-gray-50 transition-colors"
            >
              <div className="flex items-center gap-2 text-xs font-semibold text-gray-700">
                <Settings className="w-3.5 h-3.5 text-gray-500" />
                Cache Slider
              </div>
              <span className="text-[10px] font-bold text-blue-600 bg-blue-50 px-1.5 py-0.5 rounded">
                {stats ? `${Math.round(stats.quota_bytes / (1024 * 1024 * 1024))}GB` : '5GB'}
              </span>
            </button>
          </div>
        </aside>

        {/* Center / Content Area */}
        <main className="flex-1 flex flex-col bg-white overflow-hidden">
          {currentView === 'explorer' && (
            <>
              <Breadcrumbs paths={paths} onNavigate={handleNavigateBreadcrumbs} />
              <FileTree
                files={currentDisplayFiles}
                selectedHash={selectedHash}
                currentUserId={nodeId}
                uploadQueue={uploadQueue}
                isRoot={paths.length <= 1}
                onSelectFile={handleSelectFile}
                onNavigateFolder={handleNavigateFolder}
                onNavigateUp={handleNavigateUp}
                onCreateFolder={handleCreateFolder}
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
              <div className="max-w-2xl mx-auto space-y-6">
                <div className="flex items-center justify-between">
                  <h2 className="text-lg font-bold text-gray-900">Workspace & Cache Settings</h2>
                  <button
                    onClick={() => setCurrentView('explorer')}
                    className="text-xs text-blue-600 hover:underline"
                  >
                    ← Back to Files
                  </button>
                </div>
                <CacheSlider stats={stats} onStatsUpdated={refetchStats} />
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
      <TelemetryBar stats={stats} onOpenSettings={() => setCurrentView('settings')} />
    </div>
  );
}
