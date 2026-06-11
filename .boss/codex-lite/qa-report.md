# Codex Lite QA Report

## Scope

This report covers the QA/DevOps tasks for Codex Lite:

- T-020 UI smoke and visual overflow checks.
- T-021 CI and release workflow drafts.
- T-022 first-stage acceptance reporting.

## Files Reviewed or Added

- `codex-lite/playwright.config.ts`
- `codex-lite/tests/smoke/accounts.spec.ts`
- `codex-lite/tests/smoke/import.spec.ts`
- `codex-lite/tests/smoke/settings-logs.spec.ts`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `codex-lite/docs/troubleshooting.md`
- `codex-lite/docs/migration-and-recovery.md`
- `codex-lite/docs/security.md`
- `codex-lite/docs/real-account-smoke.md`

## Smoke Coverage

The Playwright smoke suite is configured for Vite Web UI validation, not native Tauri automation.

Covered paths:

- Accounts empty state.
- Import drawer opening from the empty state.
- Account list and account detail rendering with long email data.
- OAuth and API Key capability states.
- API Key import drawer path.
- Manual OAuth callback UI path.
- Settings path snapshot.
- Logs snapshot with redacted message text.
- Current account badge.
- Keyboard tab access to primary navigation and account actions.
- Import source-specific forms.
- Partial import success/failure summary.
- Horizontal overflow guard at `1180x760`, `900x620`, and `720x760`.

Not covered yet:

- Native Tauri window launch.
- Native file dialog behavior.
- Real `~/.codex/auth.json` import.
- Real OAuth token exchange.
- Real automatic OAuth localhost callback.
- Real quota endpoint smoke.
- Native switch file-write behavior from a real Tauri shell.

## CI and Release Workflows

CI workflow draft:

- `pnpm install --frozen-lockfile`
- `pnpm typecheck`
- `cargo check`
- `cargo test`

Release workflow draft:

- macOS universal Tauri build.
- Windows Tauri build.
- Linux Tauri build on Ubuntu 22.04 with common Tauri system dependencies.
- Manual release trigger requires an explicit release tag input instead of using the current branch name.
- Draft GitHub release through `tauri-apps/tauri-action`.

GitHub-hosted release runs remain to be verified in Actions.

## Local Validation Results

Commands run on 2026-06-11:

```bash
pnpm typecheck
```

Result: passed.

```bash
PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/opt/rustup/bin:$PATH" cargo check
```

Result: passed.

```bash
pnpm smoke
```

Result: passed, 24/24 tests.

```bash
pnpm smoke:ci
```

Result: passed, 24/24 tests.

```bash
PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:/opt/homebrew/opt/rustup/bin:$PATH" cargo test
```

Result: passed, 40/40 tests.

```bash
pnpm tauri:build
```

Result: passed on macOS arm64. Produced:

- `codex-lite/src-tauri/target/release/bundle/macos/Codex Lite.app`
- `codex-lite/src-tauri/target/release/bundle/dmg/Codex Lite_0.1.0_aarch64.dmg`

```bash
docker run --rm -e CI=true -v "$PWD:/workspace" -w /workspace/codex-lite ubuntu:22.04 bash -lc 'apt-get update && apt-get install -y build-essential curl file libayatana-appindicator3-dev libgtk-3-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev librsvg2-dev libssl-dev libwebkit2gtk-4.1-dev libxdo-dev patchelf pkg-config wget rpm xdg-utils && curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && apt-get install -y nodejs && corepack enable && corepack prepare pnpm@10.25.0 --activate && curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal && . "$HOME/.cargo/env" && pnpm install --frozen-lockfile && pnpm tauri build'
```

Result: passed on Ubuntu 22.04 Docker arm64. Produced:

- `codex-lite/src-tauri/target/release/bundle/deb/Codex Lite_0.1.0_arm64.deb`
- `codex-lite/src-tauri/target/release/bundle/rpm/Codex Lite-0.1.0-1.aarch64.rpm`
- `codex-lite/src-tauri/target/release/bundle/appimage/Codex Lite_0.1.0_aarch64.AppImage`

## Acceptance Status

| Area | Status | Evidence |
| --- | --- | --- |
| Frontend typecheck | Passed | `pnpm typecheck` |
| Rust check | Passed with warnings | `cargo check` |
| Rust tests | Passed | `cargo test`, 40/40 |
| UI smoke config | Passed | `pnpm smoke`, 24/24 |
| macOS native package | Passed | `pnpm tauri:build` |
| Linux native package | Passed locally | Ubuntu 22.04 Docker `pnpm tauri build` |
| CI workflow | Draft added, local equivalent passed | `.github/workflows/ci.yml` |
| Release workflow | Draft added, macOS and Linux local builds passed | `.github/workflows/release.yml` |
| Troubleshooting docs | Added | `codex-lite/docs/troubleshooting.md` |

## Release Blockers

- Windows packaging and GitHub-hosted release artifacts still need GitHub Actions validation.
- Real account smoke is still required for import, switch, quota refresh, automatic OAuth callback, manual OAuth fallback, and live quota refresh.
- Native Tauri app launch was observed locally, but file dialog/opener behavior still needs a real desktop click-through check.
- Secret store is not integrated. The current public boundary is documented in `codex-lite/docs/security.md` and `SECURITY.md`; OS secret store support should be evaluated as a later hardening task.
