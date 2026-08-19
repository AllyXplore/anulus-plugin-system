# AxAIHub Plugin System

> **English** | [简体中文](README.zh-CN.md)

The open-source plugin format, packaging toolchain, and developer documentation for **AxAIHub** — a privacy-first, local-first AI workspace on Android.

Plugins are AxAIHub's extension mechanism: HTML plugins run in a standalone window (mini-program style) or an iframe, and talk to the host through a postMessage bridge with access to storage / network / clipboard / AI chat / speech / notifications / user profile / camera / mic / location, etc. Permissions follow a strict **declare-then-authorize** model.

## Repository Structure

```
├── docs/                     # Documentation (spec / tutorial / architecture / API reference)
│   ├── 01-插件格式规范.md     # .ns metadata + .axext/.axex binary format + security model (authoritative)
│   ├── 02-打包教程.md        # Step-by-step plugin development & packaging tutorial
│   ├── 03-架构设计.md        # Module layout / data flow / security architecture
│   └── 04-API参考.md         # Wire protocol + capability list + AI callbacks / WASM / office integration
├── tools/
│   ├── axbuild/              # Cross-platform packaging CLI (Rust): keypair / new / pack / info
│   └── README.md             # Tool overview
└── samples/
    ├── axplugin-sdk.js       # Reference plugin SDK (AX.call / AX.aiFunction / AX.on, zero-dependency)
    └── hello-plugin/         # Minimal example plugin (ready to pack)
```

> The docs themselves are written in Chinese. `01-插件格式规范.md` (spec) is authoritative; `04-API参考.md` is the protocol & capability reference.

## Quick Start

```bash
# 1. Build the packaging tool
cd tools/axbuild
cargo build --release

# 2. Generate a signing keypair (only needed for encrypted .axex packages)
./target/release/axbuild keypair

# 3. Scaffold and pack a plugin
./target/release/axbuild new ./my-plugin --id com.example.demo --name "Demo" --author "me"
./target/release/axbuild pack ./my-plugin my-plugin.axext
```

Import the resulting `.axext` / `.axex` into AxAIHub (Plugins -> Import) and it runs.

## Docs at a Glance

- What a plugin **can do & how it communicates** → `docs/04-API参考.md`
- **Package format, encryption & signing** → `docs/01-插件格式规范.md`
- **Write a plugin from scratch** → `docs/02-打包教程.md`

## Key Features

- **Two distribution formats**: `.axext` plain ZIP (open sharing); `.axex` AES-256-GCM encrypted ZIP with optional Ed25519 author signature (commercial/closed-source, prevents source leaking)
- **Strict permission model**: a plugin must declare permissions in its `.ns` manifest; the user grants them at runtime or in the manage panel. Undeclared = unusable
- **Standalone window + injected bridge**: plugins run in their own window by default (mini-program style, usable in parallel with the host app); the bridge is injected by the host and all communication goes through postMessage. Sensitive commands (file reads / keys / terminal, etc.) are gated to the main window at the Rust layer — plugins can never reach them
- **AI can call plugins**: plugins declaring `ai.callable` expose functions the host AI can invoke as tools (`AX.aiFunction`, protocol in `docs/04-API参考.md` §4.2)
- **Wallet protection**: `ai.chat` uses user-selected plans + configurable quotas (per min/day/month + token budget); API keys never enter the plugin
- **System-permission borrowing**: plugins can request system permissions (camera / mic / location / notifications / contacts / calendar / SMS / body sensors / Bluetooth / nearby Wi-Fi / audio, etc.) through the host channel, with the result delivered back via the `permission_result` event

## License

- `docs/01-插件格式规范.md` (the NS language & `.axext` / `.axex` format spec): **[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)** (`LICENSE-CC-BY-4.0.txt`)
- Everything else (other docs, `tools/axbuild`, `samples`): **MIT** (`LICENSE`)

## Author & Maintenance

## Author & Maintenance
- Maintained by the AllyXplore team
- Lead developer: Ax丶现
