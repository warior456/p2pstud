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
  Trash2
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
  const [nodeAddr, setNodeAddr] = useState<string>('');
  const [relayUrl, setRelayUrl] = useState<string>('https://use1-1.relay.iroh.network/');
  const [peerInput, setPeerInput] = useState<string>('');
  const [peerRelayInput, setPeerRelayInput] = useState<string>('');
  const [connectStatus, setConnectStatus] = useState<{ msg: string; isError: boolean } | null>(null);
  const [connecting, setConnecting] = useState<boolean>(false);
  const [savingRelay, setSavingRelay] = useState<boolean>(false);
  const [copiedId, setCopiedId] = useState<boolean>(false);
  const [copiedAddr, setCopiedAddr] = useState<boolean>(false);
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
      const addr = await invoke<string>('get_node_addr');
      if (addr) setNodeAddr(addr);
    } catch {
      // Optional
    }

    try {
      const settings = await invoke<{ relay_url: string }>('get_app_settings');
      if (settings?.relay_url) setRelayUrl(settings.relay_url);
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
      console.error('Failed to set cache limit:', err);
    } finally {
      setSavingStorage(false);
    }
  };

  const handleFullMirrorToggle = async (enabled: boolean) => {
    setFullMirror(enabled);
    setSavingStorage(true);
    try {
      const bytes = enabled ? null : sliderGb * 1024 * 1024 * 1024;
      await invoke('set_cache_limit', { bytes, fullMirror: enabled });
      if (onStatsUpdated) onStatsUpdated();
    } catch (err) {
      console.error('Failed to update mirror mode:', err);
    } finally {
      setSavingStorage(false);
    }
  };

  const handlePurgeCache = async () => {
    try {
      const freed = await invoke<number>('purge_unpinned_cache');
      alert(`Purged ${(freed / (1024 * 1024)).toFixed(1)} MB from unpinned cache.`);
      if (onStatsUpdated) onStatsUpdated();
    } catch (err) {
      alert(`Failed to purge cache: ${err}`);
    }
  };

  const handleOpenDataDir = async () => {
    try {
      await invoke('open_data_dir');
    } catch (err) {
      alert(`Failed to open data directory: ${err}`);
    }
  };

  const handleSaveRelay = async () => {
    setSavingRelay(true);
    try {
      await invoke('set_relay_url', { relayUrl });
      alert('Relay URL saved. Restart app or reconnect peers to apply.');
    } catch (err) {
      alert(`Failed to save relay URL: ${err}`);
    } finally {
      setSavingRelay(false);
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
        relayUrl: peerRelayInput.trim() ? peerRelayInput.trim() : null,
      });
      setConnectStatus({ msg: 'Connected successfully to peer!', isError: false });
      setPeerInput('');
      setPeerRelayInput('');
      loadNetworkInfo();
      if (onStatsUpdated) onStatsUpdated();
    } catch (err: any) {
      setConnectStatus({ msg: `Connection failed: ${err}`, isError: true });
    } finally {
      setConnecting(false);
    }
  };

  const copyToClipboard = (text: string, isAddr: boolean) => {
    navigator.clipboard.writeText(text);
    if (isAddr) {
      setCopiedAddr(true);
      setTimeout(() => setCopiedAddr(false), 2000);
    } else {
      setCopiedId(true);
      setTimeout(() => setCopiedId(false), 2000);
    }
  };

  const usedGb = stats ? formatGigabytes(stats.used_bytes) : '0.00';
  const pinnedGb = stats ? formatGigabytes(stats.pinned_bytes) : '0.00';

  return (
    <div className="max-w-3xl mx-auto space-y-6">
      {/* Settings Navigation Tabs */}
      <div className="flex border-b border-gray-200 gap-4">
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
          {/* Identity Section */}
          <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
            <div className="flex items-center justify-between border-b border-gray-100 pb-3">
              <div className="flex items-center gap-2.5">
                <Radio className="w-5 h-5 text-blue-600" />
                <h3 className="text-sm font-bold text-gray-900">Your Node Identity</h3>
              </div>
              <span className="text-[10px] font-mono bg-green-50 text-green-700 px-2 py-0.5 rounded-full border border-green-200 flex items-center gap-1">
                <Activity className="w-3 h-3 text-green-600" />
                {nodeId ? 'Online' : 'Initializing...'}
              </span>
            </div>

            <div>
              <label className="text-xs font-semibold text-gray-700">Node ID (Ed25519 Public Key)</label>
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
              <p className="text-[11px] text-gray-400 mt-1">
                Share this Node ID with peers or friends so they can add you to their shared swarm.
              </p>
            </div>

            {nodeAddr && (
              <div>
                <label className="text-xs font-semibold text-gray-700">Full Node Address (Direct & Relay Hint)</label>
                <div className="mt-1 flex items-center gap-2">
                  <input
                    type="text"
                    readOnly
                    value={nodeAddr}
                    className="w-full text-xs font-mono bg-gray-50 border border-gray-200 rounded-lg px-3 py-2 text-gray-700 select-all focus:outline-none truncate"
                  />
                  <button
                    onClick={() => copyToClipboard(nodeAddr, true)}
                    className="px-3 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-colors"
                    title="Copy Full Node Address"
                  >
                    {copiedAddr ? <Check className="w-3.5 h-3.5 text-green-600" /> : <Copy className="w-3.5 h-3.5" />}
                    {copiedAddr ? 'Copied' : 'Copy'}
                  </button>
                </div>
              </div>
            )}
          </div>

          {/* Connect to Peer / Anchor */}
          <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
            <div className="flex items-center gap-2.5 border-b border-gray-100 pb-3">
              <UserPlus className="w-5 h-5 text-blue-600" />
              <div>
                <h3 className="text-sm font-bold text-gray-900">Connect to Peer or Anchor Node</h3>
                <p className="text-xs text-gray-500">
                  Dial a specific student laptop or cloud anchor VPS to link swarms and sync files.
                </p>
              </div>
            </div>

            <form onSubmit={handleConnectPeer} className="space-y-3">
              <div>
                <label className="text-xs font-semibold text-gray-700">Peer Node ID *</label>
                <input
                  type="text"
                  placeholder="Paste peer's Node ID (e.g. uqpewh6pnmgq...)"
                  value={peerInput}
                  onChange={(e) => setPeerInput(e.target.value)}
                  className="w-full mt-1 text-xs font-mono bg-gray-50 border border-gray-200 rounded-lg px-3 py-2 focus:bg-white focus:border-blue-500 focus:outline-none transition-all"
                  required
                />
              </div>

              <div>
                <label className="text-xs font-semibold text-gray-700">Relay URL (Optional)</label>
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
                  Share your Node ID above or connect to a friend's Node ID to begin direct file sharing.
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

          {/* Relay Server Configuration */}
          <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-4">
            <div className="flex items-center gap-2.5 border-b border-gray-100 pb-3">
              <Server className="w-5 h-5 text-blue-600" />
              <div>
                <h3 className="text-sm font-bold text-gray-900">Swarm Relay & NAT Fallback</h3>
                <p className="text-xs text-gray-500">
                  Custom TLS relay for university campus networks (Eduroam) and symmetric NAT traversal.
                </p>
              </div>
            </div>

            <div className="flex gap-2">
              <input
                type="text"
                value={relayUrl}
                onChange={(e) => setRelayUrl(e.target.value)}
                placeholder="https://use1-1.relay.iroh.network/"
                className="flex-1 text-xs font-mono bg-gray-50 border border-gray-200 rounded-lg px-3 py-2 focus:bg-white focus:border-blue-500 focus:outline-none"
              />
              <button
                onClick={handleSaveRelay}
                disabled={savingRelay}
                className="px-4 py-2 bg-gray-800 hover:bg-gray-900 text-white rounded-lg text-xs font-semibold transition-colors disabled:opacity-50"
              >
                {savingRelay ? 'Saving...' : 'Save Relay'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Storage Allocation Tab */}
      {activeTab === 'storage' && (
        <div className="bg-white p-6 rounded-xl border border-gray-200 shadow-xs space-y-6">
          <div className="flex items-center gap-3 border-b border-gray-100 pb-4">
            <HardDrive className="w-6 h-6 text-blue-600" />
            <div>
              <h2 className="text-lg font-bold text-gray-900">Host-to-Play Cache Allocation</h2>
              <p className="text-xs text-gray-500">
                Dedicating local disk space seeds course materials to peers on campus.
              </p>
            </div>
          </div>

          {/* Usage breakdown */}
          <div className="grid grid-cols-2 gap-4 bg-gray-50 p-4 rounded-lg">
            <div>
              <span className="text-xs text-gray-500 font-medium">Currently Used</span>
              <p className="text-xl font-bold text-gray-900 mt-1">{usedGb} GB</p>
            </div>
            <div>
              <span className="text-xs text-gray-500 font-medium">Pinned (Protected)</span>
              <p className="text-xl font-bold text-green-700 mt-1">{pinnedGb} GB</p>
            </div>
          </div>

          {/* Quota slider */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <label htmlFor="cache-slider" className="text-sm font-semibold text-gray-800">
                Cache Quota Limit
              </label>
              <span className="text-sm font-bold text-blue-600">
                {fullMirror ? 'Unlimited (Full Mirror)' : `${sliderGb} GB`}
              </span>
            </div>

            <input
              id="cache-slider"
              type="range"
              min={3}
              max={50}
              step={1}
              value={sliderGb}
              disabled={fullMirror || savingStorage}
              onChange={(e) => handleSliderChange(parseInt(e.target.value, 10))}
              className="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-blue-600 disabled:opacity-40"
            />

            <div className="flex justify-between text-[11px] text-gray-400">
              <span>Min: 3 GB</span>
              <span>Recommended: 5 GB</span>
              <span>50 GB</span>
            </div>
          </div>

          {/* Full mirror toggle */}
          <div className="border border-blue-100 bg-blue-50/50 p-4 rounded-lg flex items-start gap-3">
            <input
              type="checkbox"
              id="full-mirror"
              checked={fullMirror}
              disabled={savingStorage}
              onChange={(e) => handleFullMirrorToggle(e.target.checked)}
              className="mt-0.5 h-4 w-4 rounded text-blue-600 focus:ring-blue-500 border-gray-300"
            />
            <div>
              <label htmlFor="full-mirror" className="text-xs font-bold text-gray-900 flex items-center gap-1.5 cursor-pointer">
                <Zap className="w-3.5 h-3.5 text-amber-500 fill-amber-500" />
                Enable Full Mirror Mode
              </label>
              <p className="text-xs text-gray-500 mt-0.5">
                Disables the automatic LRU eviction worker and auto-downloads all workspace blobs locally to provide 100% offline redundancy.
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2 text-xs text-gray-500 border-t border-gray-100 pt-4">
            <ShieldCheck className="w-4 h-4 text-green-600 flex-shrink-0" />
            <span>Oldest unpinned items are automatically evicted when cache reaches 90% capacity.</span>
          </div>

          {/* Diagnostic & Disk Utilities */}
          <div className="border-t border-gray-100 pt-5 space-y-3">
            <h3 className="text-xs font-bold text-gray-700 uppercase tracking-wider">
              Diagnostic & Disk Management
            </h3>
            <div className="flex flex-wrap gap-3">
              <button
                onClick={handlePurgeCache}
                className="px-3.5 py-2 bg-red-50 hover:bg-red-100 text-red-700 rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-colors border border-red-200"
              >
                <Trash2 className="w-3.5 h-3.5 text-red-600" />
                Purge Unpinned Cache
              </button>
              <button
                onClick={handleOpenDataDir}
                className="px-3.5 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-colors"
              >
                <FolderOpen className="w-3.5 h-3.5 text-gray-600" />
                Open Data Folder on Disk
              </button>
            </div>
            <p className="text-[11px] text-gray-400">
              Closing the window [X] minimizes to the system tray so that P2P file seeding continues in the background. To quit completely, right-click the system tray icon and choose Quit.
            </p>
          </div>
        </div>
      )}
    </div>
  );
};
export default CacheSlider;
