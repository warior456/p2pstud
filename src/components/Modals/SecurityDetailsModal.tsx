import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ShieldAlert, ShieldX, Copy, Check, X, AlertTriangle, AlertOctagon, Info } from 'lucide-react';
import { FileEntry, QuarantineInfo } from '../../types';

interface SecurityDetailsModalProps {
  isOpen: boolean;
  file: FileEntry | null;
  onClose: () => void;
  onBlockFile?: (hash: string) => void;
  onReportDangerous?: (hash: string, reason: string) => void;
}

export const SecurityDetailsModal: React.FC<SecurityDetailsModalProps> = ({
  isOpen,
  file,
  onClose,
  onBlockFile,
  onReportDangerous,
}) => {
  const [quarantineInfo, setQuarantineInfo] = useState<QuarantineInfo | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [copied, setCopied] = useState<boolean>(false);
  const [showReportInput, setShowReportInput] = useState<boolean>(false);
  const [reportReason, setReportReason] = useState<string>('');

  useEffect(() => {
    if (isOpen && file) {
      setLoading(true);
      setShowReportInput(false);
      setReportReason('');
      invoke<QuarantineInfo>('get_quarantine_info', { hash: file.hash })
        .then((info) => setQuarantineInfo(info))
        .catch((err) => {
          console.warn('Failed to load quarantine info:', err);
          setQuarantineInfo(null);
        })
        .finally(() => setLoading(false));
    } else {
      setQuarantineInfo(null);
    }
  }, [isOpen, file]);

  if (!isOpen || !file) return null;

  const handleCopyHash = () => {
    navigator.clipboard.writeText(file.hash);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const isQuarantined = file.is_quarantined || quarantineInfo?.is_quarantined;
  const reportCount = quarantineInfo?.report_count ?? file.quarantine_report_count ?? 0;
  const reasons = quarantineInfo?.reasons || [];

  const handleSubmitReport = () => {
    if (!reportReason.trim() || !onReportDangerous) return;
    onReportDangerous(file.hash, reportReason.trim());
    setShowReportInput(false);
    setReportReason('');
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/50 backdrop-blur-xs flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl max-w-lg w-full shadow-2xl border border-gray-100 overflow-hidden animate-in fade-in zoom-in-95 duration-150">
        {/* Header */}
        <div
          className={`flex items-center justify-between px-6 py-4 border-b ${
            isQuarantined
              ? 'border-red-200 bg-gradient-to-r from-red-50 to-rose-50'
              : 'border-amber-200 bg-gradient-to-r from-amber-50 to-yellow-50'
          }`}
        >
          <div className="flex items-center gap-2.5">
            <div
              className={`w-9 h-9 rounded-xl flex items-center justify-center border ${
                isQuarantined
                  ? 'bg-red-100 text-red-700 border-red-200'
                  : 'bg-amber-100 text-amber-700 border-amber-200'
              }`}
            >
              {isQuarantined ? (
                <ShieldAlert className="w-5 h-5 text-red-600" />
              ) : (
                <AlertTriangle className="w-5 h-5 text-amber-600" />
              )}
            </div>
            <div>
              <h3 className="text-sm font-bold text-gray-900">
                {isQuarantined ? 'Swarm Quarantine Active' : 'Community Threat Warning'}
              </h3>
              <p className="text-[11px] text-gray-500">Security consensus & threat report details</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-gray-400 hover:text-gray-600 hover:bg-black/5 transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 space-y-4 max-h-[75vh] overflow-y-auto">
          {/* File Target */}
          <div className="p-3 bg-gray-50 rounded-xl border border-gray-200 space-y-1">
            <div className="text-xs font-semibold text-gray-800 break-all">{file.name}</div>
            <div className="flex items-center justify-between text-[11px] text-gray-500 font-mono">
              <span className="truncate max-w-[280px]">BLAKE3: {file.hash}</span>
              <button
                onClick={handleCopyHash}
                className="inline-flex items-center gap-1 text-xs text-blue-600 hover:text-blue-800 shrink-0 font-sans"
              >
                {copied ? <Check className="w-3.5 h-3.5 text-green-600" /> : <Copy className="w-3.5 h-3.5" />}
                {copied ? 'Copied' : 'Copy'}
              </button>
            </div>
          </div>

          {/* Status Banner */}
          {isQuarantined ? (
            <div className="p-4 bg-red-50 border border-red-200 rounded-xl space-y-2">
              <div className="flex items-center gap-2 text-xs font-bold text-red-900">
                <AlertOctagon className="w-4 h-4 text-red-600 shrink-0" />
                <span>Strict Security Lockout Active</span>
              </div>
              <p className="text-xs text-red-700 leading-relaxed">
                This file has accumulated 5 or more community security reports within a 1-hour window.
                Opening with default OS viewers, saving to disk, and automatic peer background seeding are strictly disabled.
              </p>
            </div>
          ) : (
            <div className="p-4 bg-amber-50 border border-amber-200 rounded-xl space-y-2">
              <div className="flex items-center gap-2 text-xs font-bold text-amber-900">
                <AlertTriangle className="w-4 h-4 text-amber-600 shrink-0" />
                <span>Early Warning Flag ({reportCount} / 5 reports)</span>
              </div>
              <p className="text-xs text-amber-700 leading-relaxed">
                Peers on your campus network have flagged this file as potentially suspicious.
                If 5 distinct reports arrive within 1 hour, emergency swarm quarantine will be triggered automatically.
              </p>
            </div>
          )}

          {/* Community Reported Reasons */}
          <div className="space-y-2">
            <div className="text-xs font-bold text-gray-700 flex items-center gap-1.5">
              <Info className="w-3.5 h-3.5 text-gray-500" />
              <span>Reported Security Reasons:</span>
            </div>
            {loading ? (
              <div className="text-xs text-gray-400 italic py-2">Loading reports...</div>
            ) : reasons.length > 0 ? (
              <div className="space-y-1.5">
                {reasons.map((reason, idx) => (
                  <div
                    key={idx}
                    className="p-2.5 bg-gray-50 border border-gray-200 rounded-lg text-xs text-gray-800 flex items-start gap-2"
                  >
                    <span className="text-red-500 font-bold shrink-0">•</span>
                    <span className="leading-relaxed">{reason}</span>
                  </div>
                ))}
              </div>
            ) : (
              <div className="p-3 bg-gray-50 border border-gray-200 rounded-lg text-xs text-gray-500 italic">
                Reported as hazardous / abusive by community peers (no text description provided).
              </div>
            )}
          </div>

          {/* Report Form Toggle */}
          {showReportInput ? (
            <div className="p-3 bg-red-50/50 border border-red-200 rounded-xl space-y-2">
              <label className="text-xs font-semibold text-gray-700 block">
                Add your security report:
              </label>
              <textarea
                value={reportReason}
                onChange={(e) => setReportReason(e.target.value)}
                placeholder="Describe the threat (e.g., malware executable, phishing link, scam)..."
                rows={2}
                className="w-full text-xs p-2 rounded-lg border border-gray-300 focus:outline-hidden focus:ring-1 focus:ring-red-500"
              />
              <div className="flex justify-end gap-2">
                <button
                  type="button"
                  onClick={() => setShowReportInput(false)}
                  className="px-2.5 py-1 text-xs text-gray-600 hover:bg-gray-100 rounded-md"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={handleSubmitReport}
                  disabled={!reportReason.trim()}
                  className="px-3 py-1 text-xs font-semibold bg-red-600 text-white rounded-md hover:bg-red-700 disabled:opacity-50"
                >
                  Submit Report
                </button>
              </div>
            </div>
          ) : (
            <div className="pt-1">
              <button
                type="button"
                onClick={() => setShowReportInput(true)}
                className="text-xs text-red-600 hover:text-red-700 font-semibold underline underline-offset-2"
              >
                + Submit your own report for this file
              </button>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-3.5 bg-gray-50 border-t border-gray-100 flex items-center justify-between">
          {onBlockFile ? (
            <button
              onClick={() => {
                onClose();
                onBlockFile(file.hash);
              }}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold bg-red-50 text-red-700 hover:bg-red-100 border border-red-200 transition-colors"
            >
              <ShieldX className="w-3.5 h-3.5 text-red-600" />
              Block & Purge Locally
            </button>
          ) : (
            <div />
          )}

          <button
            onClick={onClose}
            className="px-4 py-1.5 rounded-lg text-xs font-semibold bg-gray-800 text-white hover:bg-gray-900 transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
