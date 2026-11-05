import React from 'react';
import { PackageOpen, CheckCircle2, AlertCircle, X, HardDrive, Loader2 } from 'lucide-react';
import { ImportProgress } from '../types';

interface ImportingBannerProps {
  progress: ImportProgress | null;
  onDismiss: () => void;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

function formatSpeed(bytesPerSec: number): string {
  if (!bytesPerSec || bytesPerSec <= 0) return '0.0 KB/s';
  if (bytesPerSec < 1024 * 1024) {
    return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  }
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
}

export const ImportingBanner: React.FC<ImportingBannerProps> = ({ progress, onDismiss }) => {
  if (!progress) return null;

  const isCompleted = progress.status === 'completed';
  const isError = progress.status === 'error';
  const isImporting = progress.status === 'importing';

  const percent = progress.total_bytes > 0
    ? Math.min(100, Math.round((progress.bytes_processed / progress.total_bytes) * 100))
    : progress.total_entries > 0
    ? Math.min(100, Math.round((progress.current_entry / progress.total_entries) * 100))
    : 0;

  return (
    <div
      role="status"
      aria-live="polite"
      className={`border-b px-4 py-2.5 transition-all duration-300 shadow-2xs select-none ${
        isCompleted
          ? 'bg-emerald-50 border-emerald-200 text-emerald-900'
          : isError
          ? 'bg-red-50 border-red-200 text-red-900'
          : 'bg-indigo-50 border-indigo-200 text-indigo-950'
      }`}
    >
      <div className="flex items-center justify-between gap-4">
        {/* Left: Icon & status title */}
        <div className="flex items-center gap-2.5 min-w-0">
          {isCompleted ? (
            <CheckCircle2 className="w-4 h-4 text-emerald-600 shrink-0" />
          ) : isError ? (
            <AlertCircle className="w-4 h-4 text-red-600 shrink-0" />
          ) : (
            <div className="relative shrink-0 flex items-center justify-center">
              <PackageOpen className="w-4 h-4 text-indigo-600 animate-pulse" />
              <Loader2 className="w-4 h-4 text-indigo-400 absolute animate-spin opacity-40" />
            </div>
          )}

          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span className="font-semibold text-xs tracking-tight">
                {isCompleted
                  ? `Import Complete: ${progress.archive_name}`
                  : isError
                  ? `Import Failed: ${progress.archive_name}`
                  : `Importing: ${progress.archive_name}`}
              </span>
              {isImporting && (
                <span className="text-[10px] bg-indigo-100 text-indigo-700 px-1.5 py-0.5 rounded font-mono font-medium">
                  {percent}%
                </span>
              )}
            </div>

            <div className="flex items-center gap-3 text-[11px] text-gray-600 mt-0.5 flex-wrap">
              {isImporting && (
                <>
                  <span className="flex items-center gap-1 font-mono font-semibold text-indigo-800">
                    <HardDrive className="w-3 h-3 text-indigo-500" />
                    {formatSpeed(progress.bytes_per_sec)} (local disk)
                  </span>
                  <span>•</span>
                  <span>
                    {progress.total_entries > 1
                      ? `${progress.current_entry} / ${progress.total_entries} entries`
                      : 'Indexing file'}
                  </span>
                  {progress.total_bytes > 0 && (
                    <>
                      <span>•</span>
                      <span>
                        {formatBytes(progress.bytes_processed)} / {formatBytes(progress.total_bytes)}
                      </span>
                    </>
                  )}
                  {progress.current_file && (
                    <>
                      <span>•</span>
                      <span className="truncate max-w-[200px] text-gray-500 italic" title={progress.current_file}>
                        {progress.current_file}
                      </span>
                    </>
                  )}
                </>
              )}
              {isCompleted && (
                <span>
                  {progress.total_entries > 0 ? `${progress.total_entries} files` : 'Files'} successfully imported and verified into local storage repository.
                </span>
              )}
              {isError && (
                <span className="text-red-700 font-medium">
                  {progress.error || 'Extraction error occurred'}
                </span>
              )}
            </div>
          </div>
        </div>

        {/* Right: Actions */}
        <div className="flex items-center gap-2 shrink-0">
          <button
            onClick={onDismiss}
            className={`p-1 rounded transition-colors cursor-pointer ${
              isCompleted
                ? 'hover:bg-emerald-100 text-emerald-700'
                : isError
                ? 'hover:bg-red-100 text-red-700'
                : 'hover:bg-indigo-100 text-indigo-700'
            }`}
            title="Dismiss import bar"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Progress Bar (Voortgangsbalk) */}
      {isImporting && (
        <div className="w-full bg-indigo-200/80 rounded-full h-1.5 overflow-hidden mt-2">
          <div
            className="bg-indigo-600 h-1.5 rounded-full transition-all duration-150 ease-out"
            style={{ width: `${percent}%` }}
          />
        </div>
      )}
    </div>
  );
};

