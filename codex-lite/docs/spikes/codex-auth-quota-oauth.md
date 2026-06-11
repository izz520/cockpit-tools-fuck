# Codex Auth, Quota, and OAuth Spike

## Current shape

- OAuth auth files are parsed from `tokens.idToken`, `tokens.accessToken`, optional `tokens.refreshToken`, and optional `tokens.accountId`.
- API key auth files are parsed from `OPENAI_API_KEY` plus optional `baseUrl`.
- OAuth account identity is derived from the ID token JWT payload, preferring `sub`, then `accountId`, then `email`.
- Quota refresh calls `https://chatgpt.com/backend-api/wham/usage` with an OAuth bearer access token and optional `ChatGPT-Account-Id`.
- OAuth login currently uses PKCE and a manual callback URL paste flow. It does not run a local callback HTTP listener yet.

## Fixtures

- `fixtures/redacted-auth/oauth.json` contains a synthetic unsigned JWT with fixture-only email and subject values.
- `fixtures/redacted-auth/api-key.json` contains a non-real API key shaped like a key for parser coverage only.
- `fixtures/redacted-auth/invalid-empty.json` contains no usable credentials and must fail account projection.

These fixtures must never be replaced with real tokens, refresh tokens, authorization codes, or API keys.

## Tests added in T-019

- Redaction covers token fields, API key fields, Authorization headers, and OAuth callback `code` query values.
- Auth parser covers OAuth fixture projection, API key fixture projection, and invalid credential shape.
- Quota tests cover response parsing and HTTP error classification without network calls.
- OAuth tests cover auth URL construction, query decoding, callback extraction, and missing-code errors without network calls.

## Not covered yet

- Redaction gate currently fails for two cases:
  - `Authorization: Bearer <token>` leaves the bearer token visible because the current marker-based scanner stops after redacting `Bearer`.
  - A string containing both `OPENAI_API_KEY` and a later `api_key` leaves the later value visible because the scanner redacts only the first match per marker.
- Switch service is not covered by an isolated unit test because the current implementation resolves the real default `~/.codex/auth.json` path through `dirs::home_dir()` and writes through the production storage path. A reliable switch test should first introduce injectable path/storage boundaries or an integration harness that pins app data and home directories to a temporary sandbox.
- Quota is not smoke-tested against a real account because this workspace does not include an authorized redacted live account and the quota endpoint can reveal account-specific state. Current tests verify parsing and error classification only.
- OAuth token exchange is not smoke-tested because it requires a live browser authorization flow and a short-lived authorization code. Current tests verify local URL/callback handling only.

## Follow-up validation steps

1. Import a redacted real OAuth account on a local machine that owns the account.
2. Run `cargo test auth_file_service` to confirm fixture parsing still passes.
3. Run the app and complete manual OAuth callback login with a disposable account.
4. Refresh quota for that OAuth account and record only status class, response shape, and redacted error code/body snippets.
5. Add a temporary-path integration harness before testing switch backup/rollback behavior.
6. Re-run `cargo test` and `pnpm typecheck` before public release.
