import { useState } from 'react';
import { Archive, Download, ShieldAlert, CheckCircle2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';

export function BackupSettings() {
  const [isExporting, setIsExporting] = useState<boolean>(false);
  const [exportMessage, setExportMessage] = useState<string | null>(null);
  const [isSuccess, setIsSuccess] = useState<boolean>(false);
  const [destPath, setDestPath] = useState<string>('p2pstud-backup.zip');

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

  return (
    <div className="space-y-6">
      <div className="bg-white border border-gray-200 rounded-xl p-5 space-y-4 shadow-xs">
        <div className="flex items-start gap-3">
          <div className="w-10 h-10 rounded-lg bg-blue-50 text-blue-600 flex items-center justify-center border border-blue-200 shrink-0">
            <Archive className="w-5 h-5" />
          </div>
          <div>
            <h3 className="text-sm font-bold text-gray-900">Full App Data Backup</h3>
            <p className="text-xs text-gray-600 mt-1 leading-relaxed">
              Creates a lightweight ZIP archive containing your private Ed25519 identity key (<code className="text-blue-700 bg-blue-50 px-1 py-0.5 rounded font-mono text-[11px]">iroh_secret.key</code>) and workspace catalog (<code className="text-blue-700 bg-blue-50 px-1 py-0.5 rounded font-mono text-[11px]">p2pstud.db</code>).
            </p>
          </div>
        </div>

        <div className="bg-amber-50 border border-amber-200 rounded-lg p-3 flex items-center gap-2.5 text-xs text-amber-900">
          <ShieldAlert className="w-4 h-4 text-amber-600 shrink-0" />
          <span className="font-medium">Keep your backup file secure: it contains your cryptographic node identity key.</span>
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
              placeholder="e.g. p2pstud-backup.zip"
              className="flex-1 bg-white border border-gray-300 rounded-lg px-3 py-1.5 text-xs text-gray-900 placeholder-gray-400 focus:outline-none focus:border-blue-500 shadow-2xs"
            />
            <button
              onClick={handleExportBackup}
              disabled={isExporting || !destPath.trim()}
              className="px-4 py-1.5 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded-lg text-xs font-semibold flex items-center gap-1.5 shadow-2xs transition cursor-pointer"
            >
              <Download className="w-3.5 h-3.5" />
              <span>{isExporting ? 'Exporting...' : 'Export Backup'}</span>
            </button>
          </div>
          <p className="text-[11px] text-gray-400">
            Relative file names are automatically saved to your system Downloads folder.
          </p>
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
    </div>
  );
}
