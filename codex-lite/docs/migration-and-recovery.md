# Migration and Recovery

This document describes how Codex Lite stores local data and how to recover from damaged local JSON files.

## Storage Model

Codex Lite stores app-owned data in the platform app data directory:

```text
macOS:   ~/Library/Application Support/codex-lite
Windows: %APPDATA%\codex-lite
Linux:   $XDG_DATA_HOME/codex-lite or ~/.local/share/codex-lite
```

The most important files are:

```text
accounts.json
settings.json
backups/
logs/
batch-import-sessions/
```

Codex itself uses:

```text
~/.codex/auth.json
```

Codex Lite may read and write that file when importing the current local auth or switching the active account.

## schemaVersion

Local app data uses a `schemaVersion` field. The current first-stage schema version is:

```text
1.0.0
```

At this stage, migrations are intentionally conservative. If `accounts.json` or `settings.json` cannot be parsed, Codex Lite should report a structured error instead of silently overwriting the damaged file.

## Backup Locations

Before switching the active Codex account, Codex Lite backs up the previous `~/.codex/auth.json` under the app data `backups/` directory.

Keep both of these locations in mind during recovery:

```text
Codex auth:      ~/.codex/auth.json
Codex Lite data: platform app data directory/codex-lite
```

The Settings page shows the exact detected paths for the current machine.

## Recover accounts.json

If Codex Lite reports `STORAGE_INVALID_FORMAT` for `accounts.json`:

1. Quit Codex Lite.
2. Open the Codex Lite app data directory shown in Settings, or use the platform path listed above.
3. Copy `accounts.json` to a safe private location.
4. Rename the damaged file to `accounts.json.broken`.
5. Start Codex Lite again.
6. Re-import accounts from `~/.codex/auth.json`, a known-good JSON file, OAuth login, token fields, or API key fields.

Do not paste real tokens or API keys into public issues while asking for help. Redact the file before sharing any excerpt.

## Recover settings.json

If Codex Lite reports `SETTINGS_INVALID_FORMAT` for `settings.json`:

1. Quit Codex Lite.
2. Copy `settings.json` to a safe private location.
3. Rename the damaged file to `settings.json.broken`.
4. Start Codex Lite again.
5. Re-apply settings in the Settings page.

Settings are non-credential configuration. Still review the file before sharing it because paths may reveal local usernames or project names.

## Recover ~/.codex/auth.json

If switching accounts produced an unusable Codex auth file:

1. Quit Codex Lite and any Codex process that may read `~/.codex/auth.json`.
2. Open the Codex Lite `backups/` directory.
3. Find the most recent backup created before the failed switch.
4. Copy the backup to `~/.codex/auth.json`.
5. Restart Codex or Codex Lite and verify the active account.

If no backup exists, re-authenticate through Codex or import a known-good auth JSON that you control.

## Public Issue Guidance

When reporting recovery problems, include:

- Operating system and version.
- Codex Lite version.
- The error code and message.
- Whether the affected file was `accounts.json`, `settings.json`, or `~/.codex/auth.json`.
- Redacted logs only.

Never attach real `auth.json`, `accounts.json`, token values, API keys, OAuth callback URLs, or backup files.
