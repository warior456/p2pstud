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
  User,
  Vote,
  Sprout,
  FolderInput,
  Shield,
  Clock,
  Star,
  ThumbsUp,
  ThumbsDown,
  ShieldAlert,
  Archive,
  Ban
} from 'lucide-react';
import { FileEntry } from '../../types';
import { isSameNodeId } from '../../utils/nodeId';

interface FileRowProps {
  file: FileEntry;
  isSelected?: boolean;
  onToggleSelect?: (hash: string) => void;
  currentUserId?: string | null;
  onClick: () => void;
  onPin: (hash: string) => void;
  onUnpin: (hash: string) => void;
  onDownload: (hash: string) => void;
  onSaveAs?: (hash: string) => void;
  onOpenExternal?: (hash: string) => void;
  onPreview: (hash: string) => void;
  onDelete?: (hash: string) => void;
  onVoteDelete?: (hash: string) => void;
  onUnvoteDelete?: (hash: string) => void;
  onVoteKeep?: (hash: string) => void;
  onUnvoteKeep?: (hash: string) => void;
  onEvictLocal?: (hash: string) => void;
  onMove?: (hash: string) => void;
  onMoveToFolder?: (sourceHash: string, targetFolderName: string) => void;
  onRate?: (hash: string, rating: 1 | -1) => void;
  onToggleStar?: (hash: string) => void;
  onReportDangerous?: (hash: string, reason: string) => void;
  onExportFolderZip?: (folderName: string) => void;
  onDeleteFolder?: (folderName: string) => void;
  onBlockFile?: (hash: string) => void;
  onInspectSecurity?: (file: FileEntry) => void;
}

const DANGEROUS_EXTENSIONS = [
  '.exe', '.bat', '.cmd', '.msi', '.ps1', '.sh', '.bash', '.elf', '.bin', 
  '.dll', '.so', '.dylib', '.vbs', '.js', '.vbe', '.wsf', '.wsh', '.scr', '.pif',
  '.html', '.htm', '.xhtml', '.svg', '.docm', '.xlsm', '.pptm'
];

