import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Gauge, BatteryCharging, Wifi, ArrowUpRight, ArrowDownLeft, Shield, CheckCircle2, Leaf, Power } from 'lucide-react';
import { AppSettings, BatteryMode, StorageStats } from '../../types';

interface BandwidthSettingsProps {
  stats?: StorageStats | null;
  onStatsUpdated?: () => void;
}

export const BandwidthSettings: React.FC<BandwidthSettingsProps> = ({ stats }) => {
  const [bandwidthCap, setBandwidthCap] = useState<string>('512kb');
  const [batteryMode, setBatteryMode] = useState<BatteryMode>('pause');
  const [warnMetered, setWarnMetered] = useState<boolean>(true);
  const [saving, setSaving] = useState<boolean>(false);
  const [savedSuccess, setSavedSuccess] = useState<boolean>(false);

  useEffect(() => {
    invoke<AppSettings>('get_app_settings')
      .then((settings) => {
        if (settings?.bandwidth_cap) setBandwidthCap(settings.bandwidth_cap);
        if (settings?.battery_mode) {
          setBatteryMode(settings.battery_mode);
        } else if (settings?.pause_on_battery !== undefined) {
          // Migrate older builds that only had the boolean toggle.
          setBatteryMode(settings.pause_on_battery ? 'pause' : 'normal');
        }
        if (settings?.warn_metered !== undefined) setWarnMetered(settings.warn_metered);
      })
      .catch((err) => console.warn('Failed to load bandwidth settings:', err));
  }, []);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setSavedSuccess(false);
    try {
      await invoke('set_bandwidth_cap', { cap: bandwidthCap });
      await invoke('set_battery_mode', { mode: batteryMode });
      await invoke('set_warn_metered', { warn: warnMetered });
      setSavedSuccess(true);
      setTimeout(() => setSavedSuccess(false), 3000);
    } catch (err) {
      alert(`Failed to save settings: ${err}`);
    } finally {
      setSaving(false);
    }
  };

  const formatSpeed = (bytesPerSec?: number) => {
    if (!bytesPerSec || bytesPerSec === 0) return '0.0 KB/s';
    if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
    return `${(bytesPerSec / (1024 * 1024)).toFixed(2)} MB/s`;
  };

  return (
    <div className="max-w-3xl mx-auto space-y-6">
      {/* Real-time Transfer Speed Monitor Card */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
        <div className="flex items-center justify-between border-b border-gray-100 pb-3">
          <div className="flex items-center gap-2.5">
            <Gauge className="w-5 h-5 text-indigo-600" />
            <div>
              <h3 className="text-sm font-bold text-gray-900">Live Swarm Bandwidth</h3>
              <p className="text-xs text-gray-500">Real-time peer-to-peer upload and download throughput.</p>
            </div>
          </div>
          <span className="text-[11px] font-medium text-emerald-600 bg-emerald-50 border border-emerald-200 px-2.5 py-1 rounded-full flex items-center gap-1.5">
            <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
            Live Monitoring
          </span>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="p-4 bg-slate-50 border border-slate-200 rounded-xl space-y-1">
            <div className="flex items-center justify-between text-xs font-semibold text-slate-500">
              <span className="flex items-center gap-1.5">
                <ArrowUpRight className="w-4 h-4 text-emerald-600" />
                Upload / Seeding Rate
              </span>
            </div>
            <div className="text-xl font-bold font-mono text-slate-900">
              {formatSpeed(stats?.upload_bytes_per_sec)}
            </div>
            <p className="text-[10px] text-slate-400">Serving course chunks to peers</p>
          </div>

          <div className="p-4 bg-slate-50 border border-slate-200 rounded-xl space-y-1">
            <div className="flex items-center justify-between text-xs font-semibold text-slate-500">
              <span className="flex items-center gap-1.5">
                <ArrowDownLeft className="w-4 h-4 text-blue-600" />
                Download / Fetch Rate
              </span>
            </div>
            <div className="text-xl font-bold font-mono text-slate-900">
              {formatSpeed(stats?.download_bytes_per_sec)}
            </div>
            <p className="text-[10px] text-slate-400">Receiving course documents</p>
          </div>
        </div>
      </div>

      {/* Bandwidth & Battery Defense Form */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-5">
        <div className="flex items-center gap-2.5 border-b border-gray-100 pb-3">
          <Shield className="w-5 h-5 text-indigo-600" />
          <div>
            <h3 className="text-sm font-bold text-gray-900">Bandwidth & Battery Defense Controls</h3>
            <p className="text-xs text-gray-500">
              Protect your laptop's battery life, throttle background uploads, and avoid cellular hotspot data consumption.
            </p>
          </div>
        </div>

        <form onSubmit={handleSave} className="space-y-5">
          {/* Speed Limit Dropdown */}
          <div className="space-y-1.5">
            <label className="text-xs font-bold text-gray-800 flex items-center gap-1.5">
              <Gauge className="w-4 h-4 text-indigo-500" />
              Background Seeding Speed Limit
            </label>
            <p className="text-[11px] text-gray-500 leading-relaxed">
              Caps unpinned cache replication and chunk seeding so it never saturates university Wi-Fi (Eduroam) or home connections.
            </p>
            <select
              value={bandwidthCap}
              onChange={(e) => setBandwidthCap(e.target.value)}
              className="w-full text-xs bg-gray-50 border border-gray-200 rounded-lg px-3 py-2.5 text-gray-800 font-medium focus:bg-white focus:border-indigo-500 focus:outline-none transition-all cursor-pointer"
            >
              <option value="128kb">128 KB/s — Ultra-stealth / strict lecture hall mode</option>
              <option value="512kb">512 KB/s — Recommended for university campus networks</option>
              <option value="1mb">1.0 MB/s — Balanced residential broadband / library</option>
              <option value="2mb">2.0 MB/s — High-speed broadband / dorm Wi-Fi</option>
              <option value="unlimited">Unlimited — Uncapped high-speed LAN synchronization</option>
            </select>
          </div>

          <div className="space-y-3 pt-3 border-t border-gray-100">
            {/* Battery Behaviour Selector (three modes) */}
            <div className="space-y-1.5">
              <label className="text-xs font-bold text-gray-800 flex items-center gap-1.5">
                <BatteryCharging className="w-4 h-4 text-amber-500" />
                Battery Behaviour
              </label>
              <p className="text-[11px] text-gray-500 leading-relaxed">
                Choose what P2PStud may do while your laptop is unplugged. Manual downloads always work.
              </p>

              <div className="grid gap-2">
                {(
                  [
                    {
                      value: 'pause' as BatteryMode,
                      icon: Power,
                      title: 'Pause background seeding',
                      desc: 'Stops all background seeding and cache auto-fill on battery. Lowest power usage; manual downloads still work on demand.',
                    },
                    {
                      value: 'eco' as BatteryMode,
                      icon: Leaf,
                      title: 'Eco — protect only scarce files',
                      desc: 'Keeps rare single-seed course files alive, but limits replication to one file at a time, at most once every 90 seconds, capped at 128 KB/s.',
                    },
                    {
                      value: 'normal' as BatteryMode,
                      icon: BatteryCharging,
                      title: 'Full background activity',
                      desc: 'Seeds and replicates normally even on battery. Fastest swarm contribution, but uses more power.',
                    },
                  ] as const
                ).map((option) => {
                  const Icon = option.icon;
                  const active = batteryMode === option.value;
                  return (
                    <label
                      key={option.value}
                      className={`flex items-start gap-3 p-3 rounded-xl border transition cursor-pointer select-none ${
                        active
                          ? 'bg-indigo-50 border-indigo-300 ring-1 ring-indigo-200'
                          : 'bg-gray-50 hover:bg-gray-100/80 border-gray-200'
                      }`}
                    >
                      <input
                        type="radio"
                        name="battery_mode"
                        value={option.value}
                        checked={active}
                        onChange={() => setBatteryMode(option.value)}
                        className="mt-0.5 border-gray-300 text-indigo-600 focus:ring-0 cursor-pointer"
                      />
                      <div className="space-y-0.5">
                        <span className="text-xs font-bold text-gray-900 flex items-center gap-1.5">
                          <Icon className={`w-4 h-4 ${active ? 'text-indigo-600' : 'text-gray-400'}`} />
                          {option.title}
                        </span>
                        <p className="text-[11px] text-gray-500 leading-relaxed">{option.desc}</p>
                      </div>
                    </label>
                  );
                })}
              </div>
            </div>

            {/* Metered / Hotspot Toggle */}
            <label className="flex items-start gap-3 p-3 bg-gray-50 hover:bg-gray-100/80 rounded-xl border border-gray-200 transition cursor-pointer select-none">
              <input
                type="checkbox"
                checked={warnMetered}
                onChange={(e) => setWarnMetered(e.target.checked)}
                className="mt-0.5 rounded border-gray-300 text-indigo-600 focus:ring-0 cursor-pointer"
              />
              <div className="space-y-0.5">
                <span className="text-xs font-bold text-gray-900 flex items-center gap-1.5">
                  <Wifi className="w-4 h-4 text-blue-500" />
                  Warn on Mobile Hotspots & Metered Connections
                </span>
                <p className="text-[11px] text-gray-500 leading-relaxed">
                  Pauses background seeding and multi-gigabyte cache replication when tethered to mobile hotspot data or a metered Wi-Fi network.
                </p>
              </div>
            </label>
          </div>

          <div className="flex items-center gap-3 pt-2">
            <button
              type="submit"
              disabled={saving}
              className="px-5 py-2.5 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg text-xs font-semibold shadow-xs transition-colors disabled:opacity-50 flex items-center gap-2 cursor-pointer"
            >
              <Gauge className="w-4 h-4" />
              {saving ? 'Saving...' : 'Save Bandwidth & Battery Settings'}
            </button>
            {savedSuccess && (
              <span className="text-xs text-emerald-600 font-semibold flex items-center gap-1">
                <CheckCircle2 className="w-4 h-4 text-emerald-500" />
                Settings saved successfully!
              </span>
            )}
          </div>
        </form>
      </div>
    </div>
  );
};
