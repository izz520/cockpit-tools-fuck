# Contributing

Thanks for considering a contribution. Codex Lite is intended to stay small, local-first, ad-free, and easy to inspect.

## Scope

This repository currently contains the `codex-lite` desktop app. Please keep changes focused on that project unless a maintainer asks for repository-wide work.

Good contributions include:

- Bug fixes with clear reproduction steps.
- Small UX improvements that keep the app lightweight.
- Tests or smoke coverage for existing behavior.
- Documentation that helps users run, recover, or secure local data.
- Platform validation notes for macOS, Windows, and Linux.

Please avoid:

- Ads, telemetry, sponsor modules, or hosted account services.
- Uploading real tokens, API keys, logs, or auth files.
- Large rewrites without prior discussion.
- New dependencies without a clear reason.

## Development

Use pnpm for JavaScript commands:

```bash
cd codex-lite
pnpm install
pnpm typecheck
pnpm smoke
```

Rust checks:

```bash
cd codex-lite/src-tauri
cargo fmt --check
cargo check
cargo test
```

Native dev app:

```bash
cd codex-lite
pnpm tauri:dev
```

## Pull Requests

Before opening a pull request:

- Describe the user-facing change.
- List validation commands and results.
- Mention any skipped validation.
- Confirm no real credentials or private logs are included.
- Keep the diff scoped to the issue being solved.

## Security

Read `codex-lite/docs/security.md` before sharing logs or fixtures. If you believe a credential exposure or security issue exists, do not post real secrets in a public issue.
