import { useState, useEffect } from 'react';
import { Zap, X } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { SoleSeedFile } from '../types';

interface SoleSeedBannerProps {
  onShowSoleSeeds?: () => void;
}

export function SoleSeedBanner({ onShowSoleSeeds }: SoleSeedBannerProps) {
  const [soleSeedFiles, setSoleSeedFiles] = useState<SoleSeedFile[]>([]);
  const [isDismissed, setIsDismissed] = useState<boolean>(false);

  const fetchSoleSeeds = async () => {
    try {
      const files = await invoke<SoleSeedFile[]>('get_sole_seed_files');
      setSoleSeedFiles(files || []);
    } catch (err) {
      console.warn('Failed to fetch sole seed files:', err);
    }
  };

  useEffect(() => {
    fetchSoleSeeds();
    const interval = setInterval(fetchSoleSeeds, 60000);
    return () => clearInterval(interval);
  }, []);

  if (isDismissed || soleSeedFiles.length === 0) {
    return null;
  }

  const count = soleSeedFiles.length;

  return (
    <div className="bg-amber-50 border-b border-amber-200 px-4 py-2 flex items-center justify-between text-xs text-amber-900 shadow-2xs">
      <div className="flex items-center gap-2">
        <Zap className="w-4 h-4 text-amber-600 animate-pulse shrink-0" />
        <span className="font-semibold text-amber-900">
          ⚡ You are the only online copy of {count} course {count === 1 ? 'file' : 'files'}. Keep the app running to protect them.
        </span>
      </div>
      <div className="flex items-center gap-2">
        {onShowSoleSeeds && (
          <button
            onClick={onShowSoleSeeds}
            className="px-2.5 py-1 bg-amber-100 hover:bg-amber-200 text-amber-900 border border-amber-300 rounded font-medium text-xs transition cursor-pointer"
          >
            Show Files
          </button>
        )}
        <button
          onClick={() => setIsDismissed(true)}
          className="p-1 hover:bg-amber-200/70 text-amber-800 rounded transition cursor-pointer"
          title="Dismiss alert for this session"
        >
          <X className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
}
