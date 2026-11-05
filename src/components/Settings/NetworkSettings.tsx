import { useState, useEffect } from 'react';
import { 
  Plus, 
  Trash2, 
  Shield, 
  Home, 
  Stethoscope, 
  Radio, 
  Copy, 
  Check, 
  UserPlus, 
  Network, 
  RefreshCw, 
  Key, 
  Lock, 
  Eye, 
  EyeOff, 
  Server, 
  Globe, 
  Wifi,
  Database as DatabaseIcon,
  ArrowRightLeft,
  CheckCircle2
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { NetworkProfileInfo, SsidRule, PeerInfo, NetworkDatabaseInfo, TransferSummary } from '../../types';
import { ConnectionDoctor } from '../Onboarding/ConnectionDoctor';

export function NetworkSettings() {
  // Network Profile & Stealth
  const [profileInfo, setProfileInfo] = useState<NetworkProfileInfo | null>(null);
  const [ssidRules, setSsidRules] = useState<SsidRule[]>([]);
  const [overrideMode, setOverrideMode] = useState<string>('auto');
  const [newSsid, setNewSsid] = useState<string>('');
  const [newProfile, setNewProfile] = useState<'campus' | 'trusted'>('campus');
  const [showDoctor, setShowDoctor] = useState<boolean>(false);
  const [isSavingRule, setIsSavingRule] = useState<boolean>(false);

  // Identity & Swarm Peers
  const [nodeId, setNodeId] = useState<string>('');
  const [nodeTicket, setNodeTicket] = useState<string>('');
  const [nodeAddr, setNodeAddr] = useState<string>('');
  const [relayUrl, setRelayUrl] = useState<string>('https://euw1-1.relay.iroh.network/');
  const [peerInput, setPeerInput] = useState<string>('');
  const [peerRelayInput, setPeerRelayInput] = useState<string>('');
  const [networkSecret, setNetworkSecret] = useState<string>('');
  const [savingSecret, setSavingSecret] = useState<boolean>(false);
  const [showSecret, setShowSecret] = useState<boolean>(false);
  const [connectStatus, setConnectStatus] = useState<{ msg: string; isError: boolean } | null>(null);
  const [connecting, setConnecting] = useState<boolean>(false);
  const [savingRelay, setSavingRelay] = useState<boolean>(false);
  const [copiedId, setCopiedId] = useState<boolean>(false);
  const [copiedTicket, setCopiedTicket] = useState<boolean>(false);
  const [peers, setPeers] = useState<PeerInfo[]>([]);

  // Network Databases & Multi-Swarm Workspaces
  const [databases, setDatabases] = useState<NetworkDatabaseInfo[]>([]);
  const [switchingDb, setSwitchingDb] = useState<string | null>(null);
  const [transferSource, setTransferSource] = useState<string | null>(null);
  const [transferTarget, setTransferTarget] = useState<string>('');
  const [transferring, setTransferring] = useState<boolean>(false);
  const [transferSummary, setTransferSummary] = useState<TransferSummary | null>(null);

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

  const fetchDatabases = async () => {
    try {
      const list = await invoke<NetworkDatabaseInfo[]>('list_network_databases');
      if (list) setDatabases(list);
    } catch (err) {
      console.warn('Failed to list network databases:', err);
    }
  };

  // Static settings are loaded ONCE on mount so background polling never overwrites user typing
  const loadSettingsOnce = async () => {
    try {
      const settings = await invoke<{ relay_url: string; network_secret?: string }>('get_app_settings');
      if (settings?.relay_url) setRelayUrl(settings.relay_url);
      if (settings?.network_secret !== undefined) setNetworkSecret(settings.network_secret);
    } catch {
      // Fallback
    }
  };

  // Only dynamic connection data is polled every 3s
  const loadNetworkInfo = async () => {
    try {
      const id = await invoke<string>('get_node_id');
      if (id) setNodeId(id);
    } catch {}

    try {
      const ticket = await invoke<string>('get_node_ticket');
      if (ticket) setNodeTicket(ticket);
    } catch {}

    try {
      const addr = await invoke<string>('get_node_addr');
      if (addr) setNodeAddr(addr);
    } catch {}

    try {
      const peerList = await invoke<PeerInfo[]>('list_peers');
      if (peerList) setPeers(peerList);
    } catch {}
  };

  useEffect(() => {
    fetchProfileAndRules();
    loadSettingsOnce();
    fetchDatabases();
    loadNetworkInfo();
    const interval = setInterval(loadNetworkInfo, 3000);
    return () => clearInterval(interval);
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

    setIsSavingRule(true);
    try {
      await invoke('add_ssid_rule', { ssid: newSsid.trim(), profile: newProfile });
      setNewSsid('');
      await fetchProfileAndRules();
    } catch (err) {
      console.error('Failed to add rule:', err);
    } finally {
      setIsSavingRule(false);
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

  const handleConnectPeer = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!peerInput.trim()) return;

    setConnecting(true);
    setConnectStatus(null);
    try {
      await invoke('connect_peer', {
        nodeId: peerInput.trim(),
        nodeAddrOrTicket: peerInput.trim(),
        relayUrl: peerRelayInput.trim() || null,
      });
      setConnectStatus({ msg: 'Successfully connected and initiated sync!', isError: false });
      setPeerInput('');
      setPeerRelayInput('');
      await loadNetworkInfo();
    } catch (err) {
      setConnectStatus({ msg: String(err), isError: true });
    } finally {
      setConnecting(false);
    }
  };

  const handleSaveSecret = async (e: React.FormEvent) => {
    e.preventDefault();
    setSavingSecret(true);
    try {
      await invoke('set_network_secret', { secret: networkSecret.trim() });
      await fetchDatabases();
      alert('P2P Network Secret saved! Active workspace database switched.');
    } catch (err) {
      alert(`Failed to save network secret: ${err}`);
    } finally {
      setSavingSecret(false);
    }
  };

  const handleSwitchDatabase = async (secret: string) => {
    setSwitchingDb(secret);
    try {
      await invoke('switch_network_database', { secret });
      setNetworkSecret(secret);
      await fetchDatabases();
      await loadNetworkInfo();
      alert(`Switched active network to: ${secret || 'Default Open Network'}`);
    } catch (err) {
      alert(`Failed to switch network: ${err}`);
    } finally {
      setSwitchingDb(null);
    }
  };

  const handleDeleteDatabase = async (secret: string) => {
    const target = databases.find((d) => d.network_secret === secret);
    const label = target ? target.display_name : (secret || 'Default Network');
    if (!confirm(`Are you sure you want to permanently delete the database for "${label}"? This removes its catalog index and file records.`)) return;
    try {
      await invoke('delete_network_database', { secret });
      await fetchDatabases();
    } catch (err) {
      alert(`Failed to delete database: ${err}`);
    }
  };

  const handleClearActiveDatabase = async () => {
    if (!confirm('Are you sure you want to clear all file records from this active network database?\n\nThis will reset the workspace file catalog for this network back to empty.')) return;
    try {
      await invoke('clear_active_database');
      await fetchDatabases();
      alert('Active network database has been cleared.');
    } catch (err) {
      alert(`Failed to clear database: ${err}`);
    }
  };

  const handleExecuteTransfer = async () => {
    if (transferSource === null) return;
    if (transferSource === transferTarget) {
      alert('Source and target network cannot be the same.');
      return;
    }
    setTransferring(true);
    try {
      const summary = await invoke<TransferSummary>('transfer_network_files', {
        sourceSecret: transferSource,
        targetSecret: transferTarget.trim(),
      });
      setTransferSummary(summary);
      await fetchDatabases();
    } catch (err) {
      alert(`Transfer failed: ${err}`);
    } finally {
      setTransferring(false);
    }
  };

  const handleSaveRelay = async (e: React.FormEvent) => {
    e.preventDefault();
    setSavingRelay(true);
    try {
      await invoke('set_relay_url', { relayUrl: relayUrl.trim() });
      alert('Relay URL saved. Active connection will use the new relay.');
    } catch (err) {
      alert(`Failed to save relay URL: ${err}`);
    } finally {
      setSavingRelay(false);
    }
  };

  const setPresetRelay = (url: string) => {
    setRelayUrl(url);
  };

  const copyToClipboard = (text: string, isTicket: boolean) => {
    navigator.clipboard.writeText(text);
    if (isTicket) {
      setCopiedTicket(true);
      setTimeout(() => setCopiedTicket(false), 2000);
    } else {
      setCopiedId(true);
      setTimeout(() => setCopiedId(false), 2000);
    }
  };

  return (
    <div className="max-w-3xl mx-auto space-y-6 pb-12">
      {/* Active network profile status banner */}
      <div className="bg-white border border-gray-200 rounded-xl p-4 flex items-center justify-between shadow-xs">
        <div className="space-y-1.5">
          <div className="flex items-center gap-2">
            <span className="text-xs font-semibold text-gray-400 uppercase tracking-wider">Current Wi-Fi Profile</span>
            {profileInfo?.profile === 'campus' && (
              <span className="px-2 py-0.5 bg-amber-50 text-amber-700 text-xs font-bold rounded-full flex items-center gap-1 border border-amber-200">
                <Shield className="w-3 h-3 text-amber-600" />
                Campus Stealth Active
              </span>
            )}
            {profileInfo?.profile === 'trusted' && (
              <span className="px-2 py-0.5 bg-emerald-50 text-emerald-700 text-xs font-bold rounded-full flex items-center gap-1 border border-emerald-200">
                <Home className="w-3 h-3 text-emerald-600" />
                Trusted Network Active
              </span>
            )}
            {profileInfo?.profile === 'unknown' && (
              <span className="px-2 py-0.5 bg-gray-100 text-gray-700 text-xs font-medium rounded-full">
                Unclassified Network
              </span>
            )}
          </div>
          <div className="text-sm font-medium text-gray-900 flex items-center gap-2">
            <span>SSID: <strong className="font-mono">{profileInfo?.ssid || 'Ethernet / Unknown'}</strong></span>
            {profileInfo?.lecture_throttle_active && (
              <span className="text-xs text-amber-600 font-medium">
                (Lecture Hours: Cap 512 KB/s)
              </span>
            )}
          </div>
        </div>

        <button
          onClick={() => setShowDoctor(!showDoctor)}
          className="flex items-center gap-1.5 px-3 py-1.5 bg-blue-50 text-blue-700 hover:bg-blue-100 rounded-lg text-xs font-semibold transition border border-blue-200 cursor-pointer"
        >
          <Stethoscope className="w-3.5 h-3.5" />
          {showDoctor ? 'Hide Connection Doctor' : 'Connection Doctor'}
        </button>
      </div>

      {/* Connection Doctor Diagnostic Tool */}
      {showDoctor && (
        <div className="animate-in fade-in slide-in-from-top duration-200">
          <ConnectionDoctor />
        </div>
      )}

      {/* Swarm Identity & Direct Linking Card */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
        <div className="flex items-center gap-2.5 border-b border-gray-100 pb-3">
          <Radio className="w-5 h-5 text-indigo-600" />
          <div>
            <h3 className="text-sm font-bold text-gray-900">Your Swarm Identity</h3>
            <p className="text-xs text-gray-500">Share your Node ID or Ticket with classmates to form direct mesh sync paths.</p>
          </div>
        </div>

        <div className="space-y-3">
          <div>
            <label className="text-xs font-semibold text-gray-700">Public Node ID</label>
            <div className="flex gap-2 mt-1">
              <input
                type="text"
                readOnly
                value={nodeId || 'Generating node identity...'}
                className="w-full text-xs font-mono bg-gray-50 border border-gray-200 rounded-lg px-3 py-2 text-gray-600 select-all focus:outline-none"
              />
              <button
                onClick={() => copyToClipboard(nodeId, false)}
                disabled={!nodeId}
                className="px-3 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg text-xs font-semibold transition flex items-center gap-1.5 shrink-0"
              >
                {copiedId ? <Check className="w-3.5 h-3.5 text-emerald-600" /> : <Copy className="w-3.5 h-3.5" />}
                {copiedId ? 'Copied' : 'Copy'}
              </button>
            </div>
          </div>

          <div>
            <label className="text-xs font-semibold text-gray-700">Complete Node Ticket (Recommended)</label>
            <div className="flex gap-2 mt-1">
              <input
                type="text"
                readOnly
                value={nodeTicket || 'Generating ticket...'}
                className="w-full text-xs font-mono bg-gray-50 border border-gray-200 rounded-lg px-3 py-2 text-gray-600 select-all focus:outline-none"
              />
              <button
                onClick={() => copyToClipboard(nodeTicket, true)}
                disabled={!nodeTicket}
                className="px-3 py-2 bg-indigo-50 hover:bg-indigo-100 text-indigo-700 border border-indigo-200 rounded-lg text-xs font-semibold transition flex items-center gap-1.5 shrink-0"
              >
                {copiedTicket ? <Check className="w-3.5 h-3.5 text-emerald-600" /> : <Copy className="w-3.5 h-3.5" />}
                {copiedTicket ? 'Copied Ticket' : 'Copy Ticket'}
              </button>
            </div>
            <p className="text-[11px] text-gray-400 mt-1">
              This ticket encodes your Node ID, direct IPs, and relay addresses. Friends can paste this directly to link.
            </p>
          </div>
          {nodeAddr && (
            <p className="text-[10px] font-mono text-gray-400 truncate">
              Direct Socket Address: {nodeAddr}
            </p>
          )}
        </div>
      </div>

      {/* Manual Peer Link Form */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
        <div className="flex items-center gap-2.5 border-b border-gray-100 pb-3">
          <UserPlus className="w-5 h-5 text-blue-600" />
          <div>
            <h3 className="text-sm font-bold text-gray-900">Connect to Remote Peer</h3>
            <p className="text-xs text-gray-500">Paste a peer's Node Ticket or raw Node ID to manually establish a sync link.</p>
          </div>
        </div>

        <form onSubmit={handleConnectPeer} className="space-y-3">
          <div>
            <label className="text-xs font-semibold text-gray-700">Peer Ticket or Node ID</label>
            <input
              type="text"
              placeholder="node... or 64-char hex Node ID"
              value={peerInput}
              onChange={(e) => setPeerInput(e.target.value)}
              className="w-full mt-1 text-xs font-mono bg-gray-50 border border-gray-200 rounded-lg px-3 py-2 focus:bg-white focus:border-blue-500 focus:outline-none transition-all"
            />
          </div>

          <div>
            <label className="text-xs font-semibold text-gray-700">Relay URL (Optional for raw Node ID)</label>
            <input
              type="text"
              placeholder="Leave empty to use default relay"
              value={peerRelayInput}
              onChange={(e) => setPeerRelayInput(e.target.value)}
              className="w-full mt-1 text-xs font-mono bg-gray-50 border border-gray-200 rounded-lg px-3 py-2 focus:bg-white focus:border-blue-500 focus:outline-none transition-all"
            />
          </div>

          {connectStatus && (
            <div className={`p-3 rounded-lg text-xs font-medium ${connectStatus.isError ? 'bg-red-50 text-red-700 border border-red-200' : 'bg-emerald-50 text-emerald-700 border border-emerald-200'}`}>
              {connectStatus.msg}
            </div>
          )}

          <button
            type="submit"
            disabled={connecting || !peerInput.trim()}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-semibold shadow-xs transition-colors disabled:opacity-50 flex items-center gap-1.5"
          >
            <Plus className="w-3.5 h-3.5" />
            {connecting ? 'Connecting...' : 'Add Peer to Swarm'}
          </button>
        </form>
      </div>

      {/* Connected Peers Swarm Table */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
        <div className="flex items-center justify-between border-b border-gray-100 pb-3">
          <div className="flex items-center gap-2.5">
            <Network className="w-5 h-5 text-emerald-600" />
            <div>
              <h3 className="text-sm font-bold text-gray-900">Active Swarm Peers</h3>
              <p className="text-xs text-gray-500">Live peers contributing and syncing documents with your local node.</p>
            </div>
          </div>
          <button
            onClick={loadNetworkInfo}
            className="p-1.5 hover:bg-gray-100 rounded-lg text-gray-500 hover:text-gray-700 transition"
            title="Refresh Peers"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
        </div>

        {peers.length === 0 ? (
          <div className="text-center py-6 border border-dashed border-gray-200 rounded-lg">
            <Radio className="w-8 h-8 text-gray-300 mx-auto mb-2" />
            <p className="text-xs font-medium text-gray-500">No peers currently connected</p>
            <p className="text-[11px] text-gray-400 mt-0.5">Share your Node Ticket above or join the same Wi-Fi to auto-discover peers.</p>
          </div>
        ) : (
          <div className="divide-y divide-gray-100">
            {peers.map((p, idx) => (
              <div key={idx} className="py-2.5 flex items-center justify-between text-xs">
                <div>
                  <span className="font-mono font-medium text-gray-900">{p.node_id.slice(0, 16)}...{p.node_id.slice(-8)}</span>
                  {p.relay_url && <p className="text-[10px] text-gray-400 truncate max-w-sm">{p.relay_url}</p>}
                </div>
                <div className="flex items-center gap-2">
                  <span className="inline-flex items-center gap-1 px-2 py-0.5 bg-emerald-50 text-emerald-700 text-[10px] font-bold rounded-full">
                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
                    Connected
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Network Profile Classification & SSID Rules */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
        <div className="flex items-center gap-2.5 border-b border-gray-100 pb-3">
          <Shield className="w-5 h-5 text-indigo-600" />
          <div>
            <h3 className="text-sm font-bold text-gray-900">Network Profiles & Campus Stealth</h3>
            <p className="text-xs text-gray-500">
              Configure profile detection on university Wi-Fi networks (disables mDNS/DHT, enforces port 443 relay).
            </p>
          </div>
        </div>

        {/* Override Buttons */}
        <div>
          <label className="text-xs font-semibold text-gray-700 block mb-1.5">Profile Override Mode</label>
          <div className="grid grid-cols-3 gap-2">
            <button
              onClick={() => handleSetOverride('auto')}
              className={`p-2.5 rounded-lg border text-left transition cursor-pointer ${
                overrideMode === 'auto'
                  ? 'border-blue-600 bg-blue-50/50 ring-1 ring-blue-600'
                  : 'border-gray-200 hover:bg-gray-50'
              }`}
            >
              <div className="flex items-center gap-1.5 text-xs font-bold text-gray-900">
                <Globe className="w-3.5 h-3.5 text-blue-600" />
                Auto-Detect
              </div>
              <p className="text-[11px] text-gray-500 mt-0.5">Detect via SSID rules</p>
            </button>

            <button
              onClick={() => handleSetOverride('campus')}
              className={`p-2.5 rounded-lg border text-left transition cursor-pointer ${
                overrideMode === 'campus'
                  ? 'border-amber-600 bg-amber-50/50 ring-1 ring-amber-600'
                  : 'border-gray-200 hover:bg-gray-50'
              }`}
            >
              <div className="flex items-center gap-1.5 text-xs font-bold text-amber-900">
                <Shield className="w-3.5 h-3.5 text-amber-600" />
                Force Campus
              </div>
              <p className="text-[11px] text-gray-500 mt-0.5">Always enable stealth</p>
            </button>

            <button
              onClick={() => handleSetOverride('trusted')}
              className={`p-2.5 rounded-lg border text-left transition cursor-pointer ${
                overrideMode === 'trusted'
                  ? 'border-emerald-600 bg-emerald-50/50 ring-1 ring-emerald-600'
                  : 'border-gray-200 hover:bg-gray-50'
              }`}
            >
              <div className="flex items-center gap-1.5 text-xs font-bold text-emerald-900">
                <Home className="w-3.5 h-3.5 text-emerald-600" />
                Force Trusted
              </div>
              <p className="text-[11px] text-gray-500 mt-0.5">Unrestricted transfer</p>
            </button>
          </div>
        </div>

        {/* SSID Rules Table */}
        <div className="space-y-3 pt-2">
          <label className="text-xs font-semibold text-gray-700 block">Configured SSID Rules</label>
          <div className="border border-gray-200 rounded-lg overflow-hidden divide-y divide-gray-100">
            {ssidRules.map((rule) => (
              <div key={rule.ssid} className="p-3 flex items-center justify-between bg-white text-xs">
                <div className="flex items-center gap-2">
                  <Wifi className="w-3.5 h-3.5 text-gray-400" />
                  <span className="font-mono font-medium text-gray-900">{rule.ssid}</span>
                </div>
                <div className="flex items-center gap-3">
                  <span
                    className={`px-2 py-0.5 rounded-full text-[10px] font-bold ${
                      rule.profile === 'campus'
                        ? 'bg-amber-50 text-amber-700 border border-amber-200'
                        : 'bg-emerald-50 text-emerald-700 border border-emerald-200'
                    }`}
                  >
                    {rule.profile === 'campus' ? 'Campus Stealth' : 'Trusted Network'}
                  </span>
                  <button
                    onClick={() => handleDeleteRule(rule.ssid)}
                    className="text-gray-400 hover:text-red-600 transition cursor-pointer p-0.5"
                    title="Delete rule"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            ))}
          </div>

          {/* Add Rule Form */}
          <form onSubmit={handleAddRule} className="flex gap-2 pt-1">
            <input
              type="text"
              placeholder="Wi-Fi SSID name (e.g. eduroam, LibraryGuest)"
              value={newSsid}
              onChange={(e) => setNewSsid(e.target.value)}
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
              disabled={isSavingRule || !newSsid.trim()}
              className="px-3.5 py-1.5 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded-lg text-xs font-semibold flex items-center gap-1 shadow-2xs transition cursor-pointer"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>Add Rule</span>
            </button>
          </form>
        </div>
      </div>

      {/* Network Databases / Swarm Workspaces Manager (Per Issue Requirements) */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
        <div className="flex items-center justify-between border-b border-gray-100 pb-3">
          <div className="flex items-center gap-2.5">
            <DatabaseIcon className="w-5 h-5 text-purple-600" />
            <div>
              <h3 className="text-sm font-bold text-gray-900">Network Databases & Workspaces</h3>
              <p className="text-xs text-gray-500">
                Each P2P secret has its own isolated database. You can clear them or transfer files you seed between workspaces.
              </p>
            </div>
          </div>
          <button
            onClick={fetchDatabases}
            className="p-1.5 hover:bg-gray-100 rounded-lg text-gray-500 hover:text-gray-700 transition"
            title="Refresh database list"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
        </div>

        <div className="space-y-3">
          {databases.map((db) => (
            <div
              key={db.network_secret || '__default__'}
              className={`p-4 rounded-xl border transition-all ${
                db.is_active
                  ? 'border-indigo-300 bg-indigo-50/40 shadow-xs'
                  : 'border-gray-200 bg-white hover:border-gray-300'
              }`}
            >
              <div className="flex items-center justify-between">
                <div>
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-xs text-gray-900">
                      {db.display_name}
                    </span>
                    {db.is_active && (
                      <span className="px-2 py-0.5 bg-indigo-100 text-indigo-700 text-[10px] font-bold rounded-full">
                        ACTIVE NETWORK
                      </span>
                    )}
                  </div>
                  <p className="text-[11px] font-mono text-gray-400 mt-0.5">
                    File: {db.db_file} • {db.file_count} total file(s) • <strong className="text-emerald-700">{db.seedable_file_count} seedable</strong>
                  </p>
                </div>

                <div className="flex items-center gap-2">
                  {!db.is_active && (
                    <button
                      onClick={() => handleSwitchDatabase(db.network_secret)}
                      disabled={switchingDb === db.network_secret}
                      className="px-2.5 py-1 text-xs bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg font-medium transition cursor-pointer"
                    >
                      {switchingDb === db.network_secret ? 'Switching...' : 'Switch Network'}
                    </button>
                  )}
                  <button
                    onClick={() => {
                      setTransferSource(db.network_secret);
                      setTransferTarget(db.is_active ? '' : db.network_secret);
                      setTransferSummary(null);
                    }}
                    className="px-2.5 py-1 text-xs bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg font-medium transition cursor-pointer flex items-center gap-1"
                    title="Transfer files from this network to another"
                  >
                    <ArrowRightLeft className="w-3 h-3" />
                    Transfer Files...
                  </button>
                  {db.is_active ? (
                    <button
                      onClick={handleClearActiveDatabase}
                      className="px-2.5 py-1 text-xs bg-rose-50 hover:bg-rose-100 text-rose-700 border border-rose-200 rounded-lg font-medium transition cursor-pointer flex items-center gap-1"
                      title="Clear and reset all files in this active network"
                    >
                      <Trash2 className="w-3.5 h-3.5 text-rose-600" />
                      Clear Database
                    </button>
                  ) : (
                    <button
                      onClick={() => handleDeleteDatabase(db.network_secret)}
                      className="px-2.5 py-1 text-xs bg-rose-50 hover:bg-rose-100 text-rose-700 border border-rose-200 rounded-lg font-medium transition cursor-pointer flex items-center gap-1"
                      title="Permanently delete this network database"
                    >
                      <Trash2 className="w-3.5 h-3.5 text-rose-600" />
                      Delete Database
                    </button>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* Transfer Modal / Card */}
        {transferSource !== null && (
          <div className="p-4 bg-gray-50 border border-gray-200 rounded-xl space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs font-bold text-gray-900 flex items-center gap-1.5">
                <ArrowRightLeft className="w-4 h-4 text-indigo-600" />
                Transfer Seedable Files from: <code className="bg-white px-1.5 py-0.5 rounded border border-gray-200">{transferSource || 'Default Open Network'}</code>
              </span>
              <button
                onClick={() => setTransferSource(null)}
                className="text-gray-400 hover:text-gray-600 text-xs cursor-pointer"
              >
                ✕ Cancel
              </button>
            </div>

            <p className="text-[11px] text-gray-600 leading-relaxed">
              ⚠️ <strong>Strict Seed Guarantee:</strong> Only files you have cached or pinned will be transferred, and your node will become their authoritative owner and seeder in the target network. Cloud-only files are skipped to avoid creating dead files on the swarm.
            </p>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-gray-700">Destination Network Passphrase</label>
              <input
                type="text"
                placeholder="Enter target network secret (or leave blank for Default Open Network)"
                value={transferTarget}
                onChange={(e) => setTransferTarget(e.target.value)}
                className="w-full text-xs font-mono bg-white border border-gray-200 rounded-lg px-3 py-2 focus:border-indigo-500 focus:outline-none"
              />
            </div>

            {transferSummary && (
              <div className="p-3 bg-emerald-50 border border-emerald-200 rounded-lg text-xs space-y-1">
                <div className="flex items-center gap-1 text-emerald-800 font-bold">
                  <CheckCircle2 className="w-4 h-4" />
                  Transfer Completed!
                </div>
                <p className="text-emerald-700">
                  Transferred <strong>{transferSummary.transferred_count}</strong> seedable file(s) ({Math.round(transferSummary.total_bytes_transferred / (1024 * 1024))} MB).
                  {transferSummary.skipped_cloud_only_count > 0 && (
                    <span> Skipped {transferSummary.skipped_cloud_only_count} cloud-only un-cached file(s).</span>
                  )}
                </p>
              </div>
            )}

            <button
              onClick={handleExecuteTransfer}
              disabled={transferring}
              className="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg text-xs font-semibold shadow-xs transition flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
            >
              <ArrowRightLeft className="w-3.5 h-3.5" />
              {transferring ? 'Transferring Files...' : 'Execute Seedable File Transfer'}
            </button>
          </div>
        )}
      </div>

      {/* P2P Network Secret Configuration Form */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
        <div className="flex items-center gap-2.5 border-b border-gray-100 pb-3">
          <Key className="w-5 h-5 text-indigo-600" />
          <div>
            <h3 className="text-sm font-bold text-gray-900">P2P Network Passphrase / Swarm Secret</h3>
            <p className="text-xs text-gray-500">
              Isolates your peer swarm from public discovery. Only peers with the exact same secret passphrase can connect and sync files with you.
            </p>
          </div>
        </div>

        <form onSubmit={handleSaveSecret} className="space-y-3">
          <div>
            <label className="text-xs font-semibold text-gray-700">Network Secret Passphrase</label>
            <div className="relative mt-1">
              <input
                type={showSecret ? "text" : "password"}
                placeholder="Enter secret passphrase (e.g. course-group-2026)"
                value={networkSecret}
                onChange={(e) => setNetworkSecret(e.target.value)}
                className="w-full text-xs font-mono bg-gray-50 border border-gray-200 rounded-lg pl-3 pr-10 py-2 focus:bg-white focus:border-indigo-500 focus:outline-none transition-all"
              />
              <button
                type="button"
                onClick={() => setShowSecret(!showSecret)}
                className="absolute right-2.5 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 cursor-pointer"
                title={showSecret ? "Hide passphrase" : "Show passphrase"}
              >
                {showSecret ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
              </button>
            </div>
            <p className="text-[11px] text-gray-400 mt-1">
              Leave blank to use the default open network. Changing this secret isolates your workspace from other student groups.
            </p>
          </div>

          <button
            type="submit"
            disabled={savingSecret}
            className="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg text-xs font-semibold shadow-xs transition-colors flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
          >
            <Lock className="w-3.5 h-3.5" />
            {savingSecret ? 'Saving Secret...' : 'Save Network Secret'}
          </button>
        </form>
      </div>

      {/* Swarm Relay Configuration (Western Europe EU Relay) */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
        <div className="flex items-center gap-2.5 border-b border-gray-100 pb-3">
          <Server className="w-5 h-5 text-blue-600" />
          <div>
            <h3 className="text-sm font-bold text-gray-900">Encrypted Swarm Relay</h3>
            <p className="text-xs text-gray-500">
              High-speed encrypted fallback tunnel (TLS 443) for university firewalls. Uses the official Western Europe relay.
            </p>
          </div>
        </div>

        {/* Informative EU Relay badge and ToS note */}
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <span className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-blue-50 text-blue-700 text-xs font-semibold rounded-lg border border-blue-200">
              🇪🇺 Western Europe (EU) Active: <code className="font-mono">https://euw1-1.relay.iroh.network/</code>
            </span>
          </div>
          <p className="text-[11px] text-gray-500 leading-relaxed">
            🛡️ <strong>Relay Privacy & Terms of Service:</strong> Traffic is end-to-end encrypted (QUIC TLS 1.3). The relay operated by n0 serves strictly as a blind packet forwarder; relay operators cannot inspect or read your document contents. Used in compliance with official Iroh network acceptable use policies.
          </p>
        </div>

        <form onSubmit={handleSaveRelay} className="space-y-3">
          <div>
            <label className="text-xs font-semibold text-gray-700">Relay Server URL</label>
            <input
              type="text"
              value={relayUrl}
              onChange={(e) => setRelayUrl(e.target.value)}
              placeholder="https://euw1-1.relay.iroh.network/"
              className="w-full mt-1 text-xs font-mono bg-gray-50 border border-gray-200 rounded-lg px-3 py-2 focus:bg-white focus:border-blue-500 focus:outline-none transition-all"
            />
          </div>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => setPresetRelay('https://euw1-1.relay.iroh.network/')}
              className="px-3 py-1.5 bg-gray-100 hover:bg-gray-200 text-gray-700 text-xs font-medium rounded-lg transition cursor-pointer"
            >
              Reset to Official EU Relay
            </button>
            <button
              type="submit"
              disabled={savingRelay || !relayUrl.trim()}
              className="px-4 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-semibold shadow-xs transition-colors disabled:opacity-50 flex items-center gap-1.5 cursor-pointer"
            >
              <Server className="w-3.5 h-3.5" />
              {savingRelay ? 'Saving...' : 'Save Relay Configuration'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
