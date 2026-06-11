# Security

Codex Lite handles local Codex credentials. Treat all account files, backups, and logs as sensitive.

## Current Boundary

Codex Lite is a local desktop app. It does not provide a hosted account service and does not require a Codex Lite backend.

Current first-stage credential storage is local JSON:

```text
Codex auth:      ~/.codex/auth.json
Codex Lite data: platform app data directory/codex-lite/accounts.json
Backups:         platform app data directory/codex-lite/backups
Logs:            platform app data directory/codex-lite/logs
```

Local operating system account permissions are the main protection boundary for these files.

## What Not To Share

Do not upload or paste any of the following into GitHub issues, pull requests, chat tools, screenshots, or public logs:

- Real `~/.codex/auth.json`.
- Real `accounts.json`.
- Files from `backups/`.
- Full API keys.
- OAuth access tokens, refresh tokens, or ID tokens.
- Authorization headers.
- OAuth callback URLs or `code` query values.
- Logs that have not been reviewed by you.

If you need to share a snippet, replace sensitive values with placeholders such as:

```text
[REDACTED_TOKEN]
[REDACTED_API_KEY]
[REDACTED_AUTHORIZATION]
[REDACTED_OAUTH_CODE]
```

## Logs and Redaction

Codex Lite includes redaction logic for common token, API key, authorization header, and OAuth callback patterns. Redaction is a safety layer, not a promise that every future credential format will be covered.

Before sharing logs, inspect them manually. If a full token, API key, authorization header, or OAuth code appears in logs, rotate the affected credential and treat it as a release blocker.

## Network Behavior

Codex Lite may contact Codex/OpenAI endpoints when you use OAuth login or quota refresh. It should not send credentials to a Codex Lite hosted service.

Batch import preview and local account management should operate on local files.

## Secret Store Plan

The current first-stage release stores credentials in local JSON files. Before a wider public release, the planned direction is to evaluate OS-backed secret storage:

- macOS Keychain.
- Windows Credential Manager.
- Linux Secret Service compatible keyrings.

Until that is implemented and validated, assume anyone who can read your local user data directory can read Codex Lite stored credentials.

## Secret Store Evaluation

OS-backed secret storage is useful hardening, but it is not required for the first public release if the local JSON boundary stays explicit.

Current evaluation:

- macOS Keychain would reduce exposure from casual file reads, but it adds migration and recovery complexity for accounts already stored in `accounts.json`.
- Windows Credential Manager needs separate validation on a Windows runner or machine before it can be treated as release-ready.
- Linux Secret Service depends on a user session keyring and may not be available in minimal desktop environments.
- Cross-platform secret storage would need a migration path that can read existing JSON credentials, write secrets to the OS store, and recover cleanly when the OS store is locked or unavailable.

Decision for this stage: keep JSON storage documented and tested. Treat OS secret store support as a later hardening task, not as a silent fallback.

## Reporting Security Issues

Please do not open public issues with exploitable details or real credentials. Use a minimal private report path when one is available, or open a public issue with only high-level impact and no secrets.

Useful report details:

- Operating system.
- Codex Lite version or commit.
- Affected feature.
- Whether real credentials may have been exposed.
- Redacted reproduction steps.
