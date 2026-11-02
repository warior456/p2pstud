import React, { useState } from 'react';
import { 
  Folder, 
  FileText, 
  Cloud, 
  ArrowUpCircle, 
  Pin, 
  Loader2, 
  MoreVertical,
  Download,
  Eye,
  Copy,
  Trash2,
  Lock,
  User
} from 'lucide-react';
import { FileEntry } from '../../types';

interface FileRowProps {
  file: FileEntry;
  isSelected?: boolean;
  currentUserId?: string | null;
  onClick: () => void;
  onPin: (hash: string) => void;
  onUnpin: (hash: string) => void;
  onDownload: (hash: string) => void;
  onPreview: (hash: string) => void;
  onDelete?: (hash: string) => void;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

export const FileRow: React.FC<FileRowProps> = ({
  file,
  isSelected = false,
  currentUserId,
  onClick,
  onPin,
  onUnpin,
  onDownload,
  onPreview,
  onDelete,
}) => {
  const [menuOpen, setMenuOpen] = useState(false);

  const isOwner = !file.owner_node_id || (currentUserId && file.owner_node_id === currentUserId);

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    setMenuOpen(true);
  };

  const copyHash = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(file.hash);
    setMenuOpen(false);
  };

  const renderBadge = () => {
    switch (file.sync_state) {
      case 'cloud_only':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-gray-100 text-gray-600">
            <Cloud className="w-3.5 h-3.5 text-gray-500" />
            Cloud Only
          </span>
        );
      case 'cached':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-blue-50 text-blue-700">
            <ArrowUpCircle className="w-3.5 h-3.5 text-blue-500" />
            Seeding
          </span>
        );
      case 'pinned':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-green-50 text-green-700">
            <Pin className="w-3.5 h-3.5 text-green-600 fill-green-600" />
            Pinned
          </span>
        );
      case 'downloading':
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-orange-50 text-orange-700">
            <Loader2 className="w-3.5 h-3.5 text-orange-500 animate-spin" />
            {file.download_progress ?? 0}%
          </span>
        );
    }
  };

  return (
    <div
      onClick={onClick}
      onContextMenu={handleContextMenu}
      className={`relative group flex items-center justify-between px-4 py-2.5 text-sm cursor-pointer select-none border-b border-gray-100 transition-colors ${
        isSelected ? 'bg-blue-50 hover:bg-blue-100/70' : 'hover:bg-gray-50'
      }`}
    >
      <div className="flex items-center gap-3 min-w-0 flex-1">
        {file.is_dir ? (
          <Folder className="w-5 h-5 text-amber-500 flex-shrink-0" />
        ) : (
          <FileText className="w-5 h-5 text-blue-500 flex-shrink-0" />
        )}
        <span className="truncate font-medium text-gray-800">{file.name}</span>
        {file.owner_node_id && (
          <span
            className={`text-[10px] px-1.5 py-0.5 rounded flex items-center gap-1 flex-shrink-0 ${
              isOwner ? 'bg-green-50 text-green-700 border border-green-200' : 'bg-gray-100 text-gray-500'
            }`}
            title={`Uploaded by: ${file.owner_node_id}`}
          >
            <User className="w-2.5 h-2.5" />
            {isOwner ? 'Uploaded by you' : `${file.owner_node_id.slice(0, 6)}...`}
          </span>
        )}
      </div>

      <div className="flex items-center gap-4 flex-shrink-0">
        <span className="text-xs text-gray-500 w-20 text-right">
          {file.is_dir ? '--' : formatBytes(file.size_bytes)}
        </span>
        <div className="w-28 flex justify-end">{renderBadge()}</div>

        <button
          onClick={(e) => {
            e.stopPropagation();
            setMenuOpen(!menuOpen);
          }}
          className="p-1 rounded text-gray-400 hover:text-gray-700 hover:bg-gray-200"
        >
          <MoreVertical className="w-4 h-4" />
        </button>
      </div>

      {menuOpen && (
        <div 
          className="absolute right-4 top-10 z-20 w-48 bg-white border border-gray-200 rounded-lg shadow-lg py-1 text-xs"
          onMouseLeave={() => setMenuOpen(false)}
        >
          {!file.is_dir && (
            <>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onPreview(file.hash);
                  setMenuOpen(false);
                }}
                className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 text-gray-700"
              >
                <Eye className="w-3.5 h-3.5" /> Preview
              </button>

              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onDownload(file.hash);
                  setMenuOpen(false);
                }}
                className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 text-gray-700"
              >
                <Download className="w-3.5 h-3.5" /> Download
              </button>

              {file.sync_state === 'pinned' ? (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onUnpin(file.hash);
                    setMenuOpen(false);
                  }}
                  className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 text-gray-700"
                >
                  <Pin className="w-3.5 h-3.5" /> Unpin from Offline
                </button>
              ) : (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onPin(file.hash);
                    setMenuOpen(false);
                  }}
                  className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 text-gray-700"
                >
                  <Pin className="w-3.5 h-3.5 text-green-600" /> Keep Offline (Pin)
                </button>
              )}

              <div className="border-t border-gray-100 my-1" />
            </>
          )}

          <button
            onClick={copyHash}
            className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 text-gray-700"
          >
            <Copy className="w-3.5 h-3.5" /> Copy Hash (BLAKE3)
          </button>

          {!file.is_dir && (
            <>
              <div className="border-t border-gray-100 my-1" />
              {isOwner ? (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    if (onDelete) onDelete(file.hash);
                    setMenuOpen(false);
                  }}
                  className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-red-50 text-red-600 font-medium"
                >
                  <Trash2 className="w-3.5 h-3.5 text-red-600" /> Delete My File
                </button>
              ) : (
                <div 
                  className="w-full text-left px-3 py-1.5 flex items-center gap-2 text-gray-400 cursor-not-allowed"
                  title="Only the uploader can delete this file"
                >
                  <Lock className="w-3.5 h-3.5" /> Only uploader can delete
                </div>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
};
