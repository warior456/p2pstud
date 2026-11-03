import React, { useState } from 'react';
import { UploadCloud, Plus, FolderPlus, ArrowLeft, Loader2, AlertTriangle, Shield } from 'lucide-react';
import { FileEntry, UploadItem } from '../../types';
import { FileRow } from './FileRow';

interface FileTreeProps {
  files: FileEntry[];
  selectedHash: string | null;
  currentUserId?: string | null;
  uploadQueue?: UploadItem[];
  isRoot?: boolean;
  onSelectFile: (file: FileEntry) => void;
  onNavigateFolder: (name: string) => void;
  onNavigateUp?: () => void;
  onCreateFolder?: () => void;
  onPin: (hash: string) => void;
  onUnpin: (hash: string) => void;
  onDownload: (hash: string) => void;
  onPreview: (hash: string) => void;
  onDelete?: (hash: string) => void;
  onVoteDelete?: (hash: string) => void;
  onUnvoteDelete?: (hash: string) => void;
  onVoteKeep?: (hash: string) => void;
  onUnvoteKeep?: (hash: string) => void;
  onEvictLocal?: (hash: string) => void;
  onFileUploaded?: (hash: string) => void;
  onTriggerUpload?: () => void;
  onUploadFiles?: (files: FileList | File[]) => void;
  onMove?: (hash: string) => void;
  onMoveToFolder?: (sourceHash: string, targetFolderName: string) => void;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

export const FileTree: React.FC<FileTreeProps> = ({
  files,
  selectedHash,
  currentUserId,
  uploadQueue = [],
  isRoot = true,
  onSelectFile,
  onNavigateFolder,
  onNavigateUp,
  onCreateFolder,
  onPin,
  onUnpin,
  onDownload,
  onPreview,
  onDelete,
  onVoteDelete,
  onUnvoteDelete,
  onVoteKeep,
  onUnvoteKeep,
  onEvictLocal,
  onTriggerUpload,
  onUploadFiles,
  onMove,
  onMoveToFolder,
}) => {
  const [isDragging, setIsDragging] = useState(false);

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  };

