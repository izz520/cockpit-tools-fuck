# Real Account Smoke Checklist

Use this checklist only on a private machine with Codex credentials you control. Do not commit, upload, or paste real tokens, API keys, auth files, app data, backups, screenshots with secrets, or logs that have not been reviewed.

## Scope

This smoke pass verifies behavior that mocked Web UI tests cannot prove:

- Import from the current local `~/.codex/auth.json`.
- Import from a copied auth JSON file.
- Switch active account and restore from backup if needed.
- Refresh quota against the live quota endpoint.
- Complete OAuth token exchange through the manual callback fallback.
- Complete OAuth token exchange through the automatic localhost callback when port `1455` is available.
- Confirm dialog and opener capabilities in the native Tauri shell.

## Before You Start

1. Back up the current Codex auth file:

   ```bash
   cp ~/.codex/auth.json ~/.codex/auth.json.before-codex-lite-smoke
   ```

2. Start the native app:

   ```bash
   pnpm tauri:dev
   ```

3. Keep `codex-lite/docs/security.md` open and treat every local app data file as sensitive.

## Native Capability Checks

- Open Settings.
- Click `Open data directory`.
- Click `Open logs`.
- Open the import drawer.
- Choose JSON file import and confirm that the native file dialog opens.

Expected result: each native opener/dialog action works without a Tauri permission error.

## Import Checks

- Import current local auth.
- Copy `~/.codex/auth.json` to a private temporary path and import it as a JSON file.
- Confirm duplicate imports are shown as existing in batch preview.
- Import one synthetic API key account using a placeholder value you are comfortable deleting afterward.

Expected result: valid accounts are added once, duplicates are not selected by default, and failed files do not block successful files.

## Switch Checks

- Select a non-current imported account.
- Confirm the switch modal.
- Verify `~/.codex/auth.json` changed.
- Verify a backup file was created under the Codex Lite `backups/` directory.

Expected result: switching succeeds only after confirmation and leaves a recoverable backup.

## Quota Checks

- Select an OAuth account.
- Click `Refresh quota`.
- Record whether hourly and weekly quota values update.
- Temporarily disconnect the network or use an expired account if available, then refresh again.

Expected result: successful refresh stores quota. Failed refresh keeps the previous value and marks it stale with an actionable error.

## OAuth Checks

- Select `OAuth login` in the import drawer.
- Start login.
- Complete authorization in the browser.
- If the automatic callback listener is running, return to Codex Lite after browser authorization.
- If automatic callback is unavailable, paste the full callback URL and submit callback.
- Confirm import.

Expected result: a new OAuth account is stored without exposing tokens in the UI or logs. Manual callback remains available when the local listener cannot bind port `1455`.

## After You Finish

1. Restore the original auth file if needed:

   ```bash
   cp ~/.codex/auth.json.before-codex-lite-smoke ~/.codex/auth.json
   ```

2. Review logs before sharing any result.
3. Report only pass/fail, OS version, app commit/version, and redacted error codes/messages.

Never attach real `auth.json`, `accounts.json`, backup files, OAuth callback URLs, tokens, API keys, or raw logs to a public issue.
