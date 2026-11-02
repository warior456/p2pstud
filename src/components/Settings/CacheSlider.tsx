import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { HardDrive, ShieldCheck, Zap } from 'lucide-react';
import { StorageStats } from '../../types';

interface CacheSliderProps {
  stats: StorageStats | null;
  onStatsUpdated?: () => void;
}

function formatGigabytes(bytes: number): string {
  return (bytes / (1024 * 1024 * 1024)).toFixed(2);
}

export const CacheSlider: React.FC<CacheSliderProps> = ({ stats, onStatsUpdated }) => {
  const [sliderGb, setSliderGb] = useState<number>(5);
  const [fullMirror, setFullMirror] = useState<boolean>(false);
  const [saving, setSaving] = useState<boolean>(false);

  useEffect(() => {
    if (stats) {
      setFullMirror(stats.full_mirror);
      if (!stats.full_mirror && stats.quota_bytes < 1000 * 1024 * 1024 * 1024) {
        const gb = Math.round(stats.quota_bytes / (1024 * 1024 * 1024));
        setSliderGb(Math.max(3, Math.min(50, gb)));
      }
    }
  }, [stats]);

  const handleSliderChange = async (newGb: number) => {
    setSliderGb(newGb);
    setSaving(true);
    try {
      const bytes = newGb * 1024 * 1024 * 1024;
      await invoke('set_cache_limit', { bytes, fullMirror: false });
      if (onStatsUpdated) onStatsUpdated();
    } catch (err) {
      console.error('Failed to set cache limit:', err);
    } finally {
      setSaving(false);
    }
  };

  const handleFullMirrorToggle = async (enabled: boolean) => {
    setFullMirror(enabled);
    setSaving(true);
    try {
      const bytes = enabled ? null : sliderGb * 1024 * 1024 * 1024;
      await invoke('set_cache_limit', { bytes, fullMirror: enabled });
      if (onStatsUpdated) onStatsUpdated();
    } catch (err) {
      console.error('Failed to update mirror mode:', err);
    } finally {
      setSaving(false);
    }
  };

  const usedGb = stats ? formatGigabytes(stats.used_bytes) : '0.00';
  const pinnedGb = stats ? formatGigabytes(stats.pinned_bytes) : '0.00';

  return (
    <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-sm max-w-xl mx-auto space-y-6">
      <div className="flex items-center gap-3 border-b border-gray-100 pb-4">
        <HardDrive className="w-6 h-6 text-blue-600" />
        <div>
          <h2 className="text-lg font-bold text-gray-900">Host-to-Play Cache Allocation</h2>
          <p className="text-xs text-gray-500">
            Dedicating local disk space seeds course materials to peers on campus.
          </p>
        </div>
      </div>

      {/* Usage breakdown */}
      <div className="grid grid-cols-2 gap-4 bg-gray-50 p-4 rounded-lg">
        <div>
          <span className="text-xs text-gray-500 font-medium">Currently Used</span>
          <p className="text-xl font-bold text-gray-900 mt-1">{usedGb} GB</p>
        </div>
        <div>
          <span className="text-xs text-gray-500 font-medium">Pinned (Protected)</span>
          <p className="text-xl font-bold text-green-700 mt-1">{pinnedGb} GB</p>
        </div>
      </div>

      {/* Quota slider */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <label htmlFor="cache-slider" className="text-sm font-semibold text-gray-800">
            Cache Quota Limit
          </label>
          <span className="text-sm font-bold text-blue-600">
            {fullMirror ? 'Unlimited (Full Mirror)' : `${sliderGb} GB`}
          </span>
        </div>

        <input
          id="cache-slider"
          type="range"
          min={3}
          max={50}
          step={1}
          disabled={fullMirror || saving}
          value={sliderGb}
          onChange={(e) => handleSliderChange(Number(e.target.value))}
          className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-blue-600 disabled:opacity-40"
        />

        <div className="flex justify-between text-xs text-gray-400">
          <span>Min: 3 GB</span>
          <span>Recommended: 5 GB</span>
          <span>50 GB</span>
        </div>
      </div>

      {/* Full mirror toggle */}
      <div className="flex items-start gap-3 p-4 border border-blue-100 bg-blue-50/50 rounded-lg">
        <input
          id="full-mirror"
          type="checkbox"
          checked={fullMirror}
          disabled={saving}
          onChange={(e) => handleFullMirrorToggle(e.target.checked)}
          className="mt-1 w-4 h-4 text-blue-600 rounded border-gray-300 focus:ring-blue-500 cursor-pointer"
        />
        <label htmlFor="full-mirror" className="cursor-pointer">
          <div className="flex items-center gap-1.5 font-semibold text-sm text-gray-900">
            <Zap className="w-4 h-4 text-amber-500 fill-amber-500" />
            Enable Full Mirror Mode
          </div>
          <p className="text-xs text-gray-600 mt-0.5">
            Disables the automatic LRU eviction worker and auto-downloads all course workspace blobs
            locally to provide 100% offline redundancy.
          </p>
        </label>
      </div>

      <div className="flex items-center gap-2 text-xs text-gray-400">
        <ShieldCheck className="w-4 h-4 text-green-600 flex-shrink-0" />
        <span>Oldest unpinned items are automatically evicted when cache reaches 90% capacity.</span>
      </div>
    </div>
  );
};

