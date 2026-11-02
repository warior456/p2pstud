import React, { useState } from 'react';
import { UploadCloud, Plus } from 'lucide-react';
import { FileEntry } from '../../types';
import { FileRow } from './FileRow';
import { invoke } from '@tauri-apps/api/core';

interface FileTreeProps {
  files: FileEntry[];
  selectedHash: string | null;
  currentUserId?: string | null;
  onSelectFile: (file: FileEntry) => void;
  onNavigateFolder: (name: string) => void;
  onPin: (hash: string) => void;
  onUnpin: (hash: string) => void;
  onDownload: (hash: string) => void;
  onPreview: (hash: string) => void;
  onDelete?: (hash: string) => void;
  onFileUploaded?: (hash: string) => void;
  onTriggerUpload?: () => void;
}

export const FileTree: React.FC<FileTreeProps> = ({
  files,
  selectedHash,
  currentUserId,
  onSelectFile,
  onNavigateFolder,
  onPin,
  onUnpin,
  onDownload,
  onPreview,
  onDelete,
  onFileUploaded,
  onTriggerUpload,
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
      for (let i = 0; i < items.length; i++) {
        const file = items[i];
        try {
          const buffer = await file.arrayBuffer();
          const bytes = Array.from(new Uint8Array(buffer));
          const hash = await invoke<string>('upload_file_bytes', {
            name: file.name,
            data: bytes,
          });
          if (onFileUploaded) onFileUploaded(hash);
        } catch (err) {
          console.error('Failed to upload dropped file:', err);
        }
      }
    }
  };

  return (
    <div
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={`flex-1 overflow-y-auto relative min-h-[300px] transition-colors ${
        isDragging ? 'bg-blue-50/70 border-2 border-dashed border-blue-400' : 'bg-white'
      }`}
    >
      {isDragging && (
        <div className="absolute inset-0 flex flex-col items-center justify-center bg-blue-50/80 z-10 pointer-events-none">
          <UploadCloud className="w-12 h-12 text-blue-500 animate-bounce" />
          <p className="mt-2 text-sm font-semibold text-blue-600">Drop files to add to P2P workspace</p>
        </div>
      )}

      {files.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-24 text-gray-400">
          <UploadCloud className="w-12 h-12 mb-3 text-gray-300 stroke-[1.5]" />
          <p className="text-base font-medium text-gray-600">Workspace is empty</p>
          <p className="text-xs text-gray-400 mt-1 max-w-sm text-center">
            Upload files to share them with the P2P cloud. Anyone in the network can access them, and only you can delete files you uploaded.
          </p>
          {onTriggerUpload && (
            <button
              onClick={onTriggerUpload}
              className="mt-4 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-semibold flex items-center gap-2 shadow-xs transition-colors"
            >
              <Plus className="w-4 h-4" />
              Upload First File
            </button>
          )}
        </div>
      ) : (
        <div className="divide-y divide-gray-100">
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
            />
          ))}
        </div>
      )}
    </div>
  );
};
