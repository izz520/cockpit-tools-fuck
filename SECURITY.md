# Security Policy

Codex Lite handles local Codex credentials. Treat account files, app data,
backups, and logs as sensitive.

## Supported Versions

Codex Lite is still in early local testing. Until the first public release,
security fixes are handled on the main development line.

## Reporting a Vulnerability

Do not open a public issue that contains real credentials, OAuth callback URLs,
tokens, API keys, raw `auth.json`, raw `accounts.json`, backup files, or
unreviewed logs.

If a private report path is available, use it. Otherwise, open a public issue
with only:

- Operating system.
- Codex Lite version or commit.
- Affected feature.
- High-level impact.
- Redacted reproduction steps.

For the current local credential boundary and what not to share, see
[`codex-lite/docs/security.md`](./codex-lite/docs/security.md).
