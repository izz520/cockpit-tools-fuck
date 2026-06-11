# Codex Lite

Codex Lite is a lightweight local desktop app for managing Codex accounts. It is built as a smaller, ad-free tool focused on importing accounts, switching the active Codex auth file, checking quota, and inspecting local settings and logs.

The project is still early. It is suitable for local testing and small-circle use, but public release validation is still in progress.

## Features

- List locally stored Codex accounts.
- Import the current local `~/.codex/auth.json`.
- Import Codex auth JSON from selected files.
- Preview batch JSON file imports before confirming selected items.
- Import Codex auth JSON from pasted text.
- Add accounts from token fields.
- Add API key accounts with an optional display name and base URL.
- Complete OAuth import through an automatic localhost callback or manual callback URL fallback.
- Switch the active Codex auth file after a confirmation prompt.
- Back up `~/.codex/auth.json` before writing a selected account.
- Refresh quota for OAuth accounts through the current quota API.
- Show stale quota state when refresh fails.
- Show Settings with local paths and auth file state.
- Show Logs with sensitive values redacted.
- Store app data locally with versioned JSON.
- Receive OAuth browser callbacks on localhost when port `1455` is available, with manual callback URL paste as a fallback.

## Tech Stack

- Tauri 2
- Rust
- React
- TypeScript
- Vite
- Zustand
- pnpm

## Requirements

- Node.js 20 or newer.
- pnpm.
- Rust toolchain with `rustc` and `cargo`.
- Tauri 2 system dependencies for your OS.

Check the toolchain:

```bash
node --version
pnpm --version
rustc --version
cargo --version
```

On macOS, if Rust is installed but `cargo` is not found in the current shell, try:

```bash
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/opt/rustup/bin:$PATH"
```

## Platform Dependencies

Codex Lite follows the Tauri 2 desktop dependency requirements.

### macOS

- macOS development tools from Xcode Command Line Tools.
- Rust stable toolchain.
- Node.js and pnpm.

Install Xcode Command Line Tools if needed:

```bash
xcode-select --install
```

### Windows

- Microsoft Visual Studio Build Tools with the C++ desktop workload.
- WebView2 runtime.
- Rust stable MSVC toolchain.
- Node.js and pnpm.

### Linux

Ubuntu 22.04 Docker packaging has passed locally for `.deb`, `.rpm`, and `.AppImage`. Typical Tauri dependencies include WebKitGTK, GTK, appindicator, librsvg, xdg-utils, rpm, and build tooling packages. Use your distribution package manager and the Tauri 2 Linux setup guide as the source of truth before building.

## Install

From the `codex-lite` directory:

```bash
pnpm install
```

Use pnpm for all JavaScript dependency and script commands. Do not use `npm install`; this project is maintained with `pnpm-lock.yaml`.

## Run Locally

Start the native Tauri app:

```bash
pnpm tauri:dev
```

You can also run the Tauri CLI command directly through pnpm:

```bash
pnpm tauri dev
```

For Web UI-only debugging:

```bash
pnpm dev
```

The Vite dev server uses port `1420` with `strictPort: true`.

## Validation

Useful local checks:

```bash
pnpm typecheck
pnpm build
pnpm smoke
```

Rust checks run from the Tauri project:

```bash
cd src-tauri
cargo fmt --check
cargo check
cargo test
```

`pnpm smoke` runs browser-level Playwright tests against mocked Tauri commands. It does not launch the native Tauri shell.

## Local Data

Codex Lite reads and writes the default Codex auth file:

```text
~/.codex/auth.json
```

Codex Lite stores its own local data under the platform app data directory:

```text
macOS:   ~/Library/Application Support/codex-lite
Windows: %APPDATA%\codex-lite
Linux:   $XDG_DATA_HOME/codex-lite or ~/.local/share/codex-lite
```

Important files and folders:

```text
accounts.json
settings.json
backups/
logs/
batch-import-sessions/
```

The Settings page shows the exact paths detected on the current machine.

## Privacy Boundary

Codex Lite is a local desktop app. It does not provide a hosted account service and does not require a remote Codex Lite backend.

Sensitive credential material currently lives in local JSON files. Logs shown in the app are intended to be redacted, but you should still review any file before sharing it. Do not upload real `auth.json`, tokens, API keys, app data, backups, or logs to issues, pull requests, chat tools, or public repositories.

See [Security](./docs/security.md) for details.

## Migration and Recovery

Local app files use a `schemaVersion` field. If local data becomes unreadable, Codex Lite should report a structured storage error instead of silently replacing the file.

See [Migration and Recovery](./docs/migration-and-recovery.md) for backup locations and recovery steps.

## Troubleshooting

See [Troubleshooting](./docs/troubleshooting.md) for dependency install issues, Playwright browser setup, Rust PATH issues, OAuth callback notes, and local data checks.

For checks that require private live credentials or native desktop dialogs, use the [Real Account Smoke Checklist](./docs/real-account-smoke.md).

## Contributing

See [Contributing](../CONTRIBUTING.md) before opening a pull request or issue. Security-sensitive reports should follow the [Security Policy](../SECURITY.md). Codex Lite is distributed under the [MIT License](../LICENSE).

## Current Limits

- OAuth login can receive a local callback automatically when port `1455` is available; the manual callback URL field remains as a fallback.
- Quota refresh still needs broader real-account smoke validation.
- Batch import preview supports selected confirm, but quota checking during preview is not fully implemented.
- Secret store integration is not implemented; credentials are stored in local app data JSON for this stage.
- macOS arm64 packaging has passed locally. Ubuntu 22.04 Docker Linux packaging has passed locally. Windows packaging, GitHub release runs, and native launch/dialog smoke remain unverified.
- Linux support is still planned for the public release stage, with GitHub-hosted artifact validation remaining before release.
