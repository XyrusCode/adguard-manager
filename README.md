# AdGuard Manager

A lightweight desktop app that routes your DNS through [AdGuard DNS](https://adguard-dns.io) to block ads and trackers at the network level, with live traffic monitoring and per-connection visibility.

Built with Tauri (Rust backend + system webview) — no bundled browser, minimal RAM usage.

---

## Features

- **DNS ad blocking** — point any network adapter to AdGuard DNS (94.140.14.14 / 94.140.15.15) with one click
- **Per-adapter control** — enable/disable per WiFi or Ethernet adapter independently
- **Live traffic** — real-time bytes/s in and out, per interface
- **Usage history** — browsable chart and table filtered by time period and interface
- **Active connections** — live view of TCP/UDP connections with process names, filterable by app

## Download

Grab the latest release for your platform from the [Releases](../../releases) page:

| Platform | File |
|---|---|
| Windows | `AdGuard.Manager_x.x.x_x64-setup.exe` |
| Linux | `adguard-manager_x.x.x_amd64.deb` / `.AppImage` |
| macOS (Intel) | `AdGuard.Manager_x.x.x_x64.dmg` |
| macOS (Apple Silicon) | `AdGuard.Manager_x.x.x_aarch64.dmg` |

## Permissions

Changing DNS settings requires elevated privileges:

- **Windows** — right-click the `.exe` → *Run as administrator*, or run the installed app as admin
- **Linux** — the app calls `nmcli`; ensure your user is in the `netdev` group or run with `sudo`
- **macOS** — you will be prompted for your password on the first DNS change

Monitoring (live traffic, connections) works without elevated privileges.

## Build from source

### Prerequisites

- [Rust](https://rustup.rs) stable toolchain
- [Node.js](https://nodejs.org) 18+
- Platform-specific Tauri dependencies — see the [Tauri docs](https://tauri.app/start/prerequisites/)

### Steps

```bash
git clone https://github.com/YOUR_USERNAME/adguard-manager
cd adguard-manager
npm install
npm run dev        # development
npm run build      # production installer
```

## How it works

The app sets your system DNS servers to AdGuard's resolvers using native OS tools:

| OS | Tool used |
|---|---|
| Windows | `netsh interface ip set dns` |
| Linux | `nmcli con mod` |
| macOS | `networksetup -setdnsservers` |

AdGuard DNS filters ads, trackers, and malware domains before they reach your machine — no browser extension needed, works for all apps system-wide.

Data (network samples) is stored locally in a SQLite database under your OS app-data directory. It is automatically pruned after 30 days.

## License

MIT
