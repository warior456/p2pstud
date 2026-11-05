import React from 'react';
import { FileArchive, AlertTriangle, FolderInput, Upload, X, ShieldAlert } from 'lucide-react';

export interface ZipFileItem {
  name: string;
  size_bytes: number;
  path?: string;
  file?: File;
}

export type ZipTarget = File | ZipFileItem;

interface ZipImportModalProps {
  isOpen: boolean;
  file: ZipTarget | null;
  targetFolder: string;
  onUnpack: () => void;
  onUploadAsZip: () => void;
  onCancel: () => void;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export const ZipImportModal: React.FC<ZipImportModalProps> = ({
  isOpen,
  file,
  targetFolder,
  onUnpack,
  onUploadAsZip,
  onCancel,
}) => {
  if (!isOpen || !file) return null;

  const fileName = file.name;
  const fileBytes = 'size_bytes' in file ? file.size_bytes : (file as File).size;
  const displayFolder = targetFolder.replace(/\/$/, '') || 'Shared Cloud (Root)';

  return (
    <div className="fixed inset-0 z-50 bg-black/50 backdrop-blur-xs flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl max-w-md w-full shadow-2xl border border-gray-100 overflow-hidden animate-in fade-in zoom-in-95 duration-150">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-100 bg-gradient-to-r from-amber-50 to-orange-50">
          <div className="flex items-center gap-2.5">
            <div className="w-9 h-9 rounded-xl bg-amber-100 text-amber-700 flex items-center justify-center border border-amber-200">
              <FileArchive className="w-5 h-5 text-amber-600" />
            </div>
            <div>
              <h3 className="text-sm font-bold text-gray-900">Import ZIP Archive</h3>
              <p className="text-[11px] text-gray-500">Security & extraction options</p>
            </div>
          </div>
          <button
            onClick={onCancel}
            className="p-1 text-gray-400 hover:text-gray-600 rounded-lg hover:bg-white/80 transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 space-y-4">
          {/* File Information */}
          <div className="bg-gray-50 rounded-xl p-3 border border-gray-200 space-y-1.5 text-xs">
            <div className="flex justify-between items-center">
              <span className="text-gray-500 font-medium">Archive Name:</span>
              <span className="font-bold text-gray-900 font-mono truncate max-w-[200px]" title={fileName}>
                {fileName}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-500 font-medium">Size:</span>
              <span className="font-semibold text-gray-700">{formatBytes(fileBytes)}</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-500 font-medium">Destination:</span>
              <span className="font-semibold text-blue-700 font-mono truncate max-w-[200px]">
                {displayFolder}
              </span>
            </div>
          </div>

          {/* Security & Swarm Notice Warning */}
          <div className="bg-amber-50/80 border border-amber-200 rounded-xl p-3.5 flex items-start gap-3">
            <AlertTriangle className="w-5 h-5 text-amber-600 shrink-0 mt-0.5" />
            <div className="space-y-1 text-xs text-amber-900 leading-relaxed">
              <span className="font-bold block">Important Security Notice</span>
              <p className="text-[11px] text-amber-800">
                Unpacking will extract all internal files and subfolders into the shared workspace and distribute them across connected peers.
              </p>
              <div className="flex items-center gap-1.5 pt-1 text-[11px] text-amber-700 font-medium">
                <ShieldAlert className="w-3.5 h-3.5 text-amber-600 shrink-0" />
                <span>Executable scripts (.exe, .bat, .sh) are quarantined for peer safety.</span>
              </div>
            </div>
          </div>

          <p className="text-xs text-gray-500">
            How would you like to handle this archive?
          </p>

          {/* Action Options */}
          <div className="space-y-2 pt-1">
            <button
              onClick={onUnpack}
              className="w-full py-2.5 px-4 bg-blue-600 hover:bg-blue-700 text-white rounded-xl text-xs font-semibold flex items-center justify-center gap-2 shadow-sm transition-all cursor-pointer"
            >
              <FolderInput className="w-4 h-4" />
              <span>Unpack & Import All Files into Folder</span>
            </button>

            <button
              onClick={onUploadAsZip}
              className="w-full py-2.5 px-4 bg-white hover:bg-gray-50 text-gray-700 border border-gray-300 rounded-xl text-xs font-semibold flex items-center justify-center gap-2 transition-all cursor-pointer shadow-2xs"
            >
              <Upload className="w-4 h-4 text-gray-500" />
              <span>Upload as Single Compressed .zip File</span>
            </button>

            <button
              onClick={onCancel}
              className="w-full py-1.5 text-xs font-medium text-gray-400 hover:text-gray-600 text-center transition-colors cursor-pointer"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
