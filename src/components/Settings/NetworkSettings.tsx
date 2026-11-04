import { useState, useEffect } from 'react';
import { Plus, Trash2, Shield, Home, Stethoscope } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { NetworkProfileInfo, SsidRule } from '../../types';
import { ConnectionDoctor } from '../Onboarding/ConnectionDoctor';

export function NetworkSettings() {
  const [profileInfo, setProfileInfo] = useState<NetworkProfileInfo | null>(null);
  const [ssidRules, setSsidRules] = useState<SsidRule[]>([]);
  const [overrideMode, setOverrideMode] = useState<string>('auto');
  const [newSsid, setNewSsid] = useState<string>('');
  const [newProfile, setNewProfile] = useState<'campus' | 'trusted'>('campus');
  const [showDoctor, setShowDoctor] = useState<boolean>(false);
  const [isSaving, setIsSaving] = useState<boolean>(false);

  const fetchProfileAndRules = async () => {
    try {
      const info = await invoke<NetworkProfileInfo>('get_network_profile');
      setProfileInfo(info);
      setOverrideMode(info.override_active || 'auto');

      const rules = await invoke<SsidRule[]>('list_ssid_rules');
      setSsidRules(rules || []);
    } catch (err) {
      console.warn('Failed to load profile and rules:', err);
    }
  };

  useEffect(() => {
    fetchProfileAndRules();
  }, []);

  const handleSetOverride = async (mode: string) => {
    try {
      await invoke('set_network_profile_override', { profile: mode });
      setOverrideMode(mode);
      await fetchProfileAndRules();
    } catch (err) {
      console.error('Failed to set override:', err);
    }
  };

  const handleAddRule = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newSsid.trim()) return;

    setIsSaving(true);
    try {
      await invoke('add_ssid_rule', { ssid: newSsid.trim(), profile: newProfile });
      setNewSsid('');
      await fetchProfileAndRules();
    } catch (err) {
      console.error('Failed to add rule:', err);
    } finally {
      setIsSaving(false);
    }
  };

  const handleDeleteRule = async (ssid: string) => {
    try {
      await invoke('delete_ssid_rule', { ssid });
      await fetchProfileAndRules();
    } catch (err) {
      console.error('Failed to delete rule:', err);
    }
  };

  return (
    <div className="space-y-6">
      {/* Active network profile status */}
      <div className="bg-white border border-gray-200 rounded-xl p-4 flex items-center justify-between shadow-xs">
        <div className="space-y-1.5">
          <div className="flex items-center gap-2">
            {profileInfo?.profile === 'campus' ? (
              <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-amber-50 text-amber-800 border border-amber-300">
                <Shield className="w-3.5 h-3.5 text-amber-600" />
                <span>Campus Stealth Mode Active</span>
              </span>
            ) : (
              <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-emerald-50 text-emerald-800 border border-emerald-300">
                <Home className="w-3.5 h-3.5 text-emerald-600" />
                <span>Trusted / Home Network</span>
              </span>
            )}
          </div>
          <p className="text-xs text-gray-600">
            Detected Wi-Fi SSID: <strong className="text-gray-900 font-mono font-semibold">{profileInfo?.ssid || 'Ethernet / Unknown'}</strong>
            {profileInfo?.lecture_throttle_active && ' • (Lecture throttle 128 KB/s active)'}
          </p>
        </div>

        <button
          onClick={() => setShowDoctor(!showDoctor)}
          className="flex items-center gap-1.5 px-3 py-1.5 bg-blue-50 hover:bg-blue-100 text-blue-700 border border-blue-200 rounded-lg text-xs font-semibold transition cursor-pointer"
        >
          <Stethoscope className="w-4 h-4 text-blue-600" />
          <span>{showDoctor ? 'Hide Diagnostics' : 'Connection Doctor'}</span>
        </button>
      </div>

      {showDoctor && (
        <div className="bg-white border border-gray-200 rounded-xl p-5 shadow-xs">
          <ConnectionDoctor onClose={() => setShowDoctor(false)} />
        </div>
      )}

      {/* Manual override buttons */}
      <div className="space-y-2">
        <label className="block text-xs font-semibold text-gray-700">
          Profile Enforcement Mode
        </label>
        <div className="grid grid-cols-3 gap-2">
          <button
            onClick={() => handleSetOverride('auto')}
            className={`px-3 py-2 text-xs rounded-lg border font-semibold transition text-center cursor-pointer ${
              overrideMode === 'auto'
                ? 'bg-blue-50 border-blue-600 text-blue-700 shadow-2xs ring-1 ring-blue-500/20'
                : 'bg-white border-gray-200 text-gray-700 hover:bg-gray-50'
            }`}
          >
            Auto-Detect SSID
          </button>
          <button
            onClick={() => handleSetOverride('campus')}
            className={`px-3 py-2 text-xs rounded-lg border font-semibold transition text-center cursor-pointer ${
              overrideMode === 'campus'
                ? 'bg-amber-50 border-amber-600 text-amber-800 shadow-2xs ring-1 ring-amber-500/20'
                : 'bg-white border-gray-200 text-gray-700 hover:bg-gray-50'
            }`}
          >
            Force Campus Mode
          </button>
          <button
            onClick={() => handleSetOverride('trusted')}
            className={`px-3 py-2 text-xs rounded-lg border font-semibold transition text-center cursor-pointer ${
              overrideMode === 'trusted'
                ? 'bg-emerald-50 border-emerald-600 text-emerald-800 shadow-2xs ring-1 ring-emerald-500/20'
                : 'bg-white border-gray-200 text-gray-700 hover:bg-gray-50'
            }`}
          >
            Force Trusted Mode
          </button>
        </div>
      </div>

      {/* SSID Rules Table */}
      <div className="space-y-3">
        <h3 className="text-xs font-bold text-gray-600 uppercase tracking-wider">
          Known Network SSID Rules
        </h3>

        <div className="border border-gray-200 rounded-xl overflow-hidden bg-white shadow-xs">
          <table className="w-full text-left text-xs">
            <thead className="bg-gray-50 text-gray-700 border-b border-gray-200">
              <tr>
                <th className="px-4 py-2.5 font-semibold">SSID Name</th>
                <th className="px-4 py-2.5 font-semibold">Assigned Profile</th>
                <th className="px-4 py-2.5 font-semibold text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {ssidRules.map((rule) => (
                <tr key={rule.ssid} className="hover:bg-gray-50/70 transition">
                  <td className="px-4 py-2.5 font-mono font-medium text-gray-900">{rule.ssid}</td>
                  <td className="px-4 py-2.5">
                    <span
                      className={`inline-block px-2 py-0.5 rounded text-[11px] font-semibold border ${
                        rule.profile === 'campus'
                          ? 'bg-amber-50 text-amber-800 border-amber-200'
                          : 'bg-emerald-50 text-emerald-800 border-emerald-200'
                      }`}
                    >
                      {rule.profile === 'campus' ? 'Campus Stealth' : 'Trusted / Home'}
                    </span>
                  </td>
                  <td className="px-4 py-2.5 text-right">
                    <button
                      onClick={() => handleDeleteRule(rule.ssid)}
                      className="p-1 text-gray-400 hover:text-red-600 rounded transition cursor-pointer"
                      title="Delete this SSID rule"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </td>
                </tr>
              ))}
              {ssidRules.length === 0 && (
                <tr>
                  <td colSpan={3} className="px-4 py-4 text-center text-xs text-gray-400">
                    No custom SSID rules added. Unrecognized networks default to Campus Stealth Mode.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        {/* Add SSID Rule Form */}
        <form onSubmit={handleAddRule} className="flex gap-2 items-center">
          <input
            type="text"
            value={newSsid}
            onChange={(e) => setNewSsid(e.target.value)}
            placeholder="Add Wi-Fi SSID (e.g. eduroam-5g)"
            className="flex-1 bg-white border border-gray-300 rounded-lg px-3 py-1.5 text-xs text-gray-900 placeholder-gray-400 focus:outline-none focus:border-blue-500 shadow-2xs"
          />
          <select
            value={newProfile}
            onChange={(e) => setNewProfile(e.target.value as 'campus' | 'trusted')}
            className="bg-white border border-gray-300 rounded-lg px-3 py-1.5 text-xs text-gray-800 focus:outline-none focus:border-blue-500 shadow-2xs"
          >
            <option value="campus">Campus Stealth</option>
            <option value="trusted">Trusted Network</option>
          </select>
          <button
            type="submit"
            disabled={isSaving || !newSsid.trim()}
            className="px-3.5 py-1.5 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded-lg text-xs font-semibold flex items-center gap-1 shadow-2xs transition cursor-pointer"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>Add Rule</span>
          </button>
        </form>
      </div>
    </div>
  );
}
