# Codex Lite Troubleshooting

Use this guide for local development and small-circle testing. Public release packaging is still being validated.

## JavaScript Dependencies

Use `pnpm` for all JavaScript commands.

```bash
pnpm install
```

If CI or local install fails with a frozen lockfile error after smoke test changes, update the lockfile with:

```bash
pnpm install
```

Do not use `npm install`; it will create the wrong lockfile for this project.

## Playwright Smoke Tests

Smoke tests run against the Vite web UI and mock Tauri commands in the browser. They do not launch the native Tauri shell.

```bash
pnpm smoke
```

If the command fails with `playwright: command not found`, install dependencies first:

```bash
pnpm install
```

If browsers are missing, install the Chromium browser used by Playwright:

```bash
pnpm exec playwright install chromium
```

## Rust Toolchain

If `cargo` is not found in the current shell, use the installed stable toolchain path:

```bash
PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/opt/rustup/bin:$PATH" cargo check
```

Run Rust validation from the Tauri project:

```bash
cd src-tauri
cargo check
cargo test
```

## Tauri Dev Server

Start the native dev app with:

```bash
pnpm tauri:dev
```

For Web UI-only debugging, run:

```bash
pnpm dev
```

The Vite server uses port `1420` with `strictPort: true`. If the port is already in use, stop the other process before starting Codex Lite.

## Platform Dependencies

If native build or dev startup fails before the app window opens, check the Tauri platform dependencies first.

macOS:

```bash
xcode-select --install
```

Windows:

- Install Microsoft Visual Studio Build Tools with the C++ desktop workload.
- Install or repair the WebView2 runtime.
- Use the Rust MSVC toolchain.

Linux:

- Ubuntu 22.04 Docker packaging has passed locally for `.deb`, `.rpm`, and `.AppImage`.
- Install the Tauri 2 WebKitGTK, GTK, appindicator, librsvg, xdg-utils, rpm, and build tooling packages for your distribution.

## OAuth Callback

Codex Lite tries to receive the OAuth browser callback on `127.0.0.1:1455`. If the port is unavailable or the browser does not return to the app, use the manual callback field:

1. Open the import drawer.
2. Select `OAuth login`.
3. Start login and open or copy the auth URL.
4. Finish browser authorization.
5. If automatic callback succeeds, return to Codex Lite and confirm import.
6. If automatic callback is unavailable, paste the full callback URL into Codex Lite.
7. Submit the callback and confirm import.

If port `1455` is already in use, the manual callback flow can still continue by pasting the browser callback URL.

## Local Data and Logs

Codex Lite stores local app data under the platform app data directory and reads Codex auth from `~/.codex/auth.json` by default. The Settings page shows the exact paths detected on the current machine.

Logs are expected to be redacted before display. If a full token, API key, authorization header, or OAuth code appears in logs, treat it as a release blocker and rotate the affected credential.

If `accounts.json` or `settings.json` becomes invalid, see [Migration and Recovery](./migration-and-recovery.md). Do not delete files until you have made a private backup.

## Known First-Stage Gaps

- Playwright smoke is Web UI-level only and does not verify native Tauri window behavior.
- OAuth and quota still need real redacted-account smoke validation.
- GitHub-hosted Linux release artifact validation is deferred to the public GitHub phase.
- Secret store integration is not part of the first small-circle release.
