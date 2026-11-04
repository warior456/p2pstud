import { useState } from 'react';
import { HardDrive, CheckCircle2 } from 'lucide-react';
import { CredentialsStep } from './CredentialsStep';

interface OnboardingSimpleProps {
  onFinish: (settings: { passphrase: string; cacheQuotaBytes: number; fullMirror: boolean }) => void;
  onBackToModeSelect: () => void;
}

export function OnboardingSimple({ onFinish, onBackToModeSelect }: OnboardingSimpleProps) {
  const [step, setStep] = useState<'passphrase' | 'quota'>('passphrase');
  const [passphrase, setPassphrase] = useState('');
  const [quotaGb, setQuotaGb] = useState<number>(5);
  const [fullMirror, setFullMirror] = useState<boolean>(false);

  const handlePassphraseComplete = (val: string) => {
    setPassphrase(val);
    setStep('quota');
  };

  const handleFinish = () => {
    const quotaBytes = fullMirror ? 0 : quotaGb * 1024 * 1024 * 1024;
    onFinish({
      passphrase,
      cacheQuotaBytes: quotaBytes,
      fullMirror,
    });
  };

  if (step === 'passphrase') {
    return (
      <CredentialsStep
        initialValue={passphrase}
        onComplete={handlePassphraseComplete}
        onBack={onBackToModeSelect}
      />
    );
  }

  return (
    <div className="space-y-6">
      <div className="text-center space-y-2">
        <div className="w-12 h-12 bg-indigo-500/10 text-indigo-400 rounded-xl flex items-center justify-center mx-auto border border-indigo-500/20">
          <HardDrive className="w-6 h-6" />
        </div>
        <h2 className="text-lg font-semibold text-slate-100">Cooperative Storage Quota</h2>
        <p className="text-xs text-slate-400 max-w-sm mx-auto">
          Choose the local disk space you donate to store and seed course documents. You can adjust this anytime in Settings.
        </p>
      </div>

      <div className="bg-slate-800/60 border border-slate-700/60 rounded-xl p-5 space-y-4">
        <div className="flex items-center justify-between text-xs">
          <span className="text-slate-400">Cache Size</span>
          <span className="font-semibold text-blue-400">
            {fullMirror ? 'Full Mirror (Download All Files)' : `${quotaGb} GB`}
          </span>
        </div>

        {!fullMirror && (
          <input
            type="range"
            min={3}
            max={50}
            step={1}
            value={quotaGb}
            onChange={(e) => setQuotaGb(parseInt(e.target.value, 10))}
            className="w-full accent-blue-500 cursor-pointer"
          />
        )}

        <div className="flex items-center gap-2 pt-2 border-t border-slate-700/60">
          <input
            type="checkbox"
            id="full_mirror_checkbox"
            checked={fullMirror}
            onChange={(e) => setFullMirror(e.target.checked)}
            className="rounded border-slate-700 text-blue-600 focus:ring-0"
          />
          <label htmlFor="full_mirror_checkbox" className="text-xs text-slate-300 cursor-pointer">
            Full Mirror Mode (permanently mirror the entire course catalog)
          </label>
        </div>
      </div>

      <div className="flex items-center justify-between pt-4 border-t border-slate-800">
        <button
          type="button"
          onClick={() => setStep('passphrase')}
          className="px-4 py-2 text-xs text-slate-400 hover:text-slate-200 transition"
        >
          Back
        </button>
        <button
          type="button"
          onClick={handleFinish}
          className="px-5 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-xs font-medium transition flex items-center gap-1.5 shadow-sm"
        >
          <span>Complete Setup</span>
          <CheckCircle2 className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
}

