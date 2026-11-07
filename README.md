# P2P Cooperative Campus Workspace (`p2pstud`)

> A lightweight, cross-platform academic file sharing desktop application built with **Tauri v2** and **Iroh** for restrictive enterprise university networks (Eduroam).
> Full cross-platform support for **Linux** (Debian, Ubuntu, Fedora, Arch) and **Windows** (10 & 11).

---

## 🚀 Key Features

* **Host-to-Play Cooperative Storage**: Students dedicate a configurable quota (default: 5 GB, up to Full Mirror) to swarm and seed course files to classmates at LAN speeds.
* **Campus NAT / Firewall Traversal**: Embedded **Iroh** uses QUIC hole punching and seamlessly falls back to custom TLS port 443 relays when campus Wi-Fi enforces symmetric NAT or client isolation.
* **BLAKE3 Content Addressing**: All files and chunks are verified against BLAKE3 hashes via `iroh-blobs` out of the box—no custom socket protocols or roll-your-own cryptography.
* **Smart LRU Cache Eviction**: Background worker evicts oldest unpinned blobs when disk usage exceeds 90% until it falls back below 80%.
* **Pin for Offline**: Right-click to keep any slide deck or assignment offline and exempt from eviction.
* **In-App Slide & Note Viewers**: Embedded PDF reader with page rendering and Markdown editor with live preview.
* **Background Tray Persistence**: Minimizes to the system tray so P2P seeding continues in the background even after closing the main window.
* **Headless VPS Anchor**: An inexpensive cloud VPS ($3–$5/mo) provides 100% persistent availability. Features a staged token-bucket egress rate limiter (150 KB/s per session, 2 MB/s aggregate) in `anchor.rs`, with recommended kernel-level `tc`/`wondershaper` bandwidth caps for VPS providers.

---

## 🛠 Tech Stack

- **Desktop Shell**: Tauri v2 (Rust backend, React + Vite frontend)
- **P2P Layer**: Embedded `iroh`, `iroh-blobs`, `iroh-net`
- **Local DB**: Embedded SQLite (`rusqlite` with bundled SQLite source)
- **Frontend**: React 18, TailwindCSS, Lucide Icons, PDF.js, Marked
- **Anchor Daemon**: Rust binary with optional single-container Docker setup

---

## 🐧 Linux Support & Prerequisites

### 1. System Dependencies
On Linux distributions, ensure the required WebKitGTK and AppIndicator libraries are installed:

#### Ubuntu / Debian / Mint:
```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
    libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

#### Fedora / RHEL:
```bash
sudo dnf install -y webkit2gtk4.1-devel openssl-devel libappindicator-gtk3-devel \
    librsvg2-devel @development-tools
```

#### Arch Linux / Manjaro:
```bash
sudo pacman -S --needed webkit2gtk-4.1 base-devel openssl libayatana-appindicator librsvg
```

### 2. Building Packages for Linux
```bash
# Build native .deb and .AppImage packages
npm run build
npx tauri build
# Output bundles will be located in:
# target/release/bundle/deb/
# target/release/bundle/appimage/
```

---

## 💻 Local Development Setup

### Prerequisites
- [Node.js](https://nodejs.org/) (v20+)
- [Rust](https://rustup.rs/) (v1.77+)
- Platform build dependencies (see Linux section above or Visual Studio C++ Build Tools on Windows)

### Run Desktop App
```bash
# 1. Install frontend dependencies
npm install

# 2. Run in development mode (starts Vite + Tauri)
npm run tauri dev
```

### Run Unit Tests
```bash
# Run Rust unit tests (Eviction, Path traversal, SQLite tracker, Identity)
cargo test --workspace

# Run Frontend component tests (Vitest + Testing Library)
npm run test
```

---

## ⚓ Headless Anchor Node Deployment (Linux VPS)

The anchor node persists 100% of workspace files and hosts the TLS 443 relay for NAT traversal.

> **Bandwidth Throttling on Light VPS Instances:**
> While `anchor.rs` includes a staged userspace `SessionLimiter` module, VPS operators requiring hard kernel-level egress throttling can easily enforce traffic shaping using Linux `tc` / `wondershaper`:
> ```bash
> # Cap outbound bandwidth to 2 MB/s (16 Mbit/s) on interface eth0
> sudo wondershaper eth0 102400 16384
> # Or clear limits: sudo wondershaper clear eth0
> ```

### Option A: Native Binary with systemd
```bash
cargo build --release --bin anchor
sudo cp target/release/anchor /usr/local/bin/
sudo cp docker/p2pstud-anchor.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now p2pstud-anchor
```

### Option B: Single-Container Docker Deploy
```bash
cd docker
cp .env.example .env
# Edit .env: RELAY_DOMAIN and optional WORKSPACE_ID (omitted by default for shared swarm)
docker compose up -d
```

---

## 🔒 Security Architecture

1. **Strict Path Sanitization**: Path inputs undergo canonicalization checks against directory traversal (`safe_join`).
2. **Cryptographic Identity**: Node Ed25519 SecretKey stored with owner-only access permissions (`0o600` on Linux, restricted Windows ACLs).
3. **No Unauthenticated Listeners**: Control plane is strictly embedded in-process via Tauri IPC—no open HTTP/RPC ports bound to `0.0.0.0` on student laptops.
