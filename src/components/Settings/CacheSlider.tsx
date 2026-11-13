import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { 
  HardDrive, 
  ShieldCheck, 
  Zap, 
  FolderOpen, 
  Trash2, 
  Power, 
  Laptop,
  Database as DatabaseIcon,
  Sparkles
} from 'lucide-react';
import { StorageStats } from '../../types';

interface CacheSliderProps {
  stats: StorageStats | null;
  onStatsUpdated?: () => void;
}

function formatGigabytes(bytes: number): string {
  return (bytes / (1024 * 1024 * 1024)).toFixed(2);
}

export const CacheSlider: React.FC<CacheSliderProps> = ({ stats, onStatsUpdated }) => {
  // Storage Settings
  const [sliderGb, setSliderGb] = useState<number>(5);
  const [fullMirror, setFullMirror] = useState<boolean>(false);
  const [savingStorage, setSavingStorage] = useState<boolean>(false);

  // Autostart Settings (cross-platform Windows, Linux, macOS)
  const [autostartEnabled, setAutostartEnabled] = useState<boolean>(false);
  const [loadingAutostart, setLoadingAutostart] = useState<boolean>(false);

  // Cache Storage Directory
  const [cacheDir, setCacheDir] = useState<string>('');
  const [customCacheDirInput, setCustomCacheDirInput] = useState<string>('');
  const [savingCacheDir, setSavingCacheDir] = useState<boolean>(false);
  const [cacheDirFeedback, setCacheDirFeedback] = useState<{ msg: string; isError: boolean } | null>(null);

  // Public Swarm Zero-Trust Auto-Hosting Guard
  const [autoHostPublic, setAutoHostPublic] = useState<boolean>(false);
  const [savingAutoHost, setSavingAutoHost] = useState<boolean>(false);

  useEffect(() => {
    invoke<boolean>('is_autostart_enabled')
      .then((val) => setAutostartEnabled(Boolean(val)))
      .catch((err) => console.warn('Could not read autostart status:', err));

    invoke<string | null>('get_setting', { key: 'auto_host_public_files' })
      .then((val) => setAutoHostPublic(val === 'true'))
      .catch((err) => console.warn('Could not read auto_host_public_files:', err));

    invoke<string>('get_cache_directory')
      .then((dir) => {
        const safeDir = dir || '';
        setCacheDir(safeDir);
        setCustomCacheDirInput(safeDir);
      })
      .catch((err) => console.warn('Could not read cache directory:', err));
  }, []);

  const handleToggleAutoHost = async () => {
    const nextVal = !autoHostPublic;
    setSavingAutoHost(true);
    try {
      await invoke('set_auto_host_public_files', { enabled: nextVal });
      setAutoHostPublic(nextVal);
    } catch (err) {
      console.error('Failed to update auto_host_public_files:', err);
    } finally {
      setSavingAutoHost(false);
    }
  };

  const handleToggleAutostart = async () => {
    const nextVal = !autostartEnabled;
    setLoadingAutostart(true);
    try {
      await invoke('set_autostart_enabled', { enabled: nextVal });
      setAutostartEnabled(nextVal);
    } catch (err) {
      console.error('Failed to update autostart setting:', err);
    } finally {
      setLoadingAutostart(false);
    }
  };

  const handleSaveCacheDir = async (resetToDefault = false) => {
    setSavingCacheDir(true);
    setCacheDirFeedback(null);
    try {
      const pathToSet = resetToDefault ? '' : customCacheDirInput;
      const res = await invoke<string>('set_cache_directory', { path: pathToSet });
      const updatedDir = await invoke<string>('get_cache_directory');
      setCacheDir(updatedDir);
      setCustomCacheDirInput(updatedDir);
      setCacheDirFeedback({
        msg: resetToDefault
          ? 'Reset to default location. Restart app to apply.'
          : `Storage location updated to: ${res}. Restart app to apply.`,
        isError: false,
      });
    } catch (err) {
      setCacheDirFeedback({ msg: `Failed to update location: ${String(err)}`, isError: true });
    } finally {
      setSavingCacheDir(false);
    }
  };

  useEffect(() => {
    if (stats) {
      setFullMirror(stats.full_mirror);
      if (!stats.full_mirror && stats.quota_bytes < 1000 * 1024 * 1024 * 1024) {
        const gb = Math.round(stats.quota_bytes / (1024 * 1024 * 1024));
        setSliderGb(Math.max(3, Math.min(50, gb)));
      }
    }
  }, [stats]);

  const handleSliderChange = async (newGb: number) => {
    setSliderGb(newGb);
    setSavingStorage(true);
    try {
      const bytes = newGb * 1024 * 1024 * 1024;
      await invoke('set_cache_limit', { bytes, fullMirror: false });
      if (onStatsUpdated) onStatsUpdated();
    } catch (err) {
      console.error('Failed to update cache limit:', err);
    } finally {
      setSavingStorage(false);
    }
  };

  const handleToggleFullMirror = async () => {
    const nextMirror = !fullMirror;
    setFullMirror(nextMirror);
    setSavingStorage(true);
    try {
      await invoke('set_cache_limit', {
        bytes: nextMirror ? null : sliderGb * 1024 * 1024 * 1024,
        fullMirror: nextMirror,
      });
      if (onStatsUpdated) onStatsUpdated();
    } catch (err) {
      console.error('Failed to toggle full mirror:', err);
    } finally {
      setSavingStorage(false);
    }
  };

  const handlePurgeCache = async () => {
    if (confirm('Are you sure you want to purge unpinned cached blobs? Pinned files and cloud files will remain safe.')) {
      try {
        const freed = await invoke<number>('purge_unpinned_cache');
        const freedMb = (freed / (1024 * 1024)).toFixed(1);
        alert(`Purged unpinned cache! Freed ${freedMb} MB.`);
        if (onStatsUpdated) onStatsUpdated();
      } catch (err) {
        alert(`Failed to purge cache: ${err}`);
      }
    }
  };

  const handleOpenDataDir = async () => {
    try {
      await invoke('open_data_dir');
    } catch (err) {
      alert(`Failed to open data directory: ${err}`);
    }
  };

  const [optimizingDb, setOptimizingDb] = useState<boolean>(false);
  const [dbOptimizedMsg, setDbOptimizedMsg] = useState<string | null>(null);

  const handleOptimizeDb = async () => {
    setOptimizingDb(true);
    setDbOptimizedMsg(null);
    try {
      const newSizeBytes = await invoke<number>('optimize_database');
      const mb = (newSizeBytes / (1024 * 1024)).toFixed(1);
      setDbOptimizedMsg(`Database optimized! WAL flushed and truncated. Total DB footprint: ${mb} MB.`);
      if (onStatsUpdated) onStatsUpdated();
    } catch (err) {
      alert(`Database optimization failed: ${err}`);
    } finally {
      setOptimizingDb(false);
    }
  };

  const formatMb = (bytes?: number) => {
    if (!bytes || bytes === 0) return '0.0 MB';
    return (bytes / (1024 * 1024)).toFixed(2) + ' MB';
  };

  const targetHostGb = (sliderGb * 0.8).toFixed(1);

  return (
    <div className="max-w-3xl mx-auto space-y-6">
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-6">
        <div>
          <h2 className="text-base font-bold text-gray-900">Host-to-Play Cache Allocation</h2>
          <p className="text-xs text-gray-500 mt-1">
            Dedicating local disk space seeds course materials to classmates on campus.
          </p>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="p-4 bg-gray-50 rounded-lg border border-gray-100">
            <div className="text-xs font-medium text-gray-500">Currently Used</div>
            <div className="text-2xl font-bold text-gray-900 mt-1">{formatGigabytes(stats?.used_bytes || 0)} GB</div>
            <div className="text-[11px] text-gray-400 mt-0.5">
              ~{((stats?.used_bytes || 0) / 1_000_000_000).toFixed(2)} decimal GB (binary {formatGigabytes(stats?.used_bytes || 0)} GiB)
            </div>
          </div>
          <div className="p-4 bg-gray-50 rounded-lg border border-gray-100">
            <div className="text-xs font-medium text-gray-500">Pinned (Protected)</div>
            <div className="text-2xl font-bold text-green-600 mt-1">{formatGigabytes(stats?.pinned_bytes || 0)} GB</div>
            <div className="text-[11px] text-gray-400 mt-0.5">
              ~{((stats?.pinned_bytes || 0) / 1_000_000_000).toFixed(2)} decimal GB (binary {formatGigabytes(stats?.pinned_bytes || 0)} GiB)
            </div>
          </div>
        </div>

        {/* Active 80% Swarm Hosting Banner */}
        <div className="p-4 bg-blue-50/70 border border-blue-200 rounded-lg">
          <div className="flex items-center gap-2 text-xs font-semibold text-blue-900">
            <Zap className="w-4 h-4 text-blue-600" />
            <span>Active Cooperative Swarm Seeding (~80% target)</span>
          </div>
          <p className="text-[11px] text-blue-700 mt-1 leading-relaxed">
            Your node actively pre-fetches and hosts course files up to <strong>~{targetHostGb} GB</strong> (80% of quota). 
            This ensures unpinned files are seeded across campus so classmates can download them at LAN speeds.
          </p>
        </div>

        <div className="space-y-4 pt-2">
          <div className="flex justify-between items-center text-xs">
            <label htmlFor="cache-slider" className="font-semibold text-gray-700">Cache Quota Limit</label>
            <span className="font-bold text-blue-600 text-sm">{sliderGb} GB</span>
          </div>

          <input
            id="cache-slider"
            type="range"
            min="3"
            max="50"
            value={sliderGb}
            disabled={fullMirror || savingStorage}
            onChange={(e) => handleSliderChange(parseInt(e.target.value, 10))}
            className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-blue-600 disabled:opacity-50"
          />

          <div className="flex justify-between text-[11px] text-gray-400">
            <span>Min: 3 GB</span>
            <span>Recommended: 5 GB</span>
            <span>50 GB</span>
          </div>
        </div>

        <div className="p-4 bg-gray-50 rounded-lg border border-gray-200 flex items-start gap-3">
          <input
            type="checkbox"
            id="full-mirror"
            checked={fullMirror}
            disabled={savingStorage}
            onChange={handleToggleFullMirror}
            className="mt-1 h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
          />
          <label htmlFor="full-mirror" className="cursor-pointer">
            <div className="flex items-center gap-1.5 text-xs font-bold text-gray-900">
              <Zap className="w-3.5 h-3.5 text-amber-500 fill-amber-500" />
              Enable Full Mirror Mode
            </div>
            <p className="text-[11px] text-gray-500 mt-0.5">
              Disables the automatic LRU eviction worker and auto-downloads all course workspace blobs locally to provide 100% offline redundancy.
            </p>
          </label>
        </div>

        <div className="p-4 bg-gray-50 rounded-lg border border-gray-200 flex items-start gap-3">
          <input
            type="checkbox"
            id="auto-host-public"
            checked={autoHostPublic}
            disabled={savingAutoHost}
            onChange={handleToggleAutoHost}
            className="mt-1 h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500 cursor-pointer"
          />
          <label htmlFor="auto-host-public" className="cursor-pointer">
            <div className="flex items-center gap-1.5 text-xs font-bold text-gray-900">
              <ShieldCheck className="w-3.5 h-3.5 text-emerald-600" />
              Auto-Seed Public Swarm Files (Zero-Trust Guard)
            </div>
            <p className="text-[11px] text-gray-500 mt-0.5 leading-relaxed">
              When disabled (recommended for public use), background cache auto-fill and scarcity replication only run in password-protected cohorts. In public swarms, files are only cached when you explicitly open, download, or pin them to prevent liability from unvetted content.
            </p>
          </label>
        </div>

        <div className="flex items-center gap-2 text-xs text-gray-600">
          <ShieldCheck className="w-4 h-4 text-green-600 shrink-0" />
          <span>Oldest unpinned items are automatically evicted when cache exceeds 90% capacity.</span>
        </div>

        {/* PC Startup & Background Behavior */}
        <div className="p-4 bg-gray-50 rounded-xl border border-gray-200 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-start gap-3">
              <div className="w-8 h-8 rounded-lg bg-indigo-50 text-indigo-600 flex items-center justify-center border border-indigo-200 shrink-0 mt-0.5">
                <Power className="w-4 h-4" />
              </div>
              <div>
                <div className="text-xs font-bold text-gray-900">Start on System Boot</div>
                <p className="text-[11px] text-gray-500 mt-0.5 leading-relaxed">
                  Automatically launch P2P Campus Workspace in the background on computer startup (Windows, Linux, macOS) so course materials continue seeding to classmates.
                </p>
              </div>
            </div>
            <label className="relative inline-flex items-center cursor-pointer shrink-0 ml-4">
              <input
                type="checkbox"
                aria-label="Start on System Boot"
                checked={autostartEnabled}
                disabled={loadingAutostart}
                onChange={handleToggleAutostart}
                className="sr-only peer"
              />
              <div className="w-9 h-5 bg-gray-300 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-600"></div>
            </label>
          </div>
          <div className="text-[10px] text-gray-400 pl-11 flex items-center gap-1.5">
            <Laptop className="w-3 h-3 text-gray-500" />
            <span>Platform support: Windows (Registry Run), Linux (XDG Autostart), macOS (LaunchAgent).</span>
          </div>
        </div>

        {/* Cache Storage Location */}
        <div className="p-4 bg-gray-50 rounded-xl border border-gray-200 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <HardDrive className="w-4 h-4 text-gray-700" />
              <span className="text-xs font-bold text-gray-900">Cache Storage Location</span>
            </div>
            <span className="text-[10px] font-mono bg-gray-200 text-gray-700 px-2 py-0.5 rounded">
              Active Storage Path
            </span>
          </div>
          <p className="text-[11px] text-gray-500 leading-relaxed">
            Configure where cached course files and seeding chunks are stored on disk (e.g. on a secondary drive or external SSD).
          </p>
          <div className="space-y-2">
            <div className="flex gap-2">
              <input
                type="text"
                value={customCacheDirInput}
                onChange={(e) => setCustomCacheDirInput(e.target.value)}
                placeholder="e.g. D:\p2pstud-cache"
                className="flex-1 bg-white border border-gray-300 rounded-lg px-3 py-1.5 text-xs font-mono text-gray-800 placeholder-gray-400 focus:outline-none focus:border-blue-500 shadow-2xs"
              />
              <button
                onClick={() => handleSaveCacheDir(false)}
                disabled={savingCacheDir || !(customCacheDirInput || '').trim() || (customCacheDirInput || '').trim() === cacheDir}
                className="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded-lg text-xs font-semibold transition cursor-pointer shadow-2xs"
              >
                {savingCacheDir ? 'Saving...' : 'Save Location'}
              </button>
              <button
                onClick={() => handleSaveCacheDir(true)}
                disabled={savingCacheDir}
                className="px-2.5 py-1.5 bg-gray-200 hover:bg-gray-300 text-gray-700 rounded-lg text-xs font-medium transition cursor-pointer"
                title="Reset to default AppData path"
              >
                Reset Default
              </button>
            </div>
            {cacheDirFeedback && (
              <div className={`p-2 rounded text-[11px] font-medium ${cacheDirFeedback.isError ? 'bg-red-50 text-red-700 border border-red-200' : 'bg-emerald-50 text-emerald-800 border border-emerald-200'}`}>
                {cacheDirFeedback.msg}
              </div>
            )}
          </div>
        </div>

        {/* Database & Journal Storage Footprint */}
        <div className="p-4 bg-gray-50 rounded-xl border border-gray-200 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <DatabaseIcon className="w-4 h-4 text-indigo-600" />
              <span className="text-xs font-bold text-gray-900">Database & Journal Storage</span>
            </div>
            <span className="text-[10px] font-mono bg-indigo-50 text-indigo-700 border border-indigo-200 px-2 py-0.5 rounded">
              SQLite WAL
            </span>
          </div>
          <p className="text-[11px] text-gray-500 leading-relaxed">
            Index metadata, peer discovery timestamps, and tombstone records. Background checkpointing automatically truncates journal files so WAL never inflates disk consumption.
          </p>
          <div className="flex flex-wrap items-center justify-between gap-3 pt-1">
            <div className="flex items-center gap-4 text-xs font-mono text-gray-700">
              <div>
                <span className="text-gray-400 text-[10px] block font-sans">Database File</span>
                <strong>{formatMb(stats?.db_bytes)}</strong>
              </div>
              <div className="h-6 w-px bg-gray-200"></div>
              <div>
                <span className="text-gray-400 text-[10px] block font-sans">WAL Journal</span>
                <strong className={stats?.wal_bytes && stats.wal_bytes > 50 * 1024 * 1024 ? "text-amber-600 font-bold" : "text-gray-700"}>
                  {formatMb(stats?.wal_bytes)}
                </strong>
              </div>
            </div>
            <button
              onClick={handleOptimizeDb}
              disabled={optimizingDb}
              className="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white rounded-lg text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer shadow-2xs"
            >
              <Sparkles className="w-3.5 h-3.5" />
              {optimizingDb ? 'Optimizing...' : 'Flush WAL & Optimize Database'}
            </button>
          </div>
          {dbOptimizedMsg && (
            <div className="p-2 rounded text-[11px] font-medium bg-emerald-50 text-emerald-800 border border-emerald-200">
              {dbOptimizedMsg}
            </div>
          )}
        </div>

        {/* Disk Actions */}
        <div className="pt-4 border-t border-gray-100 flex flex-wrap gap-3">
          <button
            onClick={handleOpenDataDir}
            className="px-3 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-colors"
          >
            <FolderOpen className="w-3.5 h-3.5 text-gray-600" />
            Open Data Folder
          </button>

          <button
            onClick={handlePurgeCache}
            className="px-3 py-2 bg-red-50 hover:bg-red-100 text-red-700 rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-colors"
          >
            <Trash2 className="w-3.5 h-3.5 text-red-600" />
            Purge Unpinned Cache
          </button>
        </div>
      </div>
    </div>
  );
};
