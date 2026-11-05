import { useState } from 'react';
import { CheckCircle2, ChevronRight } from 'lucide-react';
import { CredentialsStep } from './CredentialsStep';

interface OnboardingAdvancedProps {
  onFinish: (settings: {
    passphrase: string;
    cacheQuotaBytes: number;
    fullMirror: boolean;
    campusModeDefault: boolean;
    warnMetered: boolean;
    bandwidthCap: string;
    pauseOnBattery: boolean;
    relayUrl: string;
  }) => void;
  onBackToModeSelect: () => void;
}

export function OnboardingAdvanced({ onFinish, onBackToModeSelect }: OnboardingAdvancedProps) {
  const [step, setStep] = useState<number>(1);
  const [passphrase, setPassphrase] = useState<string>('');
  const [quotaGb, setQuotaGb] = useState<number>(5);
  const [fullMirror, setFullMirror] = useState<boolean>(false);
  const [campusModeDefault, setCampusModeDefault] = useState<boolean>(true);
  const [warnMetered, setWarnMetered] = useState<boolean>(true);
  const [bandwidthCap, setBandwidthCap] = useState<string>('512kb');
  const [pauseOnBattery, setPauseOnBattery] = useState<boolean>(true);
  const [relayUrl, setRelayUrl] = useState<string>('https://use1-1.relay.iroh.network/');

  const handleFinish = () => {
    const quotaBytes = fullMirror ? 0 : quotaGb * 1024 * 1024 * 1024;
    onFinish({
      passphrase,
      cacheQuotaBytes: quotaBytes,
      fullMirror,
      campusModeDefault,
      warnMetered,
      bandwidthCap,
      pauseOnBattery,
      relayUrl,
    });
  };

  return (
    <div className="space-y-6">
      {/* Progress header */}
      <div className="flex items-center justify-between text-xs text-slate-400 border-b border-slate-800 pb-3">
        <span className="font-medium text-slate-200">Step {step} of 5: Advanced Setup</span>
        <span>
          {step === 1 && 'Swarm Security'}
          {step === 2 && 'Storage Quota'}
          {step === 3 && 'Network Profile'}
          {step === 4 && 'Bandwidth Controls'}
          {step === 5 && 'Relay Server'}
        </span>
      </div>

      {step === 1 && (
        <CredentialsStep
          initialValue={passphrase}
          onComplete={(val) => {
            setPassphrase(val);
            setStep(2);
          }}
          onBack={onBackToModeSelect}
        />
      )}

      {step === 2 && (
        <div className="space-y-5">
          <div className="text-center space-y-1">
            <h2 className="text-base font-semibold text-slate-100">Storage Quota</h2>
            <p className="text-xs text-slate-400">Dedicate storage to seed academic documents.</p>
          </div>

          <div className="bg-slate-800/60 border border-slate-700/60 rounded-xl p-4 space-y-3">
            <div className="flex justify-between text-xs">
              <span className="text-slate-400">Allocated Space</span>
              <span className="font-medium text-blue-400">
                {fullMirror ? 'Full Mirror' : `${quotaGb} GB`}
              </span>
            </div>
            {!fullMirror && (
              <input
                type="range"
                min={3}
                max={100}
                value={quotaGb}
                onChange={(e) => setQuotaGb(parseInt(e.target.value, 10))}
                className="w-full accent-blue-500 cursor-pointer"
              />
            )}
            <label className="flex items-center gap-2 pt-2 border-t border-slate-700/60 text-xs text-slate-300 cursor-pointer">
              <input
                type="checkbox"
                checked={fullMirror}
                onChange={(e) => setFullMirror(e.target.checked)}
                className="rounded border-slate-700 text-blue-600 focus:ring-0"
              />
              <span>Full Mirror Mode (download all files)</span>
            </label>
          </div>

          <div className="flex justify-between pt-4 border-t border-slate-800">
            <button
              onClick={() => setStep(1)}
              className="px-4 py-2 text-xs text-slate-400 hover:text-slate-200"
            >
              Back
            </button>
            <button
              onClick={() => setStep(3)}
              className="px-5 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-xs font-medium flex items-center gap-1.5"
            >
              <span>Next</span>
              <ChevronRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {step === 3 && (
        <div className="space-y-5">
          <div className="text-center space-y-1">
            <h2 className="text-base font-semibold text-slate-100">Campus Stealth & Mobile Policies</h2>
            <p className="text-xs text-slate-400">Optimize traffic patterns for restricted university Wi-Fi.</p>
          </div>

          <div className="space-y-3">
            <label className="flex items-start gap-3 bg-slate-800/60 border border-slate-700/60 rounded-xl p-3.5 cursor-pointer">
              <input
                type="checkbox"
                checked={campusModeDefault}
                onChange={(e) => setCampusModeDefault(e.target.checked)}
                className="mt-0.5 rounded border-slate-700 text-blue-600 focus:ring-0"
              />
              <div className="space-y-1">
                <span className="text-xs font-medium text-slate-200 block">
                  Enable Campus Stealth Mode on Eduroam / University Wi-Fi
                </span>
                <span className="text-[11px] text-slate-400 block">
                  Disables DHT and local broadcast scanning on campus to protect stealth and camouflage on TLS port 443.
                </span>
              </div>
            </label>

            <label className="flex items-start gap-3 bg-slate-800/60 border border-slate-700/60 rounded-xl p-3.5 cursor-pointer">
              <input
                type="checkbox"
                checked={warnMetered}
                onChange={(e) => setWarnMetered(e.target.checked)}
                className="mt-0.5 rounded border-slate-700 text-blue-600 focus:ring-0"
              />
              <div className="space-y-1">
                <span className="text-xs font-medium text-slate-200 block">
                  Warn on Mobile Hotspots & Metered Connections
                </span>
                <span className="text-[11px] text-slate-400 block">
                  Pauses background seeding and auto-fill replication when tethered to mobile data.
                </span>
              </div>
            </label>
          </div>

          <div className="flex justify-between pt-4 border-t border-slate-800">
            <button
              onClick={() => setStep(2)}
              className="px-4 py-2 text-xs text-slate-400 hover:text-slate-200"
            >
              Back
            </button>
            <button
              onClick={() => setStep(4)}
              className="px-5 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-xs font-medium flex items-center gap-1.5"
            >
              <span>Next</span>
              <ChevronRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {step === 4 && (
        <div className="space-y-5">
          <div className="text-center space-y-1">
            <h2 className="text-base font-semibold text-slate-100">Bandwidth & Battery Defense</h2>
            <p className="text-xs text-slate-400">Configure background upload and power limits.</p>
          </div>

          <div className="space-y-4 bg-slate-800/60 border border-slate-700/60 rounded-xl p-4">
            <div className="space-y-2">
              <label className="block text-xs font-medium text-slate-300">
                Background Seeding Speed Limit
              </label>
              <select
                value={bandwidthCap}
                onChange={(e) => setBandwidthCap(e.target.value)}
                className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-blue-500"
              >
                <option value="128kb">128 KB/s (Minimal impact during lectures)</option>
                <option value="512kb">512 KB/s (Recommended for campus networks)</option>
                <option value="2mb">2 MB/s (Standard broadband)</option>
                <option value="unlimited">Unlimited (Fast LAN / Home fiber)</option>
              </select>
            </div>

            <label className="flex items-center gap-2 pt-2 border-t border-slate-700/60 text-xs text-slate-300 cursor-pointer">
              <input
                type="checkbox"
                checked={pauseOnBattery}
                onChange={(e) => setPauseOnBattery(e.target.checked)}
                className="rounded border-slate-700 text-blue-600 focus:ring-0"
              />
              <span>Pause background seeding when running on battery</span>
            </label>
          </div>

          <div className="flex justify-between pt-4 border-t border-slate-800">
            <button
              onClick={() => setStep(3)}
              className="px-4 py-2 text-xs text-slate-400 hover:text-slate-200"
            >
              Back
            </button>
            <button
              onClick={() => setStep(5)}
              className="px-5 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-xs font-medium flex items-center gap-1.5"
            >
              <span>Next</span>
              <ChevronRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {step === 5 && (
        <div className="space-y-5">
          <div className="text-center space-y-1">
            <h2 className="text-base font-semibold text-slate-100">Relay Server Setup</h2>
            <p className="text-xs text-slate-400">Configure relay fallback for restrictive firewalls.</p>
          </div>

          <div className="space-y-3 bg-slate-800/60 border border-slate-700/60 rounded-xl p-4">
            <div className="space-y-2">
              <label className="block text-xs font-medium text-slate-300">
                Relay Server URL
              </label>
              <input
                type="text"
                value={relayUrl}
                onChange={(e) => setRelayUrl(e.target.value)}
                placeholder="https://euw1-1.relay.iroh.network/"
                className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-blue-500"
              />
              <p className="text-[11px] text-slate-400">
                Leave as default or enter your campus group's private VPS anchor relay URL.
              </p>
            </div>
          </div>

          <div className="flex justify-between pt-4 border-t border-slate-800">
            <button
              onClick={() => setStep(4)}
              className="px-4 py-2 text-xs text-slate-400 hover:text-slate-200"
            >
              Back
            </button>
            <button
              onClick={handleFinish}
              className="px-5 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-xs font-medium flex items-center gap-1.5 shadow-sm"
            >
              <span>Complete Setup</span>
              <CheckCircle2 className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

