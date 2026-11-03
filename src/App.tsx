import React, { useState, useEffect, useRef } from 'react';
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
import { FileEntry } from './types';
import { useStorageStats } from './hooks/useStorageStats';
import { usePeerStatus } from './hooks/usePeerStatus';
import { Breadcrumbs } from './components/Explorer/Breadcrumbs';
import { FileTree } from './components/Explorer/FileTree';
import { PdfViewer } from './components/Viewers/PdfViewer';
import { MarkdownEditor } from './components/Viewers/MarkdownEditor';
import { CacheSlider } from './components/Settings/CacheSlider';
import { TelemetryBar } from './components/Footer/TelemetryBar';
import { invoke } from '@tauri-apps/api/core';

interface BlobRecord {
  hash: string;
  name: string | null;
  size_bytes: number;
  last_accessed_at: number;
  is_pinned: boolean;
  owner_node_id: string | null;
}

export default function App() {
  const [paths, setPaths] = useState<string[]>(['Shared Cloud']);
  const [files, setFiles] = useState<FileEntry[]>([]);
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
            sync_state: r.is_pinned ? 'pinned' : 'cached',
            owner_node_id: r.owner_node_id || undefined,
            last_accessed_at: r.last_accessed_at,
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
    const interval = setInterval(loadWorkspaceFiles, 3500);
    return () => clearInterval(interval);
  }, []);

  const handleTriggerUpload = () => {
    fileInputRef.current?.click();
  };

  const handleFileInputChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const selectedFiles = e.target.files;
    if (!selectedFiles || selectedFiles.length === 0) return;

    for (let i = 0; i < selectedFiles.length; i++) {
      const file = selectedFiles[i];
      try {
        const buffer = await file.arrayBuffer();
        const bytes = Array.from(new Uint8Array(buffer));
        await invoke('upload_file_bytes', { name: file.name, data: bytes });
      } catch (err) {
        console.error('Failed to upload file:', err);
        alert(`Failed to upload ${file.name}: ${err}`);
      }
    }

    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }

    await loadWorkspaceFiles();
    refetchStats();
  };

  const handleDelete = async (hash: string, forceOrphan = false) => {
    try {
      await invoke('delete_file', { hash, forceOrphan });
      setFiles((prev) => prev.filter((f) => f.hash !== hash));
      refetchStats();
    } catch (err) {
      console.error('Delete error:', err);
      alert(`Delete failed: ${err}`);
    }
  };

  const handleDeleteOrphan = async (hash: string) => {
    const file = files.find((f) => f.hash === hash);
    const fileName = file?.name || hash.slice(0, 8);
    if (
      confirm(
        `Are you sure you want to clean up "${fileName}"?\n\nThis will remove the file from the workspace for everyone because the original owner is no longer active in the swarm.`
      )
    ) {
      await handleDelete(hash, true);
    }
  };

  const handleEvictLocal = async (hash: string) => {
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

  const handleNavigate = (index: number) => {
    if (index === -1) {
      setPaths([]);
    } else {
      setPaths(paths.slice(0, index + 1));
    }
    setCurrentView('explorer');
  };

  const handleNavigateFolder = (folderName: string) => {
    setPaths([...paths, folderName]);
    setCurrentView('explorer');
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
        prev.map((f) => (f.hash === hash ? { ...f, sync_state: 'cached' } : f))
      );
    }
  };

  const filteredFiles = files.filter((f) =>
    f.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

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
            className="p-1.5 rounded-md hover:bg-gray-100 text-gray-600 hover:text-gray-900 transition-colors"
            title="Refresh Swarm Cache"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
          <button
            onClick={() => setCurrentView('settings')}
            className={`p-1.5 rounded-md transition-colors ${
              currentView === 'settings'
                ? 'bg-blue-50 text-blue-600'
                : 'text-gray-600 hover:text-gray-900 hover:bg-gray-100'
            }`}
            title="Settings & Host-to-Play Cache Allocation"
          >
            <Settings className="w-4 h-4" />
          </button>
        </div>
      </header>

      {/* Main Workspace Body */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Navigation Sidebar */}
        <aside className="w-56 bg-white border-r border-gray-200 flex flex-col justify-between py-3">
          <div className="space-y-4 px-2">
            <div>
              <span className="px-3 text-[11px] font-semibold tracking-wider text-gray-400 uppercase">
                Workspaces
              </span>
              <div className="mt-1.5 space-y-0.5">
                <button
                  onClick={() => setCurrentView('explorer')}
                  className={`w-full flex items-center gap-2.5 px-3 py-2 text-xs rounded-lg font-medium transition-colors ${
                    currentView === 'explorer'
                      ? 'bg-blue-50 text-blue-700'
                      : 'text-gray-700 hover:bg-gray-100'
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
              <Breadcrumbs paths={paths} onNavigate={handleNavigate} />
              <FileTree
                files={filteredFiles}
                selectedHash={selectedHash}
                currentUserId={nodeId}
                onSelectFile={handleSelectFile}
                onNavigateFolder={handleNavigateFolder}
                onPin={handlePin}
                onUnpin={handleUnpin}
                onDownload={handleDownload}
                onPreview={(hash) => {
                  const f = files.find((item) => item.hash === hash);
                  if (f) handleSelectFile(f);
                }}
                onDelete={(hash) => handleDelete(hash, false)}
                onDeleteOrphan={handleDeleteOrphan}
                onEvictLocal={handleEvictLocal}
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
              hash={selectedHash ?? undefined}
              onClose={() => setCurrentView('explorer')}
            />
          )}

          {currentView === 'settings' && (
            <div className="flex-1 overflow-y-auto p-8">
              <CacheSlider stats={stats} onStatsUpdated={refetchStats} />
            </div>
          )}
        </main>
      </div>

      {/* Persistent Bottom Telemetry Status Bar */}
      <TelemetryBar stats={stats} />
    </div>
  );
}
