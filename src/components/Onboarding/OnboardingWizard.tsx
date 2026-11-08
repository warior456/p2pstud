import { useState } from 'react';
import { Sparkles, Sliders, BookOpen } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { OnboardingSimple } from './OnboardingSimple';
import { OnboardingAdvanced } from './OnboardingAdvanced';

interface OnboardingWizardProps {
  onComplete: () => void;
}

export function OnboardingWizard({ onComplete }: OnboardingWizardProps) {
  const [mode, setMode] = useState<'select' | 'simple' | 'advanced'>('select');

  const finishWizard = async (settings: {
    passphrase?: string;
    cacheQuotaBytes?: number;
    fullMirror?: boolean;
    relayUrl?: string;
    campusModeDefault?: boolean;
    bandwidthCap?: string;
    pauseOnBattery?: boolean;
    warnMetered?: boolean;
  }) => {
    try {
      if (settings.passphrase !== undefined && settings.passphrase.trim()) {
        await invoke('set_network_secret', { secret: settings.passphrase.trim() });
      }

      if (settings.fullMirror !== undefined) {
        await invoke('set_full_mirror', { enabled: Boolean(settings.fullMirror) });
      }

      if (settings.cacheQuotaBytes !== undefined && !settings.fullMirror) {
        await invoke('set_cache_limit', { limitBytes: settings.cacheQuotaBytes });
      }

      if (settings.campusModeDefault !== undefined) {
        await invoke('set_network_profile_override', {
          profile: settings.campusModeDefault ? 'campus' : 'auto',
        });
      }

      if (settings.bandwidthCap) {
        await invoke('set_bandwidth_cap', { cap: settings.bandwidthCap });
      }

      if (settings.pauseOnBattery !== undefined) {
        await invoke('set_pause_on_battery', { pause: settings.pauseOnBattery });
      }

      if (settings.warnMetered !== undefined) {
        await invoke('set_warn_metered', { warn: settings.warnMetered });
      }

      // Persist onboarding complete flag explicitly in SQLite
      await invoke('set_setting', {
        key: 'onboarding_complete',
        value: 'true',
      });

      // Also persist all current app settings in one go
      await invoke('update_app_settings', {
        settings: {
          relay_url: settings.relayUrl || 'https://euc1-1.relay.n0.iroh.link/',
          cache_limit_bytes: settings.cacheQuotaBytes || 5368709120,
          full_mirror: Boolean(settings.fullMirror),
          network_secret: settings.passphrase || '',
          onboarding_complete: 'true',
          bandwidth_cap: settings.bandwidthCap || '512kb',
          pause_on_battery: settings.pauseOnBattery !== undefined ? settings.pauseOnBattery : true,
          warn_metered: settings.warnMetered !== undefined ? settings.warnMetered : true,
        },
      });
    } catch (err) {
      console.error('Failed to save onboarding settings:', err);
    } finally {
      onComplete();
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-slate-950/95 flex items-center justify-center p-4 backdrop-blur-md">
      <div className="w-full max-w-lg bg-slate-900 border border-slate-800 rounded-2xl shadow-2xl p-6 space-y-6">
        {mode === 'select' && (
          <div className="space-y-6 text-center">
            <div className="w-14 h-14 bg-blue-600/10 text-blue-400 rounded-2xl flex items-center justify-center mx-auto border border-blue-500/20 shadow-inner">
              <BookOpen className="w-7 h-7" />
            </div>

            <div className="space-y-2">
              <h1 className="text-xl font-bold text-slate-100 tracking-tight">
                Welcome to P2P Campus Workspace
              </h1>
              <p className="text-xs text-slate-400 max-w-md mx-auto leading-relaxed">
                Cooperative "Host-to-Play" academic file sharing. Dedicate a small portion of disk space to cache and seed course materials across university networks.
              </p>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 pt-2">
              <button
                type="button"
                onClick={() => setMode('simple')}
                className="bg-slate-800/80 hover:bg-slate-800 border border-slate-700/80 hover:border-blue-500/50 rounded-xl p-4 text-left space-y-2 transition group"
              >
                <div className="w-8 h-8 rounded-lg bg-blue-500/10 text-blue-400 flex items-center justify-center group-hover:bg-blue-500/20 transition">
                  <Sparkles className="w-4 h-4" />
                </div>
                <div>
                  <h3 className="text-xs font-semibold text-slate-100 group-hover:text-blue-300 transition">
                    Quick Setup (Recommended)
                  </h3>
                  <p className="text-[11px] text-slate-400 mt-1 leading-normal">
                    Enter your class passphrase and confirm storage size in 2 simple steps.
                  </p>
                </div>
              </button>

              <button
                type="button"
                onClick={() => setMode('advanced')}
                className="bg-slate-800/80 hover:bg-slate-800 border border-slate-700/80 hover:border-indigo-500/50 rounded-xl p-4 text-left space-y-2 transition group"
              >
                <div className="w-8 h-8 rounded-lg bg-indigo-500/10 text-indigo-400 flex items-center justify-center group-hover:bg-indigo-500/20 transition">
                  <Sliders className="w-4 h-4" />
                </div>
                <div>
                  <h3 className="text-xs font-semibold text-slate-100 group-hover:text-indigo-300 transition">
                    Custom / Advanced Setup
                  </h3>
                  <p className="text-[11px] text-slate-400 mt-1 leading-normal">
                    Fine-tune Eduroam stealth profiles, custom relay URLs, and bandwidth limits.
                  </p>
                </div>
              </button>
            </div>
          </div>
        )}

        {mode === 'simple' && (
          <OnboardingSimple
            onFinish={(res) => finishWizard({
              passphrase: res.passphrase,
              cacheQuotaBytes: res.cacheQuotaBytes,
              fullMirror: res.fullMirror,
            })}
            onBackToModeSelect={() => setMode('select')}
          />
        )}

        {mode === 'advanced' && (
          <OnboardingAdvanced
            onFinish={(res) => finishWizard({
              passphrase: res.passphrase,
              cacheQuotaBytes: res.cacheQuotaBytes,
              fullMirror: res.fullMirror,
              campusModeDefault: res.campusModeDefault,
              relayUrl: res.relayUrl,
              bandwidthCap: res.bandwidthCap,
              pauseOnBattery: res.pauseOnBattery,
              warnMetered: res.warnMetered,
            })}
            onBackToModeSelect={() => setMode('select')}
          />
        )}
      </div>
    </div>
  );
}

