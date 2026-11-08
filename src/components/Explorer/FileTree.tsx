import React, { useState, useMemo } from 'react';
import { 
  UploadCloud, 
  Plus, 
  FolderPlus, 
  ArrowLeft, 
  Loader2, 
  AlertTriangle, 
  Shield, 
  Trash2, 
  Filter, 
  Star, 
  HardDrive,
  Download,
  FolderInput,
  Archive,
  X
} from 'lucide-react';
import { FileEntry, UploadItem } from '../../types';
import { isSameNodeId } from '../../utils/nodeId';
import { FileRow } from './FileRow';

interface FileTreeProps {
  files: FileEntry[];
  selectedHash?: string | null;
  selectedHashes?: string[];
  onToggleSelectHash?: (hash: string) => void;
  onClearSelection?: () => void;
  onBatchDelete?: (hashes: string[]) => void;
  onBatchDownload?: (hashes: string[]) => void;
  onBatchMove?: (hashes: string[]) => void;
  currentUserId?: string | null;
  uploadQueue?: UploadItem[];
  isRoot?: boolean;
  currentFolderName?: string;
  onSelectFile: (file: FileEntry) => void;
  onNavigateFolder: (name: string) => void;
  onNavigateUp?: () => void;
  onCreateFolder?: () => void;
  onDeleteFolder?: (folderName: string) => void;
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
  onRate?: (hash: string, rating: 1 | -1) => void;
  onToggleStar?: (hash: string) => void;
  onReportDangerous?: (hash: string, reason: string) => void;
  onExportFolderZip?: (folderVirtualPath: string) => void;
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
  selectedHashes,
  onToggleSelectHash,
  onClearSelection,
  onBatchDelete,
  onBatchDownload,
  onBatchMove,
  currentUserId,
  uploadQueue = [],
  isRoot = true,
  currentFolderName,
  onSelectFile,
  onNavigateFolder,
  onNavigateUp,
  onCreateFolder,
  onDeleteFolder,
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
  onRate,
  onToggleStar,
  onReportDangerous,
  onExportFolderZip,
}) => {
  const [isDragging, setIsDragging] = useState(false);
  const [offlineOnly, setOfflineOnly] = useState(false);
  const [starredOnly, setStarredOnly] = useState(false);
  const [internalSelectedHashes, setInternalSelectedHashes] = useState<string[]>([]);

  const activeSelected = selectedHashes !== undefined ? selectedHashes : internalSelectedHashes;

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

  const filteredFiles = useMemo(() => {
    return files.filter((f) => {
      if (f.is_dir) return true; // Keep folders visible
      if (offlineOnly && f.sync_state !== 'pinned' && f.sync_state !== 'cached') {
        return false;
      }
      if (starredOnly && !f.is_starred) {
        return false;
      }
      return true;
    });
  }, [files, offlineOnly, starredOnly]);

  const selectableFiles = useMemo(() => filteredFiles.filter((f) => !f.is_dir), [filteredFiles]);

  const allFilteredSelected =
    selectableFiles.length > 0 &&
    selectableFiles.every((f) => activeSelected.includes(f.hash));

  const handleToggleSelect = (hash: string) => {
    if (onToggleSelectHash) {
      onToggleSelectHash(hash);
    } else {
      setInternalSelectedHashes((prev) =>
        prev.includes(hash) ? prev.filter((h) => h !== hash) : [...prev, hash]
      );
    }
  };

  const handleClear = () => {
    if (onClearSelection) {
      onClearSelection();
    } else {
      setInternalSelectedHashes([]);
    }
  };

  const handleToggleSelectAll = () => {
    if (allFilteredSelected) {
      handleClear();
    } else {
      const allHashes = selectableFiles.map((f) => f.hash);
      if (onToggleSelectHash) {
        allHashes.forEach((h) => {
          if (!activeSelected.includes(h)) onToggleSelectHash(h);
        });
      } else {
        setInternalSelectedHashes(allHashes);
      }
    }
  };

  const selectedFilesList = useMemo(() => {
    return files.filter((f) => activeSelected.includes(f.hash));
  }, [files, activeSelected]);

  const selectedFilesTotalBytes = useMemo(() => {
    return selectedFilesList.reduce((acc, f) => acc + (f.size_bytes || 0), 0);
  }, [selectedFilesList]);

  const ownedSelectedCount = useMemo(() => {
    if (!currentUserId) return 0;
    return selectedFilesList.filter((f) => isSameNodeId(f.owner_node_id, currentUserId)).length;
  }, [selectedFilesList, currentUserId]);

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
      <div className="flex items-center justify-between px-4 py-2 bg-gray-50/70 border-b border-gray-100 text-xs gap-2 flex-wrap">
        <div className="flex items-center gap-2 shrink-0">
          {!isRoot && onNavigateUp && (
            <button
              onClick={onNavigateUp}
              className="flex items-center gap-1.5 px-2.5 py-1 text-gray-700 bg-white border border-gray-200 hover:bg-gray-100 rounded-md font-medium transition-colors shrink-0 shadow-2xs cursor-pointer"
              title="Go back to parent directory"
            >
              <ArrowLeft className="w-3.5 h-3.5 shrink-0" />
              <span>Back</span>
            </button>
          )}

          {selectableFiles.length > 0 && (
            <label className="flex items-center gap-1.5 cursor-pointer text-gray-600 hover:text-gray-900 pr-1 select-none">
              <input
                type="checkbox"
                checked={allFilteredSelected}
                onChange={handleToggleSelectAll}
                className="w-3.5 h-3.5 rounded border-gray-300 text-blue-600 focus:ring-blue-500 cursor-pointer"
                title="Select or deselect all files in current view"
              />
              <span className="text-[11px] font-medium hidden sm:inline">Select All</span>
            </label>
          )}

          <span className="text-gray-400 font-medium whitespace-nowrap">
            {filteredFiles.length} {filteredFiles.length === 1 ? 'item' : 'items'}
          </span>

          {/* Quick Filters */}
          <div className="flex items-center gap-1 border-l border-gray-200 pl-2 ml-1">
            <button
              onClick={() => setOfflineOnly(!offlineOnly)}
              className={`flex items-center gap-1 px-2 py-0.5 rounded text-xs transition-colors ${
                offlineOnly
                  ? 'bg-blue-100 text-blue-700 font-semibold'
                  : 'text-gray-500 hover:bg-gray-200'
              }`}
              title="Show only locally cached or pinned files available offline"
            >
              <HardDrive className="w-3 h-3" />
              <span>Offline Ready</span>
            </button>

            <button
              onClick={() => setStarredOnly(!starredOnly)}
              className={`flex items-center gap-1 px-2 py-0.5 rounded text-xs transition-colors ${
                starredOnly
                  ? 'bg-amber-100 text-amber-800 font-semibold'
                  : 'text-gray-500 hover:bg-gray-200'
              }`}
              title="Show only starred study files"
            >
              <Star className="w-3 h-3 text-amber-500 fill-amber-500" />
              <span>Starred</span>
            </button>
          </div>
        </div>

        {/* Action buttons: only show in toolbar when files exist to prevent duplication with empty state buttons */}
        {filteredFiles.length > 0 && (
          <div className="flex items-center gap-2">
            {!isRoot && currentFolderName && onExportFolderZip && (
              <button
                onClick={() => onExportFolderZip(currentFolderName)}
                className="flex items-center gap-1.5 px-2.5 py-1 text-gray-700 bg-white border border-gray-200 hover:bg-gray-100 rounded-md font-medium transition-colors shadow-2xs cursor-pointer text-xs"
                title="Export this entire folder as a compressed ZIP archive"
              >
                <Archive className="w-3.5 h-3.5 text-amber-600" />
                <span>Export Folder as ZIP</span>
              </button>
            )}
            {onCreateFolder && (
              <button
                onClick={onCreateFolder}
                className="flex items-center gap-1.5 px-2.5 py-1 text-gray-700 bg-white border border-gray-200 hover:bg-gray-100 rounded-md font-medium transition-colors shadow-2xs cursor-pointer"
              >
                <FolderPlus className="w-3.5 h-3.5 text-blue-600" />
                <span>New Folder</span>
              </button>
            )}
            {onTriggerUpload && (
              <button
                onClick={onTriggerUpload}
                className="flex items-center gap-1.5 px-2.5 py-1 text-white bg-blue-600 hover:bg-blue-700 rounded-md font-semibold transition-colors shadow-2xs cursor-pointer"
              >
                <Plus className="w-3.5 h-3.5" />
                <span>Upload File</span>
              </button>
            )}
          </div>
        )}
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

      {filteredFiles.length === 0 && activeUploads.length === 0 ? (
        <div className="flex flex-col items-center justify-center flex-1 text-center p-8">
          <div className="w-16 h-16 rounded-full bg-blue-50 flex items-center justify-center text-blue-500 mb-4 shadow-xs">
            {offlineOnly || starredOnly ? <Filter className="w-8 h-8" /> : <UploadCloud className="w-8 h-8" />}
          </div>
          <h3 className="text-base font-semibold text-gray-800 mb-1">
            {offlineOnly
              ? 'No offline files available'
              : starredOnly
              ? 'No starred files yet'
              : isRoot
              ? 'Shared Workspace is Empty'
              : 'This Folder is Empty'}
          </h3>
          <p className="text-xs text-gray-400 max-w-sm mb-4">
            {offlineOnly
              ? 'Download or pin files from the cloud swarm to make them available offline.'
              : starredOnly
              ? 'Click the star icon on important study documents to highlight them here.'
              : 'Drag and drop study files, notes, or PDFs here to share them directly across the peer swarm.'}
          </p>
          <div className="flex items-center gap-2">
            {!isRoot && files.length === 0 && onDeleteFolder && currentFolderName && (
              <button
                onClick={() => onDeleteFolder(currentFolderName)}
                className="px-3.5 py-2 bg-red-50 hover:bg-red-100 text-red-600 rounded-lg text-xs font-semibold flex items-center gap-2 transition-colors border border-red-200"
              >
                <Trash2 className="w-4 h-4 text-red-600" />
                Delete This Empty Folder
              </button>
            )}
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
          {filteredFiles.map((file) => (
            <FileRow
              key={file.hash || file.name}
              file={file}
              isSelected={activeSelected.includes(file.hash) || selectedHash === file.hash}
              onToggleSelect={() => handleToggleSelect(file.hash)}
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
              onRate={onRate}
              onToggleStar={onToggleStar}
              onReportDangerous={onReportDangerous}
              onExportFolderZip={onExportFolderZip}
              onDeleteFolder={onDeleteFolder}
            />
          ))}
        </div>
      )}

      {/* Floating Sticky Batch Actions Toolbar */}
      {activeSelected.length > 0 && (
        <div className="sticky bottom-0 inset-x-0 bg-gray-900 text-white px-4 py-2.5 flex items-center justify-between shadow-2xl border-t border-gray-800 z-30 animate-in slide-in-from-bottom duration-200">
          <div className="flex items-center gap-2.5 text-xs">
            <span className="font-bold bg-blue-600 px-2 py-0.5 rounded-full text-[11px] tracking-wide">
              {activeSelected.length} selected
            </span>
            <span className="text-gray-300 text-xs hidden sm:inline">
              Total: {formatBytes(selectedFilesTotalBytes)}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => onBatchDownload && onBatchDownload(activeSelected)}
              className="px-3 py-1 bg-gray-800 hover:bg-gray-700 text-gray-100 rounded-md text-xs font-semibold flex items-center gap-1.5 transition-colors cursor-pointer border border-gray-700 shadow-2xs"
              title="Download selected files"
            >
              <Download className="w-3.5 h-3.5 text-blue-400" />
              <span>Download ({activeSelected.length})</span>
            </button>

            <button
              onClick={() => onBatchMove && onBatchMove(activeSelected)}
              className="px-3 py-1 bg-gray-800 hover:bg-gray-700 text-gray-100 rounded-md text-xs font-semibold flex items-center gap-1.5 transition-colors cursor-pointer border border-gray-700 shadow-2xs"
              title="Move selected files to another folder"
            >
              <FolderInput className="w-3.5 h-3.5 text-amber-400" />
              <span>Move ({activeSelected.length})</span>
            </button>

            <button
              onClick={() => onBatchDelete && onBatchDelete(activeSelected)}
              className={`px-3 py-1 rounded-md text-xs font-semibold flex items-center gap-1.5 transition-colors cursor-pointer shadow-2xs ${
                ownedSelectedCount > 0
                  ? 'bg-red-600/90 hover:bg-red-600 text-white'
                  : 'bg-gray-800 hover:bg-gray-700 text-gray-300 border border-gray-700'
              }`}
              title={
                ownedSelectedCount > 0
                  ? `Delete ${ownedSelectedCount} file(s) owned by you`
                  : `Vote to delete ${activeSelected.length} community file(s)`
              }
            >
              <Trash2 className="w-3.5 h-3.5 text-red-400" />
              <span>
                {ownedSelectedCount > 0
                  ? `Delete (${ownedSelectedCount} owned)`
                  : `Vote Delete (${activeSelected.length})`}
              </span>
            </button>

            <button
              onClick={handleClear}
              className="p-1 text-gray-400 hover:text-white rounded-md hover:bg-gray-800 transition-colors ml-1 cursor-pointer"
              title="Clear selection"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
