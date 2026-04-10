# CoS Desktop

A native desktop app that turns Claude Code into an autonomous Chief of Staff. Replaces cron-based polling with a Rust event loop — zero token burn when idle, instant message delivery when active.

Built with Tauri v2 (Rust) + SolidJS + TailwindCSS v4.

## What It Does

- **Messaging** — Bidirectional chat with your Claude CoS agent. Send text, images, and questions that block until answered.
- **Sessions** — Live tmux terminal view. Monitor and send commands to any session/window directly.
- **Dashboard** — Project overview pulled from your Obsidian vault (optional).
- **Todos** — Daily task tracking, addable from the app or by the CoS agent.
- **Telegram Fallback** — When you toggle to "away", messages route through Telegram instead.
- **Setup Wizard** — First-run onboarding detects prerequisites and configures everything.

The app runs a local HTTP server (default `localhost:7890`) that the CoS agent uses to communicate. No crons, no polling from Claude's side — the app pushes events to Claude only when something happens.

## Prerequisites

| Dependency | macOS | Windows (WSL) |
|---|---|---|
| **Rust** (1.77.2+) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` (inside WSL) |
| **Node.js** (18+) | `brew install node` | `winget install OpenJS.NodeJS.LTS` |
| **tmux** | `brew install tmux` | `sudo apt install tmux` (inside WSL) |
| **Claude Code CLI** | `npm install -g @anthropic-ai/claude-code` | `npm install -g @anthropic-ai/claude-code` (inside WSL) |

### Windows-specific

Windows requires WSL with Ubuntu and Visual Studio Build Tools:

```powershell
# Install WSL if not already
wsl --install

# Install Visual Studio Build Tools (required by Tauri)
winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

Then install Rust, tmux, and Claude Code **inside WSL**.

## Install from Source

### macOS

```bash
git clone https://github.com/hecarrillo/chief-of-staff.git
cd chief-of-staff
npm install
npx tauri build
```

The `.dmg` installer will be at:
```
src-tauri/target/release/bundle/dmg/CoS Desktop_0.1.0_aarch64.dmg
```

Drag to `/Applications` and launch from Spotlight.

### Windows

```powershell
git clone https://github.com/hecarrillo/chief-of-staff.git
cd chief-of-staff
npm install
npx tauri build
```

The `.msi` installer will be at:
```
src-tauri\target\release\bundle\msi\CoS Desktop_0.1.0_x64_en-US.msi
```

Run the installer. Launch from Start Menu.

## Development

```bash
npm install
npx tauri dev
```

Frontend hot-reloads at `localhost:1420`. Rust backend recompiles on save.

## Configuration

On first launch, the setup wizard walks you through configuration. Settings are stored at:

| | Path |
|---|---|
| **macOS** | `~/.cos-desktop/config.json` |
| **Windows** | `C:\Users\<you>\.cos-desktop\config.json` |

### Config fields

| Field | Default | Description |
|---|---|---|
| `cos_session` | `cos` | tmux session name. Use different names for multiple instances. |
| `cos_cwd` | `~` | Working directory where Claude starts. |
| `http_port` | `7890` | HTTP bridge port. Use different ports for multiple instances. |
| `vault_path` | *(empty)* | Obsidian vault path for the Dashboard page. |
| `bot_token` | *(empty)* | Telegram bot token (optional, for away-mode fallback). |
| `chat_id` | *(empty)* | Telegram chat ID (optional). |
| `cos_framework` | *(built-in)* | System prompt for the CoS agent. Editable in Settings. |

### Multiple instances

Run multiple CoS agents simultaneously by giving each a unique session name and port:

- Instance 1: session `cos`, port `7890`
- Instance 2: session `cos-ops`, port `7891`

Each instance needs its own Telegram bot token if using Telegram fallback.

## Architecture

```
┌─────────────────────────────────────────┐
│            CoS Desktop (Tauri)          │
│                                         │
│  SolidJS Frontend ◄──── Tauri Events    │
│       │                     ▲           │
│       │ IPC                 │           │
│       ▼                     │           │
│  Rust Backend               │           │
│   ├── HTTP Server (axum) ◄──┼── Claude  │
│   ├── Telegram Poller       │   (curl)  │
│   ├── Vault Watcher         │           │
│   ├── Response Watcher      │           │
│   └── tmux Manager ────────►│           │
│                        tmux session     │
└─────────────────────────────────────────┘
```

- **App to Claude**: Rust sends keystrokes to the tmux session
- **Claude to App**: Claude uses `curl POST localhost:7890/message`
- **Telegram**: Rust polls Telegram API, forwards messages both directions
- **Vault**: `notify` crate watches Obsidian files for dashboard updates

## License

[Business Source License 1.1](LICENSE) — free for non-commercial use. Commercial use requires a paid license. Converts to Apache 2.0 on 2030-04-09.

Contact: hector@realanalytica.com
