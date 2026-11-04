import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { 
  HardDrive, 
  ShieldCheck, 
  Zap, 
  Network, 
  Copy, 
  Check, 
  UserPlus, 
  Radio, 
  Activity, 
  Server,
  RefreshCw,
  FolderOpen,
  Trash2,
  Globe,
  Wifi,
  Lock,
  Key,
  Eye,
  EyeOff
} from 'lucide-react';
import { StorageStats, PeerInfo } from '../../types';

interface CacheSliderProps {
  stats: StorageStats | null;
  onStatsUpdated?: () => void;
}

function formatGigabytes(bytes: number): string {
  return (bytes / (1024 * 1024 * 1024)).toFixed(2);
}

export const CacheSlider: React.FC<CacheSliderProps> = ({ stats, onStatsUpdated }) => {
  const [activeTab, setActiveTab] = useState<'storage' | 'network'>('storage');

  // Storage Settings
  const [sliderGb, setSliderGb] = useState<number>(5);
  const [fullMirror, setFullMirror] = useState<boolean>(false);
  const [savingStorage, setSavingStorage] = useState<boolean>(false);

  // Network & Identity Settings
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

  useEffect(() => {
    if (stats) {
      setFullMirror(stats.full_mirror);
      if (!stats.full_mirror && stats.quota_bytes < 1000 * 1024 * 1024 * 1024) {
        const gb = Math.round(stats.quota_bytes / (1024 * 1024 * 1024));
        setSliderGb(Math.max(3, Math.min(50, gb)));
      }
    }
  }, [stats]);

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
    loadNetworkInfo();
    const interval = setInterval(loadNetworkInfo, 3000);
    return () => clearInterval(interval);
  }, []);

  const handleSliderChange = async (newGb: number) => {
    setSliderGb(newGb);
    setSavingStorage(true);
    try {
      const bytes = newGb * 1024 * 1024 * 1024;
      await invoke('set_cache_limit', { bytes, fullMirror: false });
      if (onStatsUpdated) onStatsUpdated();
    } catch (err) {
      console.error('Failed to update cache limit:', err);
    } finally {
      setSavingStorage(false);
    }
  };

  const handleToggleFullMirror = async () => {
    const nextMirror = !fullMirror;
    setFullMirror(nextMirror);
    setSavingStorage(true);
    try {
      await invoke('set_cache_limit', {
        bytes: nextMirror ? null : sliderGb * 1024 * 1024 * 1024,
        fullMirror: nextMirror,
      });
      if (onStatsUpdated) onStatsUpdated();
    } catch (err) {
      console.error('Failed to toggle full mirror:', err);
    } finally {
      setSavingStorage(false);
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
      if (onStatsUpdated) onStatsUpdated();
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

  const handlePurgeCache = async () => {
    if (confirm('Are you sure you want to purge unpinned cached blobs? Pinned files and cloud files will remain safe.')) {
      try {
        const freed = await invoke<number>('purge_unpinned_cache');
        const freedMb = (freed / (1024 * 1024)).toFixed(1);
        alert(`Purged unpinned cache! Freed ${freedMb} MB.`);
        if (onStatsUpdated) onStatsUpdated();
      } catch (err) {
        alert(`Failed to purge cache: ${err}`);
      }
    }
  };

  const handleOpenDataDir = async () => {
    try {
      await invoke('open_data_dir');
    } catch (err) {
      alert(`Failed to open data directory: ${err}`);
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

  const targetHostGb = (sliderGb * 0.8).toFixed(1);

  return (
    <div className="max-w-3xl mx-auto space-y-6">
      {/* Navigation Tabs */}
      <div className="flex border-b border-gray-200 gap-8">
        <button
          onClick={() => setActiveTab('network')}
          className={`flex items-center gap-2 pb-3 px-1 text-sm font-semibold border-b-2 transition-colors ${
            activeTab === 'network'
              ? 'border-blue-600 text-blue-600'
              : 'border-transparent text-gray-500 hover:text-gray-700'
          }`}
        >
          <Network className="w-4 h-4" />
          Network & Swarm Peers
        </button>
        <button
          onClick={() => setActiveTab('storage')}
          className={`flex items-center gap-2 pb-3 px-1 text-sm font-semibold border-b-2 transition-colors ${
            activeTab === 'storage'
              ? 'border-blue-600 text-blue-600'
              : 'border-transparent text-gray-500 hover:text-gray-700'
          }`}
        >
          <HardDrive className="w-4 h-4" />
          Cache & Disk Quota
        </button>
      </div>

      {/* Network Configuration Tab */}
      {activeTab === 'network' && (
        <div className="space-y-6">
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
                  This single ticket encodes your Node ID, direct IPs, and relay addresses. Friends can paste this directly to link.
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
                  Leave blank to use the default open network. Changing this secret isolates your workspace from random strangers on GitHub.
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
      )}

      {/* Storage & Disk Quota Tab */}
      {activeTab === 'storage' && (
        <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-6">
          <div>
            <h2 className="text-base font-bold text-gray-900">Host-to-Play Cache Allocation</h2>
            <p className="text-xs text-gray-500 mt-1">
              Dedicating local disk space seeds course materials to classmates on campus.
            </p>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="p-4 bg-gray-50 rounded-lg border border-gray-100">
              <div className="text-xs font-medium text-gray-500">Currently Used</div>
              <div className="text-2xl font-bold text-gray-900 mt-1">{formatGigabytes(stats?.used_bytes || 0)} GB</div>
            </div>
            <div className="p-4 bg-gray-50 rounded-lg border border-gray-100">
              <div className="text-xs font-medium text-gray-500">Pinned (Protected)</div>
              <div className="text-2xl font-bold text-green-600 mt-1">{formatGigabytes(stats?.pinned_bytes || 0)} GB</div>
            </div>
          </div>

          {/* Active 80% Swarm Hosting Banner */}
          <div className="p-4 bg-blue-50/70 border border-blue-200 rounded-lg">
            <div className="flex items-center gap-2 text-xs font-semibold text-blue-900">
              <Zap className="w-4 h-4 text-blue-600" />
              <span>Active Cooperative Swarm Seeding (~80% target)</span>
            </div>
            <p className="text-[11px] text-blue-700 mt-1 leading-relaxed">
              Your node actively pre-fetches and hosts course files up to <strong>~{targetHostGb} GB</strong> (80% of quota). 
              This ensures unpinned files are seeded across campus so classmates can download them at LAN speeds.
            </p>
          </div>

          <div className="space-y-4 pt-2">
            <div className="flex justify-between items-center text-xs">
              <label htmlFor="cache-slider" className="font-semibold text-gray-700">Cache Quota Limit</label>
              <span className="font-bold text-blue-600 text-sm">{sliderGb} GB</span>
            </div>

            <input
              id="cache-slider"
              type="range"
              min="3"
              max="50"
              value={sliderGb}
              disabled={fullMirror || savingStorage}
              onChange={(e) => handleSliderChange(parseInt(e.target.value, 10))}
              className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-blue-600 disabled:opacity-50"
            />

            <div className="flex justify-between text-[11px] text-gray-400">
              <span>Min: 3 GB</span>
              <span>Recommended: 5 GB</span>
              <span>50 GB</span>
            </div>
          </div>

          <div className="p-4 bg-gray-50 rounded-lg border border-gray-200 flex items-start gap-3">
            <input
              type="checkbox"
              id="full-mirror"
              checked={fullMirror}
              disabled={savingStorage}
              onChange={handleToggleFullMirror}
              className="mt-1 h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
            />
            <label htmlFor="full-mirror" className="cursor-pointer">
              <div className="flex items-center gap-1.5 text-xs font-bold text-gray-900">
                <Zap className="w-3.5 h-3.5 text-amber-500 fill-amber-500" />
                Enable Full Mirror Mode
              </div>
              <p className="text-[11px] text-gray-500 mt-0.5">
                Disables the automatic LRU eviction worker and auto-downloads all course workspace blobs locally to provide 100% offline redundancy.
              </p>
            </label>
          </div>

          <div className="flex items-center gap-2 text-xs text-gray-600">
            <ShieldCheck className="w-4 h-4 text-green-600 shrink-0" />
            <span>Oldest unpinned items are automatically evicted when cache exceeds 90% capacity.</span>
          </div>

          {/* Disk Actions */}
          <div className="pt-4 border-t border-gray-100 flex flex-wrap gap-3">
            <button
              onClick={handleOpenDataDir}
              className="px-3 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-colors"
            >
              <FolderOpen className="w-3.5 h-3.5 text-gray-600" />
              Open Data Folder
            </button>

            <button
              onClick={handlePurgeCache}
              className="px-3 py-2 bg-red-50 hover:bg-red-100 text-red-700 rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-colors"
            >
              <Trash2 className="w-3.5 h-3.5 text-red-600" />
              Purge Unpinned Cache
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
