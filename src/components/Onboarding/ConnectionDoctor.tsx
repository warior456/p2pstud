import { useState, useEffect } from 'react';
import { CheckCircle2, XCircle, RefreshCw, AlertCircle, Stethoscope } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { DoctorTestResult } from '../../types';

interface ConnectionDoctorProps {
  onClose?: () => void;
}

export function ConnectionDoctor({ onClose: _onClose }: ConnectionDoctorProps) {
  const [isRunning, setIsRunning] = useState<boolean>(false);
  const [testResults, setTestResults] = useState<Record<string, { ok: boolean; detail: string }>>({});
  const [adviceList, setAdviceList] = useState<string[]>([]);

  const tests = [
    { id: 'mdns', name: 'Campus Local Discovery (mDNS)', desc: 'Detects peers on the same local Wi-Fi router' },
    { id: 'quic_udp', name: 'Direct UDP / QUIC', desc: 'Checks if university firewall permits direct peer-to-peer UDP' },
    { id: 'relay_tcp', name: 'Encrypted Relay (Port 443 TCP)', desc: 'Verifies fallback HTTPS-camouflaged tunnel connectivity' },
    { id: 'nat_type', name: 'NAT Architecture Inspection', desc: 'Identifies symmetric NAT vs full cone traversal' },
  ];

  const startDiagnostics = async () => {
    setIsRunning(true);
    setTestResults({});
    setAdviceList([]);

    try {
      await invoke('run_connection_doctor');
    } catch (err) {
      console.error('Failed to trigger connection doctor:', err);
      setIsRunning(false);
    }
  };

  useEffect(() => {
    let mounted = true;
    let unlistenResult: (() => void) | undefined;
    let unlistenComplete: (() => void) | undefined;
    let timeoutId: any;

    listen<DoctorTestResult>('doctor-test-result', (event) => {
      if (!mounted) return;
      setTestResults((prev) => ({
        ...prev,
        [event.payload.test]: {
          ok: event.payload.ok,
          detail: event.payload.detail,
        },
      }));
    }).then((unlisten) => {
      unlistenResult = unlisten;
    });

    listen<{ advice: string[] }>('doctor-complete', (event) => {
      if (!mounted) return;
      setAdviceList(event.payload.advice || []);
      setIsRunning(false);
      clearTimeout(timeoutId);
    }).then((unlisten) => {
      unlistenComplete = unlisten;
    });

    setIsRunning(true);
    setTestResults({});
    setAdviceList([]);
    invoke('run_connection_doctor').catch((err) => {
      console.error('Failed to trigger connection doctor:', err);
      if (mounted) setIsRunning(false);
    });

    // Safety timeout: ensure button and status never stay stuck on "Analyzing..."
    timeoutId = setTimeout(() => {
      if (mounted) {
        setIsRunning(false);
      }
    }, 8000);

    return () => {
      mounted = false;
      clearTimeout(timeoutId);
      if (unlistenResult) unlistenResult();
      if (unlistenComplete) unlistenComplete();
    };
  }, []);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between pb-3 border-b border-gray-200">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-lg bg-teal-50 text-teal-600 flex items-center justify-center border border-teal-200">
            <Stethoscope className="w-4 h-4" />
          </div>
          <div>
            <h2 className="text-sm font-bold text-gray-900">Connection Doctor</h2>
            <p className="text-[11px] text-gray-500">Campus network traversal & diagnostic analysis</p>
          </div>
        </div>
        <button
          onClick={startDiagnostics}
          disabled={isRunning}
          className="flex items-center gap-1.5 px-3 py-1.5 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded-lg text-xs font-semibold transition cursor-pointer shadow-2xs"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${isRunning ? 'animate-spin' : ''}`} />
          <span>{isRunning ? 'Analyzing...' : 'Run Diagnostics'}</span>
        </button>
      </div>

      <div className="space-y-3">
        {tests.map((t) => {
          const res = testResults[t.id];
          const isPending = isRunning && !res;

          return (
            <div
              key={t.id}
              className="bg-white border border-gray-200 rounded-xl p-3.5 flex items-start justify-between gap-4 shadow-2xs"
            >
              <div className="space-y-1">
                <div className="flex items-center gap-2">
                  <span className="text-xs font-semibold text-gray-900">{t.name}</span>
                </div>
                <p className="text-[11px] text-gray-500">{t.desc}</p>
                {res && (
                  <p className={`text-[11px] mt-1 font-mono font-medium ${res.ok ? 'text-teal-700' : 'text-amber-700'}`}>
                    {res.detail}
                  </p>
                )}
              </div>

              <div className="shrink-0 pt-0.5">
                {isPending && (
                  <div className="flex items-center gap-1.5 text-xs text-amber-600 font-medium">
                    <span className="w-2 h-2 rounded-full bg-amber-500 animate-ping" />
                    <span>Testing...</span>
                  </div>
                )}
                {!isPending && res && (
                  res.ok ? (
                    <div className="flex items-center gap-1 text-xs text-teal-700 font-semibold bg-teal-50 px-2 py-0.5 rounded border border-teal-200">
                      <CheckCircle2 className="w-4 h-4 text-teal-600" />
                      <span>Pass</span>
                    </div>
                  ) : (
                    <div className="flex items-center gap-1 text-xs text-rose-700 font-semibold bg-rose-50 px-2 py-0.5 rounded border border-rose-200">
                      <XCircle className="w-4 h-4 text-rose-600" />
                      <span>Blocked</span>
                    </div>
                  )
                )}
                {!isPending && !res && (
                  <span className="text-xs text-gray-400">Queued</span>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {adviceList.length > 0 && (
        <div className="bg-amber-50 border border-amber-200 rounded-xl p-4 space-y-2">
          <div className="flex items-center gap-2 text-xs font-bold text-amber-900">
            <AlertCircle className="w-4 h-4 text-amber-600 shrink-0" />
            <span>Campus Network Diagnostic Advice</span>
          </div>
          <div className="space-y-1.5 text-xs text-amber-900/90 pl-6 list-disc font-medium">
            {adviceList.map((advice, i) => (
              <div key={i} className="leading-relaxed">
                • {advice}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
