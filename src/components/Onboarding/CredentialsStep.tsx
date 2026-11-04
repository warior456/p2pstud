import React, { useState } from 'react';
import { KeyRound, ShieldCheck, Info } from 'lucide-react';

interface CredentialsStepProps {
  initialValue?: string;
  onComplete: (passphrase: string) => void;
  onBack?: () => void;
}

export function CredentialsStep({ initialValue = '', onComplete, onBack }: CredentialsStepProps) {
  const [passphrase, setPassphrase] = useState(initialValue);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onComplete(passphrase.trim());
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      <div className="text-center space-y-2">
        <div className="w-12 h-12 bg-blue-500/10 text-blue-400 rounded-xl flex items-center justify-center mx-auto border border-blue-500/20">
          <KeyRound className="w-6 h-6" />
        </div>
        <h2 className="text-lg font-semibold text-slate-100">Study Group Passphrase</h2>
        <p className="text-xs text-slate-400 max-w-sm mx-auto">
          This shared secret cryptographically isolates your study group's network. class members using the same passphrase automatically discover and share notes with each other.
        </p>
      </div>

      <div className="space-y-2">
        <label className="block text-xs font-medium text-slate-300">
          Passphrase / Network Secret
        </label>
        <input
          type="text"
          value={passphrase}
          onChange={(e) => setPassphrase(e.target.value)}
          placeholder="e.g. compsci-2026-fall (leave blank for public swarm)"
          className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500 transition"
          autoFocus
        />
        <div className="bg-slate-800/60 border border-slate-700/60 rounded-lg p-2.5 flex items-start gap-2 text-xs text-slate-400">
          <Info className="w-4 h-4 text-blue-400 shrink-0 mt-0.5" />
          <span>
            <strong>Standalone P2P Mode:</strong> If your university has not set up a central authentication server or campus email OTP, standalone mode works out-of-the-box without any external dependencies.
          </span>
        </div>
      </div>

      <div className="flex items-center justify-between pt-4 border-t border-slate-800">
        {onBack ? (
          <button
            type="button"
            onClick={onBack}
            className="px-4 py-2 text-xs text-slate-400 hover:text-slate-200 transition"
          >
            Back
          </button>
        ) : <div />}
        <button
          type="submit"
          className="px-5 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-xs font-medium transition flex items-center gap-1.5 shadow-sm"
        >
          <span>Continue</span>
          <ShieldCheck className="w-4 h-4" />
        </button>
      </div>
    </form>
  );
}

