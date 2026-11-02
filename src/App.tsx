import { useState, useEffect } from 'react';
import { 
  FolderGit2, 
  Settings, 
  FileCode, 
  FileText, 
  Share2, 
  RefreshCw,
  Search
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

// Mock course folder structure for initial campus experience
const INITIAL_FILES: FileEntry[] = [
  {
    name: 'CS101-Lecture-01-Distributed-Systems.pdf',
    hash: 'b3a89e1f00000000000000000000000000000000000000000000000000000001',
    size_bytes: 4210000,
    is_dir: false,
    sync_state: 'cached',
  },
  {
    name: 'CS101-Assignment-2-P2P.md',
    hash: 'c8914b3000000000000000000000000000000000000000000000000000000002',
    size_bytes: 14200,
    is_dir: false,
    sync_state: 'pinned',
  },
  {
    name: 'Midterm-Exam-Review-Slides.pdf',
    hash: 'e41029aa00000000000000000000000000000000000000000000000000000003',
    size_bytes: 15400000,
    is_dir: false,
    sync_state: 'cloud_only',
  },
  {
    name: 'Lab-Solutions-Week-4.md',
    hash: 'f90112dd00000000000000000000000000000000000000000000000000000004',
    size_bytes: 8500,
    is_dir: false,
    sync_state: 'downloading',
    download_progress: 68,
  },
];

export function App() {
  const [paths, setPaths] = useState<string[]>(['CS101-2026']);
  const [files, setFiles] = useState<FileEntry[]>(INITIAL_FILES);
  const [selectedHash, setSelectedHash] = useState<string | null>(null);
  const [currentView, setCurrentView] = useState<'explorer' | 'pdf' | 'markdown' | 'settings'>('explorer');
  const [searchQuery, setSearchQuery] = useState('');
  const [pdfData, setPdfData] = useState<Uint8Array | null>(null);

  const { stats, refetch: refetchStats } = useStorageStats();
  const { nodeId } = usePeerStatus();

  useEffect(() => {
    async function loadWorkspaceFiles() {
      try {
        const records = await invoke<Array<{ hash: string; size_bytes: number; is_pinned: boolean }>>('list_files');
        if (records && records.length > 0) {
          setFiles(
            records.map((r) => ({
              name: `Document-${r.hash.slice(0, 8)}.pdf`,
              hash: r.hash,
              size_bytes: r.size_bytes,
              is_dir: false,
              sync_state: r.is_pinned ? 'pinned' : 'cached',
            }))
          );
        }
      } catch {
        // Fallback to initial files if running in standalone browser preview
      }
    }
    loadWorkspaceFiles();
  }, []);

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
          // Fallback minimal PDF mock for development preview
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

  const handleDownload = (hash: string) => {
    setFiles((prev) =>
      prev.map((f) =>
        f.hash === hash ? { ...f, sync_state: 'downloading', download_progress: 15 } : f
      )
    );
    setTimeout(() => {
      setFiles((prev) =>
        prev.map((f) => (f.hash === hash ? { ...f, sync_state: 'cached' } : f))
      );
      refetchStats();
    }, 1500);
  };

  const handleFileUploaded = (hash: string) => {
    const newEntry: FileEntry = {
      name: `Uploaded-Document-${hash.slice(0, 8)}.pdf`,
      hash,
      size_bytes: 2048000,
      is_dir: false,
      sync_state: 'cached',
    };
    setFiles((prev) => [newEntry, ...prev]);
    refetchStats();
  };

  const filteredFiles = files.filter((f) =>
    f.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-gray-50 text-gray-900 select-none">
      {/* Top Header Bar */}
      <header className="h-12 bg-white border-b border-gray-200 px-4 flex items-center justify-between shadow-xs z-10">
        <div className="flex items-center gap-3">
          <div className="bg-blue-600 text-white p-1.5 rounded-lg shadow-xs">
            <FolderGit2 className="w-5 h-5" />
          </div>
          <div>
            <h1 className="font-bold text-sm tracking-tight text-gray-900">P2P Campus Workspace</h1>
            <p className="text-[10px] text-gray-500 font-mono truncate max-w-[280px]">
              {nodeId ? `Node: ${nodeId}` : 'Initializing Iroh Quic Engine...'}
            </p>
          </div>
        </div>

        {/* Global Search Bar */}
        <div className="relative w-72">
          <Search className="w-3.5 h-3.5 text-gray-400 absolute left-2.5 top-2.5" />
          <input
            type="text"
            placeholder="Search workspace files & hashes..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full text-xs pl-8 pr-3 py-1.5 bg-gray-100 border border-transparent rounded-md focus:bg-white focus:border-blue-500 focus:outline-none transition-all"
          />
        </div>

        {/* Action icons */}
        <div className="flex items-center gap-2">
          <button
            onClick={() => refetchStats()}
            className="p-1.5 rounded-md text-gray-600 hover:text-gray-900 hover:bg-gray-100 transition-colors"
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
                  Campus P2P Mesh
                </button>
                <button
                  onClick={() => setCurrentView('markdown')}
                  className={`w-full flex items-center gap-2.5 px-3 py-2 text-xs rounded-lg font-medium transition-colors ${
                    currentView === 'markdown'
                      ? 'bg-blue-50 text-blue-700'
                      : 'text-gray-700 hover:bg-gray-100'
                  }`}
                >
                  <FileText className="w-4 h-4 text-emerald-500" />
                  Collaborative Notes
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
                  <FileCode className="w-3.5 h-3.5 text-red-500" />
                  Slide Decks (.pdf)
                </button>
                <button
                  onClick={() => setSearchQuery('.md')}
                  className="w-full flex items-center gap-2.5 px-3 py-1.5 text-xs text-gray-600 rounded-lg hover:bg-gray-100"
                >
                  <FileText className="w-3.5 h-3.5 text-blue-500" />
                  Markdown (.md)
                </button>
              </div>
            </div>
          </div>

          <div className="px-3">
            <button
              onClick={() => setCurrentView('settings')}
              className="w-full flex items-center justify-between p-2 rounded-lg bg-gray-50 border border-gray-200 text-xs hover:bg-gray-100 transition-colors"
            >
              <div className="flex items-center gap-2">
                <Settings className="w-3.5 h-3.5 text-gray-600" />
                <span className="font-medium text-gray-700">Cache Slider</span>
              </div>
              <span className="text-[10px] font-bold text-blue-600 bg-blue-100 px-1.5 py-0.5 rounded">
                {stats?.full_mirror ? 'Mirror' : `${Math.round((stats?.quota_bytes ?? 5368709120) / (1024*1024*1024))}GB`}
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
                onSelectFile={handleSelectFile}
                onNavigateFolder={handleNavigateFolder}
                onPin={handlePin}
                onUnpin={handleUnpin}
                onDownload={handleDownload}
                onPreview={(hash) => {
                  const f = files.find((item) => item.hash === hash);
                  if (f) handleSelectFile(f);
                }}
                onFileUploaded={handleFileUploaded}
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

      {/* Live Telemetry Footer */}
      <TelemetryBar
        stats={stats}
        onOpenSettings={() => setCurrentView('settings')}
      />
    </div>
  );
}
export default App;
