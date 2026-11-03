import React, { useState } from 'react';
import { UploadCloud, CheckCircle2, AlertCircle, Loader2, ChevronDown, ChevronUp, X } from 'lucide-react';
import { UploadItem } from '../../types';

interface UploadQueueDrawerProps {
  queue: UploadItem[];
  onDismiss: (id: string) => void;
  onClearAll: () => void;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

export const UploadQueueDrawer: React.FC<UploadQueueDrawerProps> = ({
  queue,
  onDismiss,
  onClearAll,
}) => {
  const [isCollapsed, setIsCollapsed] = useState(false);

  if (queue.length === 0) return null;

  const activeCount = queue.filter((item) => item.status === 'reading' || item.status === 'uploading').length;
  const errorCount = queue.filter((item) => item.status === 'error').length;
  const completedCount = queue.filter((item) => item.status === 'completed').length;

  return (
    <div className="fixed bottom-10 right-6 z-50 w-96 max-w-[calc(100vw-2rem)] bg-white rounded-xl shadow-2xl border border-gray-200 overflow-hidden transition-all duration-200 animate-in slide-in-from-bottom-4">
      {/* Drawer Header */}
      <div className="flex items-center justify-between px-4 py-3 bg-gray-900 text-white cursor-pointer select-none" onClick={() => setIsCollapsed(!isCollapsed)}>
        <div className="flex items-center gap-2.5">
          <div className="relative">
            <UploadCloud className="w-5 h-5 text-blue-400" />
            {activeCount > 0 && (
              <span className="absolute -top-1 -right-1 flex h-2.5 w-2.5">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75"></span>
                <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-blue-500"></span>
              </span>
            )}
          </div>
          <div>
            <h4 className="text-xs font-semibold">
              {activeCount > 0
                ? `Uploading ${activeCount} ${activeCount === 1 ? 'file' : 'files'}...`
                : errorCount > 0
                ? `Upload completed with ${errorCount} errors`
                : `${completedCount} ${completedCount === 1 ? 'upload' : 'uploads'} completed`}
            </h4>
            <p className="text-[10px] text-gray-400">
              {activeCount > 0 ? 'Broadcasting directly to campus swarm' : 'All files ready in shared workspace'}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-1.5" onClick={(e) => e.stopPropagation()}>
          <button
            onClick={() => setIsCollapsed(!isCollapsed)}
            className="p-1 text-gray-400 hover:text-white rounded hover:bg-gray-800 transition-colors"
            title={isCollapsed ? 'Expand' : 'Collapse'}
          >
            {isCollapsed ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
          </button>
          <button
            onClick={onClearAll}
            className="p-1 text-gray-400 hover:text-white rounded hover:bg-gray-800 transition-colors"
            title="Dismiss all"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Drawer Body */}
      {!isCollapsed && (
        <div className="max-h-64 overflow-y-auto divide-y divide-gray-100 bg-gray-50/60 p-2">
          {queue.map((item) => (
            <div
              key={item.id}
              className="p-2.5 bg-white rounded-lg shadow-xs mb-1.5 border border-gray-100 flex flex-col gap-1.5 transition-all"
            >
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 overflow-hidden">
                  {item.status === 'reading' || item.status === 'uploading' ? (
                    <Loader2 className="w-4 h-4 text-blue-600 animate-spin shrink-0" />
                  ) : item.status === 'completed' ? (
                    <CheckCircle2 className="w-4 h-4 text-emerald-600 shrink-0" />
                  ) : (
                    <AlertCircle className="w-4 h-4 text-red-500 shrink-0" />
                  )}
                  <span className="text-xs font-medium text-gray-800 truncate" title={item.name}>
                    {item.name}
                  </span>
                </div>

                <div className="flex items-center gap-2 shrink-0">
                  <span className="text-[10px] text-gray-400 font-mono">
                    {formatBytes(item.size_bytes)}
                  </span>
                  <button
                    onClick={() => onDismiss(item.id)}
                    className="text-gray-300 hover:text-gray-600 p-0.5 rounded transition-colors"
                    title="Dismiss"
                  >
                    <X className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>

              {/* Progress bar */}
              <div className="w-full bg-gray-100 h-1.5 rounded-full overflow-hidden">
                <div
                  className={`h-full transition-all duration-300 ${
                    item.status === 'completed'
                      ? 'bg-emerald-500 w-full'
                      : item.status === 'error'
                      ? 'bg-red-500 w-full'
                      : 'bg-blue-600'
                  }`}
                  style={{
                    width: item.status === 'error' || item.status === 'completed' ? '100%' : `${item.progress}%`,
                  }}
                />
              </div>

              {/* Status footer message */}
              <div className="flex items-center justify-between text-[10px]">
                <span
                  className={
                    item.status === 'completed'
                      ? 'text-emerald-600 font-medium'
                      : item.status === 'error'
                      ? 'text-red-500 font-medium truncate max-w-[200px]'
                      : 'text-gray-400'
                  }
                >
                  {item.status === 'reading'
                    ? 'Reading file into memory...'
                    : item.status === 'uploading'
                    ? 'Importing into Iroh & syncing swarm...'
                    : item.status === 'completed'
                    ? 'Uploaded successfully'
                    : item.error || 'Upload failed'}
                </span>
                <span className="text-gray-400 font-mono">{item.progress}%</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

