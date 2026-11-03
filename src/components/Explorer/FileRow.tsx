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
  HardDriveDownload,
  AlertTriangle,
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
  onDeleteOrphan?: (hash: string) => void;
  onEvictLocal?: (hash: string) => void;
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
  onDeleteOrphan,
  onEvictLocal,
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
      className={`group flex items-center justify-between px-4 py-3 hover:bg-gray-50 border-b border-gray-100 cursor-pointer relative ${
        isSelected ? 'bg-blue-50/50' : ''
      }`}
    >
      <div className="flex items-center gap-3 overflow-hidden">
        {file.is_dir ? (
          <Folder className="w-5 h-5 text-blue-500 fill-blue-50 shrink-0" />
        ) : (
          <FileText className="w-5 h-5 text-gray-400 shrink-0" />
        )}
        <div className="flex flex-col truncate">
          <span className="text-sm font-medium text-gray-800 truncate">{file.name}</span>
          <div className="flex items-center gap-2 text-[10px] text-gray-400 mt-0.5">
            {file.owner_node_id && (
              <span className="flex items-center gap-1 font-mono">
                <User className="w-2.5 h-2.5" />
                {isOwner ? 'You (Owner)' : `Owner: ${file.owner_node_id.slice(0, 8)}...`}
              </span>
            )}
            <span className="font-mono">{file.hash.slice(0, 10)}...</span>
          </div>
        </div>
      </div>

      <div className="flex items-center gap-4">
        {!file.is_dir && (
          <span className="text-xs text-gray-400 font-mono w-16 text-right">
            {formatBytes(file.size_bytes)}
          </span>
        )}

        <div className="w-24 flex justify-end">{renderBadge()}</div>

        <button
          onClick={(e) => {
            e.stopPropagation();
            setMenuOpen(!menuOpen);
          }}
          className="p-1 rounded hover:bg-gray-200 text-gray-400 hover:text-gray-600"
        >
          <MoreVertical className="w-4 h-4" />
        </button>
      </div>

      {/* Context Menu */}
      {menuOpen && (
        <div
          onMouseLeave={() => setMenuOpen(false)}
          className="absolute right-4 top-10 bg-white border border-gray-200 shadow-lg rounded-lg py-1 z-20 text-xs w-56 font-normal"
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

              {file.sync_state !== 'cloud_only' && onEvictLocal && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onEvictLocal(file.hash);
                    setMenuOpen(false);
                  }}
                  className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 text-gray-700"
                >
                  <HardDriveDownload className="w-3.5 h-3.5 text-gray-500" /> Free Local Disk Space
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
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    if (onDeleteOrphan) onDeleteOrphan(file.hash);
                    setMenuOpen(false);
                  }}
                  className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-amber-50 text-amber-700 font-medium"
                  title="Remove abandoned file whose owner no longer exists in workspace"
                >
                  <AlertTriangle className="w-3.5 h-3.5 text-amber-600" /> Clean Up Orphaned File
                </button>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
};
