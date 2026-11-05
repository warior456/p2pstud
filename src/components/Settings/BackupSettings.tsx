import { useState, useEffect } from 'react';
import { 
  Archive, 
  Download, 
  ShieldAlert, 
  CheckCircle2, 
  Upload, 
  RefreshCw, 
  Sparkles, 
  FolderSearch,
  AlertTriangle 
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { UpdateInfo } from '../../types';

export function BackupSettings() {
  // Export states
  const [isExporting, setIsExporting] = useState<boolean>(false);
  const [exportMessage, setExportMessage] = useState<string | null>(null);
  const [isSuccess, setIsSuccess] = useState<boolean>(false);
  const [destPath, setDestPath] = useState<string>('p2pstud-backup.zip');

  // Restore states
  const [restorePath, setRestorePath] = useState<string>('');
  const [isRestoring, setIsRestoring] = useState<boolean>(false);
  const [restoreMessage, setRestoreMessage] = useState<string | null>(null);
  const [restoreSuccess, setRestoreSuccess] = useState<boolean>(false);

  // Update states
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [swarmMedian, setSwarmMedian] = useState<string | null>(null);
  const [isCheckingUpdate, setIsCheckingUpdate] = useState<boolean>(false);
  const [isDownloadingUpdate, setIsDownloadingUpdate] = useState<boolean>(false);
  const [updateActionMsg, setUpdateActionMsg] = useState<string | null>(null);

  const fetchUpdateStatus = async () => {
    try {
      const info = await invoke<UpdateInfo>('check_app_update');
      setUpdateInfo(info);
    } catch (err) {
      console.warn('Could not check update status:', err);
    }
    try {
      const swarmInfo = await invoke<any>('get_swarm_version_info');
      if (swarmInfo?.median_version) {
        setSwarmMedian(swarmInfo.median_version);
      }
    } catch (_) {}
  };

  useEffect(() => {
    fetchUpdateStatus();
  }, []);

  const handleBrowseExportPath = async () => {
    try {
      const chosen = await invoke<string | null>('pick_backup_save_dialog');
      if (chosen) {
        setDestPath(chosen);
      }
    } catch (err) {
      console.error('Failed to open save dialog:', err);
    }
  };

  const handleExportBackup = async () => {
    setIsExporting(true);
    setExportMessage(null);
    try {
      const savedPath = await invoke<string>('export_app_backup', { destPath });
      setExportMessage(`Backup successfully saved to: ${savedPath}`);
      setIsSuccess(true);
    } catch (err) {
      setExportMessage(`Export failed: ${String(err)}`);
      setIsSuccess(false);
    } finally {
      setIsExporting(false);
    }
  };

  const handleBrowseRestorePath = async () => {
    try {
      const chosen = await invoke<string | null>('pick_backup_restore_dialog');
      if (chosen) {
        setRestorePath(chosen);
      }
    } catch (err) {
      console.error('Failed to open restore dialog:', err);
    }
  };

  const handleRestoreBackup = async () => {
    if (!restorePath.trim()) return;
    if (
      !confirm(
        'Warning: Restoring will overwrite your current workspace database and identity with the backup. A safety copy of your current database will be preserved. Do you wish to proceed?'
      )
    ) {
      return;
    }

    setIsRestoring(true);
    setRestoreMessage(null);
    try {
      const msg = await invoke<string>('restore_app_backup', { filePath: restorePath.trim() });
      setRestoreMessage(msg);
      setRestoreSuccess(true);
    } catch (err) {
      setRestoreMessage(`Restore failed: ${String(err)}`);
      setRestoreSuccess(false);
    } finally {
      setIsRestoring(false);
    }
  };

  const handleCheckUpdateNow = async () => {
    setIsCheckingUpdate(true);
    setUpdateActionMsg(null);
    try {
      const info = await invoke<UpdateInfo>('check_app_update');
      setUpdateInfo(info);
      if (!info.update_available) {
        setUpdateActionMsg(`You are up to date! Version ${info.current_version} is the latest available.`);
      }
    } catch (err) {
      setUpdateActionMsg(`Update check failed: ${String(err)}`);
    } finally {
      setIsCheckingUpdate(false);
    }
  };

  const handleDownloadUpdate = async () => {
    setIsDownloadingUpdate(true);
    try {
      const msg = await invoke<string>('download_app_update');
      setUpdateActionMsg(msg);
      await fetchUpdateStatus();
    } catch (err) {
      setUpdateActionMsg(`Download failed: ${String(err)}`);
    } finally {
      setIsDownloadingUpdate(false);
    }
  };

  const handleInstallAndRestart = async () => {
    try {
      await invoke('install_app_update');
    } catch (err) {
      alert(`Install and restart failed: ${err}`);
    }
  };

  return (
    <div className="space-y-6">
      {/* Software Updates Section */}
      <div className="bg-white border border-gray-200 rounded-xl p-5 space-y-4 shadow-xs">
        <div className="flex items-start justify-between">
          <div className="flex items-start gap-3">
            <div className="w-10 h-10 rounded-lg bg-indigo-50 text-indigo-600 flex items-center justify-center border border-indigo-200 shrink-0">
              <Sparkles className="w-5 h-5" />
            </div>
            <div>
              <h3 className="text-sm font-bold text-gray-900">Software Updates</h3>
              <p className="text-xs text-gray-600 mt-1 leading-relaxed">
                P2P Campus Workspace checks for non-intrusive updates in the background. Updates are prompted strictly on startup or shutdown to prevent study interruptions.
              </p>
            </div>
          </div>
          <button
            onClick={handleCheckUpdateNow}
            disabled={isCheckingUpdate}
            className="px-3 py-1.5 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer disabled:opacity-50"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isCheckingUpdate ? 'animate-spin' : ''}`} />
            <span>{isCheckingUpdate ? 'Checking...' : 'Check for Updates'}</span>
          </button>
        </div>

        <div className="flex items-center gap-3 text-xs text-gray-700 bg-gray-50 p-3 rounded-lg border border-gray-100 flex-wrap">
          <span>Current Installed Version: <strong className="font-mono font-semibold text-gray-900">{updateInfo?.current_version || '...'}</strong></span>
          {swarmMedian && (
            <>
              <span>•</span>
              <span>Swarm Median: <strong className="font-mono font-semibold text-blue-700">v{swarmMedian}</strong></span>
            </>
          )}
          <span>•</span>
          <span>Latest Available: <strong className="font-mono font-semibold text-gray-900">{updateInfo?.version || '...'}</strong></span>
        </div>

        {updateInfo?.update_available && (
          <div className="bg-indigo-50/80 border border-indigo-200 rounded-lg p-3.5 space-y-2">
            <div className="flex items-center justify-between">
              <div className="text-xs font-bold text-indigo-950">
                🚀 New Version Available: {updateInfo.version}
              </div>
              <span className="text-[10px] font-mono bg-indigo-100 text-indigo-800 px-2 py-0.5 rounded">
                {(updateInfo.download_size_bytes / (1024 * 1024)).toFixed(1)} MB
              </span>
            </div>
            <p className="text-xs text-indigo-900 leading-relaxed">
              {updateInfo.notes}
            </p>
            <div className="pt-1 flex gap-2">
              {updateInfo.is_pending ? (
                <button
                  onClick={handleInstallAndRestart}
                  className="px-3.5 py-1.5 bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg text-xs font-semibold transition cursor-pointer"
                >
                  Restart & Install Now
                </button>
              ) : (
                <button
                  onClick={handleDownloadUpdate}
                  disabled={isDownloadingUpdate}
                  className="px-3.5 py-1.5 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white rounded-lg text-xs font-semibold transition cursor-pointer"
                >
                  {isDownloadingUpdate ? 'Downloading...' : 'Update on Next Restart'}
                </button>
              )}
            </div>
          </div>
        )}

        {updateActionMsg && (
          <div className="p-3 bg-gray-50 border border-gray-200 rounded-lg text-xs text-gray-800 font-medium">
            {updateActionMsg}
          </div>
        )}
      </div>

      {/* Full App Data Backup Export */}
      <div className="bg-white border border-gray-200 rounded-xl p-5 space-y-4 shadow-xs">
        <div className="flex items-start gap-3">
          <div className="w-10 h-10 rounded-lg bg-blue-50 text-blue-600 flex items-center justify-center border border-blue-200 shrink-0">
            <Archive className="w-5 h-5" />
          </div>
          <div>
            <h3 className="text-sm font-bold text-gray-900">Export Workspace Backup</h3>
            <p className="text-xs text-gray-600 mt-1 leading-relaxed">
              Creates a secure ZIP archive containing your private Ed25519 node identity key (<code className="text-blue-700 bg-blue-50 px-1 py-0.5 rounded font-mono text-[11px]">iroh_secret.key</code>) and workspace catalog (<code className="text-blue-700 bg-blue-50 px-1 py-0.5 rounded font-mono text-[11px]">p2pstud.db</code>).
            </p>
          </div>
        </div>

        <div className="bg-amber-50 border border-amber-200 rounded-lg p-3 flex items-center gap-2.5 text-xs text-amber-900">
          <ShieldAlert className="w-4 h-4 text-amber-600 shrink-0" />
          <span className="font-medium">Keep your backup file secure: it contains your cryptographic identity key.</span>
        </div>

        <div className="space-y-2">
          <label className="block text-xs font-semibold text-gray-700">
            Destination Backup File Path
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={destPath}
              onChange={(e) => setDestPath(e.target.value)}
              placeholder="e.g. D:\Backups\p2pstud-backup.zip"
              className="flex-1 bg-white border border-gray-300 rounded-lg px-3 py-1.5 text-xs text-gray-900 placeholder-gray-400 focus:outline-none focus:border-blue-500 shadow-2xs font-mono"
            />
            <button
              onClick={handleBrowseExportPath}
              className="px-3 py-1.5 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg text-xs font-medium flex items-center gap-1 transition cursor-pointer border border-gray-200"
              title="Browse destination folder"
            >
              <FolderSearch className="w-3.5 h-3.5" />
              <span>Browse...</span>
            </button>
            <button
              onClick={handleExportBackup}
              disabled={isExporting || !destPath.trim()}
              className="px-4 py-1.5 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded-lg text-xs font-semibold flex items-center gap-1.5 shadow-2xs transition cursor-pointer"
            >
              <Download className="w-3.5 h-3.5" />
              <span>{isExporting ? 'Exporting...' : 'Export Backup'}</span>
            </button>
          </div>
        </div>

        {exportMessage && (
          <div
            className={`p-3 rounded-lg text-xs flex items-center gap-2 font-medium ${
              isSuccess
                ? 'bg-emerald-50 text-emerald-800 border border-emerald-200'
                : 'bg-red-50 text-red-800 border border-red-200'
            }`}
          >
            {isSuccess && <CheckCircle2 className="w-4 h-4 text-emerald-600 shrink-0" />}
            <span>{exportMessage}</span>
          </div>
        )}
      </div>

      {/* Restore Workspace Backup */}
      <div className="bg-white border border-gray-200 rounded-xl p-5 space-y-4 shadow-xs">
        <div className="flex items-start gap-3">
          <div className="w-10 h-10 rounded-lg bg-emerald-50 text-emerald-600 flex items-center justify-center border border-emerald-200 shrink-0">
            <Upload className="w-5 h-5" />
          </div>
          <div>
            <h3 className="text-sm font-bold text-gray-900">Restore Workspace Backup</h3>
            <p className="text-xs text-gray-600 mt-1 leading-relaxed">
              Restore your catalog database and identity key from an existing <code className="text-emerald-700 bg-emerald-50 px-1 py-0.5 rounded font-mono text-[11px]">.zip</code> backup archive.
            </p>
          </div>
        </div>

        <div className="space-y-2">
          <label className="block text-xs font-semibold text-gray-700">
            Backup ZIP Archive Location
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={restorePath}
              onChange={(e) => setRestorePath(e.target.value)}
              placeholder="Select a p2pstud backup ZIP archive..."
              className="flex-1 bg-white border border-gray-300 rounded-lg px-3 py-1.5 text-xs text-gray-900 placeholder-gray-400 focus:outline-none focus:border-blue-500 shadow-2xs font-mono"
            />
            <button
              onClick={handleBrowseRestorePath}
              className="px-3 py-1.5 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg text-xs font-medium flex items-center gap-1 transition cursor-pointer border border-gray-200"
            >
              <FolderSearch className="w-3.5 h-3.5" />
              <span>Select File...</span>
            </button>
            <button
              onClick={handleRestoreBackup}
              disabled={isRestoring || !restorePath.trim()}
              className="px-4 py-1.5 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white rounded-lg text-xs font-semibold flex items-center gap-1.5 shadow-2xs transition cursor-pointer"
            >
              <Upload className="w-3.5 h-3.5" />
              <span>{isRestoring ? 'Restoring...' : 'Restore Backup'}</span>
            </button>
          </div>
        </div>

        {restoreMessage && (
          <div
            className={`p-3 rounded-lg text-xs flex items-center gap-2 font-medium ${
              restoreSuccess
                ? 'bg-emerald-50 text-emerald-800 border border-emerald-200'
                : 'bg-red-50 text-red-800 border border-red-200'
            }`}
          >
            {restoreSuccess ? (
              <CheckCircle2 className="w-4 h-4 text-emerald-600 shrink-0" />
            ) : (
              <AlertTriangle className="w-4 h-4 text-red-600 shrink-0" />
            )}
            <span>{restoreMessage}</span>
          </div>
        )}
      </div>
    </div>
  );
}
