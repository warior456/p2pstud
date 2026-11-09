import { useState, useEffect } from 'react';
import { CheckCircle2, XCircle, RefreshCw, AlertCircle, Stethoscope, Play, ShieldQuestion } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { DoctorTestResult, DoctorReport, NetworkProfileInfo } from '../../types';

interface ConnectionDoctorProps {
  onClose?: () => void;
}

interface TestRow {
  id: string;
  name: string;
  desc: string;
  critical?: boolean;
}

const TESTS: TestRow[] = [
  {
    id: 'engine_local',
    name: 'Local P2P Engine & LAN Discovery',
    desc: 'Checks that the node is running and can discover peers on this Wi-Fi',
  },
  {
    id: 'relay_tcp',
    name: 'Encrypted Relay (TCP 443)',
    desc: 'Critical fallback: HTTPS-camouflaged tunnel used whenever direct peer connections are blocked',
    critical: true,
  },
  {
    id: 'direct_udp',
    name: 'Direct QUIC / UDP',
    desc: 'Optional: allows faster direct transfers on trusted networks. Campus mode intentionally uses the relay only.',
  },
  {
    id: 'swarm_peers',
    name: 'Swarm Peers Seen',
    desc: 'How many peers were recently discovered. Zero is normal on a new or small swarm.',
  },
];

export function ConnectionDoctor({ onClose: _onClose }: ConnectionDoctorProps) {
  const [hasRun, setHasRun] = useState<boolean>(false);
  const [isRunning, setIsRunning] = useState<boolean>(false);
  const [testResults, setTestResults] = useState<Record<string, { ok: boolean; detail: string }>>({});
  const [adviceList, setAdviceList] = useState<string[]>([]);
  const [profile, setProfile] = useState<NetworkProfileInfo | null>(null);

  useEffect(() => {
    invoke<NetworkProfileInfo>('get_network_profile')
      .then(setProfile)
      .catch(() => {});
  }, []);

  const startDiagnostics = async () => {
    setHasRun(true);
    setIsRunning(true);
    setTestResults({});
    setAdviceList([]);

    try {
      const report = await invoke<DoctorReport>('run_connection_doctor');
      const map: Record<string, { ok: boolean; detail: string }> = {};
      const results = report?.results || [];
      for (const res of results) {
        map[res.test] = { ok: res.ok, detail: res.detail };
      }
      for (const t of TESTS) {
        if (!map[t.id]) {
          map[t.id] = { ok: false, detail: 'This check did not return a result. Try running the doctor again.' };
        }
      }
      setTestResults(map);
      setAdviceList(report?.advice || []);
    } catch (err) {
      console.error('Failed to trigger connection doctor:', err);
      const map: Record<string, { ok: boolean; detail: string }> = {};
      for (const t of TESTS) {
        map[t.id] = { ok: false, detail: `Diagnostic could not run: ${err}` };
      }
      setTestResults(map);
    } finally {
      setIsRunning(false);
    }
  };

  useEffect(() => {
    let mounted = true;
    let unlistenResult: (() => void) | undefined;
    let unlistenComplete: (() => void) | undefined;

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
    }).then((unlisten) => {
      unlistenComplete = unlisten;
    });

    // Intentionally NOT auto-running: campus firewalls can flag a burst of probes,
    // and users should know what the doctor does before it touches the network.
    return () => {
      mounted = false;
      if (unlistenResult) unlistenResult();
      if (unlistenComplete) unlistenComplete();
    };
  }, []);

  const networkUnverified = profile?.is_known_network === false || profile?.profile === 'unknown';

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
          <span>{isRunning ? 'Analyzing...' : hasRun ? 'Run Again' : 'Run Diagnostics'}</span>
        </button>
      </div>

      {networkUnverified && (
        <div className="bg-amber-50 border border-amber-200 rounded-xl p-3.5 flex items-start gap-2.5">
          <ShieldQuestion className="w-4 h-4 text-amber-600 shrink-0 mt-0.5" />
          <div className="text-xs text-amber-900 leading-relaxed">
            <strong>Unverified network{profile?.ssid ? ` "${profile.ssid}"` : ''}.</strong> P2PStud has not
            classified this Wi-Fi yet. Running the doctor sends a few short TCP/UDP probes to the relay and
            your local network — nothing is uploaded or shared. Classify the network from the network banner
            or Settings → Network so campus stealth rules apply.
          </div>
        </div>
      )}

      {!hasRun && !isRunning && (
        <div className="bg-white border border-gray-200 rounded-xl p-5 text-center space-y-3">
          <div className="mx-auto w-10 h-10 rounded-full bg-teal-50 text-teal-600 flex items-center justify-center border border-teal-200">
            <Stethoscope className="w-5 h-5" />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-gray-900">Check your connection before troubleshooting</h3>
            <p className="text-xs text-gray-500 mt-1 max-w-md mx-auto leading-relaxed">
              The doctor verifies relay reachability and reports which connection paths (LAN, direct, relay)
              are available on this network. It only runs when you click the button.
            </p>
          </div>
          <button
            onClick={startDiagnostics}
            className="inline-flex items-center gap-1.5 px-4 py-2 bg-teal-600 hover:bg-teal-700 text-white rounded-lg text-xs font-semibold transition cursor-pointer shadow-2xs"
          >
            <Play className="w-3.5 h-3.5" />
            <span>Run Connection Doctor</span>
          </button>
        </div>
      )}

      {(hasRun || isRunning) && (
        <div className="space-y-3">
          {TESTS.map((t) => {
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
                    {t.critical && (
                      <span className="text-[9px] font-bold uppercase tracking-wide bg-rose-50 text-rose-700 border border-rose-200 px-1.5 py-0.5 rounded">
                        Required
                      </span>
                    )}
                  </div>
                  <p className="text-[11px] text-gray-500">{t.desc}</p>
                  {res && (
                    <p className={`text-[11px] mt-1 font-mono font-medium ${res.ok ? 'text-teal-700' : 'text-rose-700'}`}>
                      {res.detail}
                    </p>
                  )}
                  {!isPending && !res && (
                    <p className="text-[11px] mt-1 font-mono font-medium text-gray-500">
                      Not run yet
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
                        <span>{t.critical ? 'Blocked' : 'Unavailable'}</span>
                      </div>
                    )
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}

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
