import React, { useState } from 'react';
import { UploadCloud } from 'lucide-react';
import { FileEntry } from '../../types';
import { FileRow } from './FileRow';
import { invoke } from '@tauri-apps/api/core';

interface FileTreeProps {
  files: FileEntry[];
  selectedHash: string | null;
  onSelectFile: (file: FileEntry) => void;
  onNavigateFolder: (name: string) => void;
  onPin: (hash: string) => void;
  onUnpin: (hash: string) => void;
  onDownload: (hash: string) => void;
  onPreview: (hash: string) => void;
  onFileUploaded?: (hash: string) => void;
}

export const FileTree: React.FC<FileTreeProps> = ({
  files,
  selectedHash,
  onSelectFile,
  onNavigateFolder,
  onPin,
  onUnpin,
  onDownload,
  onPreview,
  onFileUploaded,
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
        // In Tauri with webview, (file as any).path may provide system path
        const path = (file as any).path;
        if (path) {
          try {
            const hash = await invoke<string>('add_file', { localPath: path });
            if (onFileUploaded) onFileUploaded(hash);
          } catch (err) {
            console.error('Failed to upload file:', err);
          }
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
        <div className="flex flex-col items-center justify-center py-20 text-gray-400">
          <UploadCloud className="w-10 h-10 mb-2 stroke-[1.5]" />
          <p className="text-sm">Folder is empty</p>
          <p className="text-xs text-gray-400 mt-1">Drag and drop files here to start sharing</p>
        </div>
      ) : (
        <div className="divide-y divide-gray-100">
          {files.map((file) => (
            <FileRow
              key={file.hash || file.name}
              file={file}
              isSelected={selectedHash === file.hash}
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
            />
          ))}
        </div>
      )}
    </div>
  );
};