  const handleDragLeave = () => {
    setIsDragging(false);
  };

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);

    const items = e.dataTransfer.files;
    if (items.length > 0) {
      if (onUploadFiles) {
        onUploadFiles(items);
      }
    }
  };

  const activeUploads = uploadQueue.filter(
    (item) => item.status === 'reading' || item.status === 'uploading'
  );

  const nowSecs = Math.floor(Date.now() / 1000);
  const pendingDeletionFiles = files.filter(
    (f) => !f.is_dir && f.deletion_scheduled_at && f.deletion_scheduled_at > nowSecs
  );

  return (
    <div
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={`flex-1 overflow-y-auto bg-white transition-colors flex flex-col ${
        isDragging ? 'bg-blue-50/50 border-2 border-dashed border-blue-400' : ''
      }`}
    >
      {/* Pending Community Deletion Swarm Warning Banner */}
      {pendingDeletionFiles.length > 0 && (
        <div className="bg-red-50 border-b border-red-200 px-4 py-2.5 flex items-center justify-between text-xs text-red-900 shadow-2xs">
          <div className="flex items-center gap-2">
            <AlertTriangle className="w-4 h-4 text-red-600 shrink-0 animate-bounce" />
            <span>
              <strong>Pending Community Deletion:</strong> {pendingDeletionFiles.length}{' '}
              {pendingDeletionFiles.length === 1 ? 'file has' : 'files have'} reached the 86% delete consensus and will be permanently deleted after the 24h grace period. Review and click <strong>Defend</strong> to keep files you need!
            </span>
          </div>
          <div className="flex items-center gap-1 font-semibold text-red-700 bg-red-100/80 px-2 py-0.5 rounded border border-red-300 shrink-0 ml-2">
            <Shield className="w-3.5 h-3.5 text-red-600" />
            <span>Grace Period Active</span>
          </div>
        </div>
      )}

      {/* Action Toolbar */}
      <div className="flex items-center justify-between px-4 py-2 bg-gray-50/70 border-b border-gray-100 text-xs">
        <div className="flex items-center gap-2">
          {!isRoot && onNavigateUp && (
            <button
              onClick={onNavigateUp}
              className="flex items-center gap-1.5 px-2.5 py-1 text-gray-700 bg-white border border-gray-200 hover:bg-gray-100 rounded-md font-medium transition-colors"
              title="Go back to parent directory"
            >
              <ArrowLeft className="w-3.5 h-3.5" />
              <span>Back</span>
            </button>
          )}
          <span className="text-gray-400 font-medium">
            {files.length} {files.length === 1 ? 'item' : 'items'}
          </span>
        </div>

        <div className="flex items-center gap-2">
          {onCreateFolder && (
            <button
              onClick={onCreateFolder}
              className="flex items-center gap-1.5 px-2.5 py-1 text-gray-700 bg-white border border-gray-200 hover:bg-gray-100 rounded-md font-medium transition-colors shadow-2xs"
            >
              <FolderPlus className="w-3.5 h-3.5 text-blue-600" />
              <span>New Folder</span>
            </button>
          )}
          {onTriggerUpload && (
            <button
              onClick={onTriggerUpload}
              className="flex items-center gap-1.5 px-2.5 py-1 text-white bg-blue-600 hover:bg-blue-700 rounded-md font-semibold transition-colors shadow-2xs"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>Upload File</span>
            </button>
          )}
        </div>
      </div>

      {/* Live Uploading Progress Rows */}
      {activeUploads.length > 0 && (
        <div className="bg-blue-50/40 border-b border-blue-100 divide-y divide-blue-100/60">
          {activeUploads.map((item) => (
            <div key={item.id} className="px-4 py-2.5 flex items-center justify-between gap-3">
              <div className="flex items-center gap-2.5 overflow-hidden flex-1">
                <Loader2 className="w-4 h-4 text-blue-600 animate-spin shrink-0" />
                <div className="flex flex-col truncate flex-1">
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-xs font-semibold text-gray-800 truncate">{item.name}</span>
                    <span className="text-[10px] text-blue-600 font-mono font-medium">{item.progress}%</span>
                  </div>
                  <div className="w-full bg-blue-200/60 h-1.5 rounded-full mt-1 overflow-hidden">
                    <div
                      className="bg-blue-600 h-full transition-all duration-300"
                      style={{ width: `${item.progress}%` }}
                    />
                  </div>
                </div>
              </div>
              <div className="text-[10px] text-gray-400 font-mono shrink-0">
                {formatBytes(item.size_bytes)}
              </div>
            </div>
          ))}
        </div>
      )}

      {files.length === 0 && activeUploads.length === 0 ? (
        <div className="flex flex-col items-center justify-center flex-1 text-center p-8">
          <div className="w-16 h-16 rounded-full bg-blue-50 flex items-center justify-center text-blue-500 mb-4 shadow-xs">
            <UploadCloud className="w-8 h-8" />
          </div>
          <h3 className="text-base font-semibold text-gray-800 mb-1">
            {isRoot ? 'Shared Workspace is Empty' : 'This Folder is Empty'}
          </h3>
          <p className="text-xs text-gray-400 max-w-sm mb-4">
            Drag and drop study files, notes, or PDFs here to share them directly across the peer swarm.
          </p>
          <div className="flex items-center gap-2">
            {onCreateFolder && (
              <button
                onClick={onCreateFolder}
                className="px-3.5 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg text-xs font-semibold flex items-center gap-2 transition-colors"
              >
                <FolderPlus className="w-4 h-4 text-blue-600" />
                Create Folder
              </button>
            )}
            {onTriggerUpload && (
              <button
                onClick={onTriggerUpload}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-semibold flex items-center gap-2 shadow-xs transition-colors"
              >
                <Plus className="w-4 h-4" />
                Upload File
              </button>
            )}
          </div>
        </div>
      ) : (
        <div className="divide-y divide-gray-100 flex-1">
          {files.map((file) => (
            <FileRow
              key={file.hash || file.name}
              file={file}
              isSelected={selectedHash === file.hash}
              currentUserId={currentUserId}
              onClick={() => {
                if (file.is_dir) {
                  onNavigateFolder(file.name);
                } else {
                  onSelectFile(file);
                }
              }}
              onPin={onPin}
              onUnpin={onUnpin}
              onDownload={onDownload}
              onPreview={onPreview}
              onDelete={onDelete}
              onVoteDelete={onVoteDelete}
              onUnvoteDelete={onUnvoteDelete}
              onVoteKeep={onVoteKeep}
              onUnvoteKeep={onUnvoteKeep}
              onEvictLocal={onEvictLocal}
              onMove={onMove}
              onMoveToFolder={onMoveToFolder}
            />
          ))}
        </div>
      )}
    </div>
  );
};
