import { useState, useEffect } from 'react';
import { 
  Plus, 
  Trash2, 
  Shield, 
  Home, 
  Stethoscope,
  Radio,
  Activity,
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
  Wifi
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { NetworkProfileInfo, SsidRule, PeerInfo } from '../../types';
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
  const [relayUrl, setRelayUrl] = useState<string>('https://use1-1.relay.iroh.network/');
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

  const loadNetworkInfo = async () => {
    try {
      const id = await invoke<string>('get_node_id');
      if (id) setNodeId(id);
    } catch {
      // Starting
    }

    try {
      const ticket = await invoke<string>('get_node_ticket');
      if (ticket) setNodeTicket(ticket);
    } catch {
      // Starting
    }

    try {
      const addr = await invoke<string>('get_node_addr');
      if (addr) setNodeAddr(addr);
    } catch {
      // Optional
    }

    try {
      const settings = await invoke<{ relay_url: string; network_secret?: string }>('get_app_settings');
      if (settings?.relay_url) setRelayUrl(settings.relay_url);
      if (settings?.network_secret !== undefined) setNetworkSecret(settings.network_secret);
    } catch {
      // Fallback
    }

    try {
      const peerList = await invoke<PeerInfo[]>('list_peers');
      if (peerList) setPeers(peerList);
    } catch {
      // Fallback
    }
  };

  useEffect(() => {
    fetchProfileAndRules();
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
        relayUrl: peerRelayInput.trim() || null,
      });
      setConnectStatus({ msg: 'Successfully connected and synced workspace catalog with peer!', isError: false });
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
      alert('P2P Network Secret saved! Please restart the application to rejoin with the new secret.');
    } catch (err) {
      alert(`Failed to save network secret: ${err}`);
    } finally {
      setSavingSecret(false);
    }
  };

  const handleSaveRelay = async (e: React.FormEvent) => {
    e.preventDefault();
    setSavingRelay(true);
    try {
      await invoke('set_relay_url', { relayUrl: relayUrl.trim() });
      alert('Relay URL saved. Restart app to connect through the new relay.');
    } catch (err) {
      alert(`Failed to save relay URL: ${err}`);
    } finally {
      setSavingRelay(false);
    }
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
    <div className="max-w-3xl mx-auto space-y-6">
      {/* Active network profile status banner */}
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

      {/* Swarm Auto-Discovery Status */}
      <div className="bg-gradient-to-r from-blue-50 to-indigo-50 p-4 rounded-xl border border-blue-200 shadow-xs">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Globe className="w-4 h-4 text-blue-600" />
            <span className="text-xs font-bold text-gray-900">Zero-Config Swarm Discovery</span>
          </div>
          <span className="text-[10px] font-semibold bg-blue-100 text-blue-700 px-2 py-0.5 rounded-full border border-blue-200">
            Active & Listening
          </span>
        </div>
        <div className="mt-2 grid grid-cols-1 sm:grid-cols-3 gap-2 text-[11px] text-gray-600">
          <div className="flex items-center gap-1.5 bg-white/70 p-2 rounded-lg border border-blue-100">
            <Wifi className="w-3.5 h-3.5 text-green-600" />
            <span>LAN Swarm (mDNS)</span>
          </div>
          <div className="flex items-center gap-1.5 bg-white/70 p-2 rounded-lg border border-blue-100">
            <Radio className="w-3.5 h-3.5 text-blue-600" />
            <span>WAN DNS / Pkarr</span>
          </div>
          <div className="flex items-center gap-1.5 bg-white/70 p-2 rounded-lg border border-blue-100">
            <Server className="w-3.5 h-3.5 text-indigo-600" />
            <span>BitTorrent DHT</span>
          </div>
        </div>
        <p className="text-[11px] text-gray-500 mt-2">
          Devices on the same Wi-Fi, Ethernet, or campus network discover each other automatically. For remote friends or cloud anchors, share your Node Ticket below.
        </p>
      </div>

      {/* Identity Section */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
        <div className="flex items-center justify-between border-b border-gray-100 pb-3">
          <div className="flex items-center gap-2.5">
            <Radio className="w-5 h-5 text-blue-600" />
            <h3 className="text-sm font-bold text-gray-900">Your Node Identity & Ticket</h3>
          </div>
          <span className="text-[10px] font-mono bg-green-50 text-green-700 px-2 py-0.5 rounded-full border border-green-200 flex items-center gap-1">
            <Activity className="w-3 h-3 text-green-600" />
            {nodeId ? 'Online' : 'Initializing...'}
          </span>
        </div>

        {nodeTicket && (
          <div>
            <label className="text-xs font-semibold text-gray-700">P2P Node Ticket (Recommended for Sharing)</label>
            <div className="mt-1 flex items-center gap-2">
              <input
                type="text"
                readOnly
                value={nodeTicket}
                className="w-full text-xs font-mono bg-blue-50/50 border border-blue-200 rounded-lg px-3 py-2 text-blue-900 select-all focus:outline-none truncate"
              />
              <button
                onClick={() => copyToClipboard(nodeTicket, true)}
                className="px-3 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-medium flex items-center gap-1.5 transition-colors"
                title="Copy Node Ticket"
              >
                {copiedTicket ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
                {copiedTicket ? 'Copied' : 'Copy'}
              </button>
            </div>
            <p className="text-[11px] text-gray-400 mt-1">
              This ticket encodes your Node ID, direct IPs, and relay addresses. Friends can paste this directly to link.
            </p>
          </div>
        )}

        <div>
          <label className="text-xs font-semibold text-gray-700">Raw Node ID (Ed25519 Public Key)</label>
          <div className="mt-1 flex items-center gap-2">
            <input
              type="text"
              readOnly
              value={nodeId || 'Initializing Iroh Quic Engine...'}
              className="w-full text-xs font-mono bg-gray-50 border border-gray-200 rounded-lg px-3 py-2 text-gray-700 select-all focus:outline-none"
            />
            <button
              onClick={() => nodeId && copyToClipboard(nodeId, false)}
              disabled={!nodeId}
              className="px-3 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-colors disabled:opacity-50"
              title="Copy Node ID"
            >
              {copiedId ? <Check className="w-3.5 h-3.5 text-green-600" /> : <Copy className="w-3.5 h-3.5" />}
              {copiedId ? 'Copied' : 'Copy'}
            </button>
          </div>
        </div>

        {nodeAddr && (
          <details className="text-xs text-gray-500 cursor-pointer pt-1">
            <summary className="font-semibold text-gray-700 hover:text-gray-900">
              Advanced: View Full JSON Address
            </summary>
            <div className="mt-2 flex items-center gap-2">
              <input
                type="text"
                readOnly
                value={nodeAddr}
                className="w-full text-[11px] font-mono bg-gray-50 border border-gray-200 rounded-lg px-2.5 py-1.5 text-gray-600 select-all focus:outline-none truncate"
              />
              <button
                onClick={() => copyToClipboard(nodeAddr, false)}
                className="px-2.5 py-1.5 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg text-xs font-medium flex items-center gap-1 transition-colors"
              >
                <Copy className="w-3 h-3" />
              </button>
            </div>
          </details>
        )}
      </div>

      {/* Connect to Peer / Anchor */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
        <div className="flex items-center gap-2.5 border-b border-gray-100 pb-3">
          <UserPlus className="w-5 h-5 text-blue-600" />
          <div>
            <h3 className="text-sm font-bold text-gray-900">Connect to Peer or Anchor Node</h3>
            <p className="text-xs text-gray-500">
              Paste a Node Ticket, JSON address, or Node ID to connect and sync the shared cloud.
            </p>
          </div>
        </div>

        <form onSubmit={handleConnectPeer} className="space-y-3">
          <div>
            <label className="text-xs font-semibold text-gray-700">Peer Address / Ticket *</label>
            <input
              type="text"
              placeholder="Paste Node Ticket (node...), JSON address, or Node ID"
              value={peerInput}
              onChange={(e) => setPeerInput(e.target.value)}
              className="w-full mt-1 text-xs font-mono bg-gray-50 border border-gray-200 rounded-lg px-3 py-2 focus:bg-white focus:border-blue-500 focus:outline-none transition-all"
              required
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
            <div
              className={`p-2.5 rounded-lg text-xs font-medium ${
                connectStatus.isError
                  ? 'bg-red-50 text-red-700 border border-red-200'
                  : 'bg-green-50 text-green-700 border border-green-200'
              }`}
            >
              {connectStatus.msg}
            </div>
          )}

          <button
            type="submit"
            disabled={connecting || !peerInput.trim()}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-semibold flex items-center gap-2 shadow-xs transition-colors disabled:opacity-50"
          >
            <UserPlus className="w-3.5 h-3.5" />
            {connecting ? 'Connecting...' : 'Connect to Peer'}
          </button>
        </form>
      </div>

      {/* Connected Peers List */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
        <div className="flex items-center justify-between border-b border-gray-100 pb-3">
          <div className="flex items-center gap-2.5">
            <Network className="w-5 h-5 text-blue-600" />
            <div>
              <h3 className="text-sm font-bold text-gray-900">Connected Peers ({peers.length})</h3>
              <p className="text-xs text-gray-500">Live peers actively connected in your P2P swarm.</p>
            </div>
          </div>
          <button
            onClick={loadNetworkInfo}
            className="p-1.5 rounded-md hover:bg-gray-100 text-gray-500"
            title="Refresh peer list"
          >
            <RefreshCw className="w-3.5 h-3.5" />
          </button>
        </div>

        {peers.length === 0 ? (
          <div className="text-center py-6 text-gray-400">
            <p className="text-xs">No remote peers connected yet.</p>
            <p className="text-[11px] text-gray-400 mt-1">
              Once another node opens the app on your network or you connect to an anchor node, it will appear here.
            </p>
          </div>
        ) : (
          <div className="divide-y divide-gray-100 border border-gray-100 rounded-lg overflow-hidden">
            {peers.map((p) => (
              <div key={p.node_id} className="p-3 bg-gray-50 flex items-center justify-between text-xs">
                <div>
                  <span className="font-mono font-semibold text-gray-800">{p.node_id.slice(0, 16)}...</span>
                  {p.relay_url && <p className="text-[10px] text-gray-400 truncate max-w-sm">{p.relay_url}</p>}
                </div>
                <div className="text-right">
                  <span className="text-[10px] bg-green-100 text-green-700 px-2 py-0.5 rounded font-mono font-medium">
                    {p.latency_ms ? `${p.latency_ms} ms` : 'Connected'}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Campus Wi-Fi Profile & Stealth Mode */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
        <div className="flex items-center gap-2.5 border-b border-gray-100 pb-3">
          <Shield className="w-5 h-5 text-amber-600" />
          <div>
            <h3 className="text-sm font-bold text-gray-900">Campus Wi-Fi Stealth & SSID Rules</h3>
            <p className="text-xs text-gray-500">
              Configure profile detection on university Wi-Fi networks (disables mDNS/DHT, enforces port 443 relay).
            </p>
          </div>
        </div>

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
        <div className="space-y-3 pt-2">
          <h4 className="text-xs font-bold text-gray-600 uppercase tracking-wider">
            Known Network SSID Rules
          </h4>

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
              disabled={isSavingRule || !newSsid.trim()}
              className="px-3.5 py-1.5 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded-lg text-xs font-semibold flex items-center gap-1 shadow-2xs transition cursor-pointer"
            >
              <Plus className="w-3.5 h-3.5" />
              <span>Add Rule</span>
            </button>
          </form>
        </div>
      </div>

      {/* Swarm Network Secret / Workspace Passphrase */}
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
                className="absolute right-2.5 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
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
            className="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg text-xs font-semibold shadow-xs transition-colors flex items-center gap-1.5"
          >
            <Lock className="w-3.5 h-3.5" />
            {savingSecret ? 'Saving Secret...' : 'Save Network Secret'}
          </button>
        </form>
      </div>

      {/* Custom Relay Configuration */}
      <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
        <div className="flex items-center gap-2.5 border-b border-gray-100 pb-3">
          <Server className="w-5 h-5 text-blue-600" />
          <div>
            <h3 className="text-sm font-bold text-gray-900">Custom Swarm Relay</h3>
            <p className="text-xs text-gray-500">
              Optional TLS 443 relay URL for bypassing restrictive campus firewalls.
            </p>
          </div>
        </div>

        <form onSubmit={handleSaveRelay} className="space-y-3">
          <div>
            <label className="text-xs font-semibold text-gray-700">Relay Server URL</label>
            <input
              type="text"
              value={relayUrl}
              onChange={(e) => setRelayUrl(e.target.value)}
              className="w-full mt-1 text-xs font-mono bg-gray-50 border border-gray-200 rounded-lg px-3 py-2 focus:bg-white focus:border-blue-500 focus:outline-none transition-all"
            />
          </div>
          <button
            type="submit"
            disabled={savingRelay}
            className="px-4 py-2 bg-gray-800 hover:bg-gray-900 text-white rounded-lg text-xs font-semibold shadow-xs transition-colors"
          >
            {savingRelay ? 'Saving...' : 'Save Relay Configuration'}
          </button>
        </form>
      </div>
    </div>
  );
}