function isDangerousFile(filename: string): boolean {
  const lower = filename.toLowerCase();
  return DANGEROUS_EXTENSIONS.some((ext) => lower.endsWith(ext));
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
  onToggleSelect,
  currentUserId,
  onClick,
  onPin,
  onUnpin,
  onDownload,
  onSaveAs,
  onOpenExternal,
  onPreview,
  onDelete,
  onVoteDelete,
  onUnvoteDelete,
  onVoteKeep,
  onUnvoteKeep,
  onEvictLocal,
  onMove,
  onMoveToFolder,
  onRate,
  onToggleStar,
  onReportDangerous,
  onExportFolderZip,
  onDeleteFolder,
  onBlockFile,
  onInspectSecurity,
}) => {
  const [menuOpen, setMenuOpen] = useState(false);
  const [isDragTarget, setIsDragTarget] = useState(false);

  const isOwner = Boolean(
    file.owner_node_id && currentUserId && isSameNodeId(file.owner_node_id, currentUserId)
  );
  const isSoleSeed = (file.seeds_count ?? 1) <= 1;
  const deleteVotes = file.delete_votes || [];
  const keepVotes = file.keep_votes || [];
  const hasVotedDelete = Boolean(currentUserId && deleteVotes.some((v) => isSameNodeId(v, currentUserId)));
  const hasVotedKeep = Boolean(currentUserId && keepVotes.some((v) => isSameNodeId(v, currentUserId)));

  const totalVotes = deleteVotes.length + keepVotes.length;
  const deleteRatio = totalVotes > 0 ? Math.round((deleteVotes.length / totalVotes) * 100) : 0;

  const nowSecs = Math.floor(Date.now() / 1000);
  const isScheduledForDeletion = Boolean(file.deletion_scheduled_at && file.deletion_scheduled_at > nowSecs);
  const remainingSecs = isScheduledForDeletion ? ((file.deletion_scheduled_at || 0) - nowSecs) : 0;
  const remainingHours = Math.floor(remainingSecs / 3600);
  const remainingMins = Math.floor((remainingSecs % 3600) / 60);
  const timeRemaining = remainingHours > 0 ? `${remainingHours}h ${remainingMins}m` : `${remainingMins}m`;

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    setMenuOpen(true);
  };

  const copyHash = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(file.hash);
    setMenuOpen(false);
  };

  const handleSafeAction = (action: () => void, isDangerousPrompt: string) => {
    if (file.is_quarantined) {
      const proceed = window.confirm(
        `SECURITY ALERT: "${file.name}" is currently QUARANTINED by community safety consensus.\n\nAre you sure you want to proceed?`
      );
      if (!proceed) return;
    }

    if (isDangerousFile(file.name)) {
      const proceed = window.confirm(
        `SECURITY WARNING: "${file.name}" is an executable or script (${isDangerousPrompt}). Running untrusted software downloaded over P2P may harm your computer.\n\nDo you want to proceed?`
      );
      if (!proceed) return;
    }

    action();
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
      draggable={!file.is_dir}
      onDragStart={(e) => {
        if (!file.is_dir) {
          e.dataTransfer.setData('text/p2pstud-hash', file.hash);
          e.dataTransfer.setData('text/plain', file.name);
        }
      }}
      onDragOver={(e) => {
        if (file.is_dir) {
          e.preventDefault();
          e.stopPropagation();
          setIsDragTarget(true);
        }
      }}
      onDragLeave={() => {
        if (file.is_dir) {
          setIsDragTarget(false);
        }
      }}
      onDrop={(e) => {
        if (file.is_dir) {
          e.preventDefault();
          e.stopPropagation();
          setIsDragTarget(false);
          const hash = e.dataTransfer.getData('text/p2pstud-hash');
          if (hash && onMoveToFolder) {
            onMoveToFolder(hash, file.name);
          }
        }
      }}
      className={`group flex items-center justify-between px-4 py-3 hover:bg-gray-50 border-b border-gray-100 cursor-pointer relative transition-all ${
        isSelected ? 'bg-blue-50/50' : ''
      } ${file.is_quarantined ? 'bg-red-50/40 border-red-100' : ''} ${
        isDragTarget ? 'bg-blue-100/70 border-blue-400 ring-2 ring-blue-400/50' : ''
      }`}
    >
      <div className="flex items-center gap-3 overflow-hidden">
        {/* Selection Checkbox */}
        {onToggleSelect && !file.is_dir && (
          <input
            type="checkbox"
            checked={isSelected}
            onChange={(e) => {
              e.stopPropagation();
              onToggleSelect(file.hash);
            }}
            onClick={(e) => e.stopPropagation()}
            className="w-4 h-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500 cursor-pointer shrink-0"
            aria-label={`Select ${file.name}`}
          />
        )}

        {/* Star Button for study prioritization */}
        {!file.is_dir && onToggleStar && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              onToggleStar(file.hash);
            }}
            className="p-1 rounded text-gray-300 hover:text-amber-400 transition-colors"
            title={file.is_starred ? 'Remove star' : 'Star file for quick access'}
          >
            <Star
              className={`w-4 h-4 ${
                file.is_starred ? 'text-amber-400 fill-amber-400' : 'hover:text-amber-400'
              }`}
            />
          </button>
        )}

        {file.is_dir ? (
          <Folder className="w-5 h-5 text-blue-500 fill-blue-50 shrink-0" />
        ) : file.is_quarantined ? (
          <ShieldAlert className="w-5 h-5 text-red-600 shrink-0 animate-pulse" />
        ) : (
          <FileText className="w-5 h-5 text-gray-400 shrink-0" />
        )}

        <div className="flex flex-col truncate">
          <div className="flex items-center gap-2 truncate">
            <span
              className={`text-sm font-medium truncate ${
                file.is_quarantined ? 'text-red-900 line-through' : 'text-gray-800'
              }`}
            >
              {file.name}
            </span>

            {file.is_quarantined ? (
              <span
                onClick={(e) => {
                  e.stopPropagation();
                  onInspectSecurity?.(file);
                }}
                className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-bold bg-red-100 text-red-800 border border-red-300 hover:bg-red-200 cursor-pointer transition-colors"
                title={`Quarantined by community security reports (${file.quarantine_report_count || 5}+ reports). Click to inspect threat details.`}
              >
                <ShieldAlert className="w-3 h-3 text-red-600" />
                ⛔ Quarantined ({file.quarantine_report_count || 5} reports)
              </span>
            ) : (file.quarantine_report_count || 0) > 0 ? (
              <span
                onClick={(e) => {
                  e.stopPropagation();
                  onInspectSecurity?.(file);
                }}
                className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-semibold bg-amber-100 text-amber-800 border border-amber-300 hover:bg-amber-200 cursor-pointer transition-colors"
                title={`Reported as suspicious by ${file.quarantine_report_count} peer(s). Click to inspect threat details.`}
              >
                <ShieldAlert className="w-3 h-3 text-amber-600" />
                ⚠️ Flagged ({file.quarantine_report_count})
              </span>
            ) : null}

            {isScheduledForDeletion ? (
              <span
                className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-bold bg-red-100 text-red-800 border border-red-300 animate-pulse"
                title={`Consensus reached (>= 86%). Scheduled for permanent deletion in ${timeRemaining}. Cast a Keep vote to defend and rescue this file!`}
              >
                <Clock className="w-3 h-3 text-red-600" />
                ⚠️ Deleting in {timeRemaining} ({deleteVotes.length}🗑️ / {keepVotes.length}🛡️)
              </span>
            ) : deleteVotes.length > 0 ? (
              <span
                className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-semibold bg-amber-100 text-amber-800 border border-amber-300"
                title={`${deleteVotes.length} delete vote(s), ${keepVotes.length} keep vote(s). Delete consensus: ${deleteRatio}%. Deletion requires >= 3 delete votes and >= 86% ratio.`}
              >
                <AlertTriangle className="w-3 h-3 text-amber-600" />
                {deleteVotes.length}🗑️ {keepVotes.length > 0 ? `| ${keepVotes.length}🛡️` : ''} ({deleteRatio}%)
              </span>
            ) : null}

            {!file.is_dir && !isOwner && (isScheduledForDeletion || deleteVotes.length > 0) && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  if (hasVotedKeep) {
                    if (onUnvoteKeep) onUnvoteKeep(file.hash);
                  } else {
                    if (onVoteKeep) onVoteKeep(file.hash);
                  }
                }}
                className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium transition-colors shadow-2xs ${
                  hasVotedKeep
                    ? 'bg-emerald-600 text-white hover:bg-emerald-700'
                    : 'bg-emerald-50 text-emerald-700 border border-emerald-300 hover:bg-emerald-100'
                }`}
                title={hasVotedKeep ? 'Click to retract your keep vote' : 'Click to vote to keep and defend this file from deletion'}
              >
                <Shield className="w-2.5 h-2.5" />
                {hasVotedKeep ? 'Defending' : 'Defend'}
              </button>
            )}

            {!file.is_dir && file.seeds_count !== undefined && (
              <span
                className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium border ${
                  file.seeds_count <= 1
                    ? 'bg-amber-50 text-amber-700 border-amber-200'
                    : 'bg-emerald-50 text-emerald-700 border-emerald-200'
                }`}
                title={
                  file.seeds_count <= 1
                    ? 'Only 1 active seed in swarm! The smart cache automatically prioritizes caching and seeding this file.'
                    : `${file.seeds_count} active seeds across the swarm`
                }
              >
                <Sprout className="w-3 h-3" />
                {file.seeds_count} {file.seeds_count === 1 ? 'seed (rare)' : 'seeds'}
              </span>
            )}
          </div>

          <div className="flex items-center gap-2 text-[10px] text-gray-400 mt-0.5">
            {file.is_dir ? (
              <span className="font-medium text-blue-600">
                {file.item_count ?? 0} {file.item_count === 1 ? 'item' : 'items'}
              </span>
            ) : (
              <>
                {file.owner_node_id && (
                  <span
                    className={`flex items-center gap-1 font-mono ${
                      isOwner ? 'text-blue-600 font-semibold' : 'text-gray-500'
                    }`}
                    title={isOwner ? 'You created and own this file' : `Owner Node ID: ${file.owner_node_id}`}
                  >
                    <User className="w-2.5 h-2.5" />
                    {isOwner ? 'You (Owner)' : `Owner: ${file.owner_node_id.slice(0, 8)}...`}
                  </span>
                )}
                <span className="font-mono text-gray-400" title={`Blob hash: ${file.hash}`}>
                  #{file.hash.slice(0, 8)}...
                </span>
              </>
            )}
          </div>
        </div>
      </div>

      <div className="flex items-center gap-4">
        {/* Rating Controls (Thumbs up / down) */}
        {!file.is_dir && onRate && (
          <div className="flex items-center gap-1 bg-gray-100/70 rounded-md px-1.5 py-0.5 text-xs text-gray-600">
            <button
              onClick={(e) => {
                e.stopPropagation();
                onRate(file.hash, 1);
              }}
              className="p-0.5 hover:text-emerald-600 transition-colors"
              title="Helpful study material (+1)"
            >
              <ThumbsUp className="w-3 h-3" />
            </button>
            <span
              className={`font-mono text-[11px] font-semibold px-0.5 ${
                (file.net_rating ?? 0) > 0
                  ? 'text-emerald-600'
                  : (file.net_rating ?? 0) < 0
                  ? 'text-red-500'
                  : 'text-gray-500'
              }`}
            >
              {(file.net_rating ?? 0) > 0 ? `+${file.net_rating}` : file.net_rating ?? 0}
            </span>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onRate(file.hash, -1);
              }}
              className="p-0.5 hover:text-red-600 transition-colors"
              title="Low quality / inaccurate (-1)"
            >
              <ThumbsDown className="w-3 h-3" />
            </button>
          </div>
        )}

        <span className="text-xs text-gray-400 font-mono w-16 text-right">
          {formatBytes(file.size_bytes)}
        </span>

        <div className="w-24 flex justify-end">
          {file.is_dir ? (
            <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium bg-gray-100 text-gray-600">
              Folder
            </span>
          ) : (
            renderBadge()
          )}
        </div>

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
          className="absolute right-4 top-10 bg-white border border-gray-200 shadow-lg rounded-lg py-1 z-20 text-xs w-60 font-normal"
        >
          {file.is_dir ? (
            <>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setMenuOpen(false);
                  if (onClick) onClick();
                }}
                className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 text-gray-700"
              >
                <Folder className="w-3.5 h-3.5 text-blue-500" /> Open Folder
              </button>
              {onExportFolderZip && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setMenuOpen(false);
                    onExportFolderZip(file.name);
                  }}
                  className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 text-gray-700"
                  title="Export this folder as a compressed ZIP archive"
                >
                  <Archive className="w-3.5 h-3.5 text-amber-600" /> Export Folder as ZIP
                </button>
              )}
              {onDeleteFolder && (
                <>
                  <div className="border-t border-gray-100 my-1" />
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      setMenuOpen(false);
                      onDeleteFolder(file.name);
                    }}
                    className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-red-50 text-red-600 font-medium cursor-pointer"
                  >
                    <Trash2 className="w-3.5 h-3.5 text-red-600" /> Delete Folder
                  </button>
                </>
              )}
            </>
          ) : (
            <>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setMenuOpen(false);
                  handleSafeAction(() => onPreview(file.hash), 'Preview');
                }}
                className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 text-gray-700"
              >
                <Eye className="w-3.5 h-3.5" /> Preview
              </button>

              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setMenuOpen(false);
                  handleSafeAction(() => onDownload(file.hash), 'Download');
                }}
                className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 text-gray-700"
              >
                <Download className="w-3.5 h-3.5" /> Save to Downloads Folder
              </button>

              {onSaveAs && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setMenuOpen(false);
                    handleSafeAction(() => onSaveAs(file.hash), 'Save As');
                  }}
                  className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 text-gray-700"
                >
                  <HardDriveDownload className="w-3.5 h-3.5 text-blue-600" /> Save As...
                </button>
              )}

              {onOpenExternal && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setMenuOpen(false);
                    handleSafeAction(() => onOpenExternal(file.hash), 'Open');
                  }}
                  className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 text-gray-700"
                  title="Open this file in your operating system's default application"
                >
                  <Eye className="w-3.5 h-3.5 text-indigo-600" /> Open with System Viewer
                </button>
              )}

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

              {onMove && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setMenuOpen(false);
                    onMove(file.hash);
                  }}
                  className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 text-gray-700"
                >
                  <FolderInput className="w-3.5 h-3.5 text-blue-600" /> Move to Folder...
                </button>
              )}

              {file.sync_state !== 'cloud_only' && onEvictLocal && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    if (isSoleSeed) {
                      alert(
                        `Cannot remove "${file.name}" from local disk: You are currently the ONLY active seed in the swarm!\n\nIf you delete your copy, this file will be permanently lost for all students. Wait until at least one other peer caches it, or vote to delete the file if it is no longer needed.`
                      );
                      return;
                    }
                    onEvictLocal(file.hash);
                    setMenuOpen(false);
                  }}
                  className={`w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-gray-100 ${
                    isSoleSeed ? 'text-gray-400' : 'text-gray-700'
                  }`}
                  title={
                    isSoleSeed
                      ? 'You are the only active seed! Eviction is blocked to prevent permanent file loss.'
                      : 'Remove local copy to free space; file stays in cloud catalog'
                  }
                >
                  <HardDriveDownload className={`w-3.5 h-3.5 ${isSoleSeed ? 'text-amber-500' : 'text-gray-500'}`} />
                  <span>Free Local Disk Space</span>
                  {isSoleSeed && (
                    <span className="ml-auto text-[9px] font-semibold text-amber-700 bg-amber-50 px-1 py-0.5 rounded border border-amber-200">
                      Sole Seed
                    </span>
                  )}
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

          {!file.is_dir && onInspectSecurity && (file.is_quarantined || (file.quarantine_report_count || 0) > 0) && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                setMenuOpen(false);
                onInspectSecurity(file);
              }}
              className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-amber-50 text-amber-800 font-medium"
            >
              <ShieldAlert className="w-3.5 h-3.5 text-amber-600" /> View Threat Details
            </button>
          )}

          {!file.is_dir && onReportDangerous && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                setMenuOpen(false);
                const reason = window.prompt(
                  `Report "${file.name}" for security quarantine.\nProvide reason (e.g. malware, trojan, phishing, scam):`
                );
                if (reason && reason.trim()) {
                  onReportDangerous(file.hash, reason.trim());
                }
              }}
              className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-red-50 text-red-600"
            >
              <ShieldAlert className="w-3.5 h-3.5 text-red-600" /> Report Dangerous / Malicious
            </button>
          )}

          {!file.is_dir && onBlockFile && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                setMenuOpen(false);
                const confirm = window.confirm(
                  `Block and purge "${file.name}" locally?\n\nThis will permanently delete the file from your disk and ignore all swarm announcements for this content.`
                );
                if (confirm) {
                  onBlockFile(file.hash);
                }
              }}
              className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-rose-50 text-rose-700"
              title="Permanently remove from local disk and block future sync"
            >
              <Ban className="w-3.5 h-3.5 text-rose-700" /> Block & Purge Locally
            </button>
          )}

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
                <>
                  {hasVotedKeep ? (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        if (onUnvoteKeep) onUnvoteKeep(file.hash);
                        setMenuOpen(false);
                      }}
                      className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-emerald-50 text-emerald-800 font-medium"
                      title="Retract your vote to keep this file"
                    >
                      <Shield className="w-3.5 h-3.5 text-emerald-600 fill-emerald-600" /> Retract Keep Vote ({keepVotes.length} 🛡️)
                    </button>
                  ) : (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        if (onVoteKeep) onVoteKeep(file.hash);
                        setMenuOpen(false);
                      }}
                      className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-emerald-50 text-emerald-800 font-medium"
                      title="Vote to KEEP/DEFEND this file against community deletion"
                    >
                      <Shield className="w-3.5 h-3.5 text-emerald-600" /> Defend / Vote to Keep ({keepVotes.length} 🛡️)
                    </button>
                  )}

                  {hasVotedDelete ? (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        if (onUnvoteDelete) onUnvoteDelete(file.hash);
                        setMenuOpen(false);
                      }}
                      className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-amber-50 text-amber-800 font-medium"
                      title="Retract your vote to delete this file"
                    >
                      <Vote className="w-3.5 h-3.5 text-amber-600" /> Retract Delete Vote ({deleteVotes.length} 🗑️)
                    </button>
                  ) : (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        if (onVoteDelete) onVoteDelete(file.hash);
                        setMenuOpen(false);
                      }}
                      className="w-full text-left px-3 py-1.5 flex items-center gap-2 hover:bg-amber-50 text-amber-800 font-medium"
                      title="Vote to delete this file. Requires 3 community votes and >= 86% ratio to schedule deletion."
                    >
                      <Vote className="w-3.5 h-3.5 text-amber-600" /> Vote to Delete ({deleteVotes.length} 🗑️)
                    </button>
                  )}
                </>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
};
