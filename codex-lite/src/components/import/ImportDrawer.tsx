import { CheckCircle2, Copy, ExternalLink, FileJson, FolderOpen, KeyRound, LockKeyhole, TextCursorInput, X } from 'lucide-react';
import type { ReactNode } from 'react';
import { useEffect, useMemo } from 'react';
import { useCodexAccountsStore } from '../../stores/useCodexAccountsStore';
import { type ImportSource, useImportFlowStore } from '../../stores/useImportFlowStore';
import type { CodexAccountView } from '../../types/codex';
import { Button } from '../ui/Button';
import { ErrorBanner } from '../ui/ErrorBanner';
import { IconButton } from '../ui/IconButton';
import { BatchImportPreviewTable } from './BatchImportPreviewTable';
import './ImportDrawer.css';

interface ImportSourceOption {
  id: ImportSource;
  label: string;
  description: string;
  icon: ReactNode;
}

const importSources: ImportSourceOption[] = [
  {
    id: 'local',
    label: 'Current local auth',
    description: 'Use ~/.codex/auth.json',
    icon: <FolderOpen size={15} />,
  },
  {
    id: 'jsonFile',
    label: 'JSON file',
    description: 'Choose one auth JSON file',
    icon: <FileJson size={16} />,
  },
  {
    id: 'jsonText',
    label: 'JSON text',
    description: 'Paste auth JSON content',
    icon: <TextCursorInput size={16} />,
  },
  {
    id: 'token',
    label: 'Token',
    description: 'Paste id/access/refresh token',
    icon: <LockKeyhole size={16} />,
  },
  {
    id: 'apiKey',
    label: 'API Key',
    description: 'Add an API key account',
    icon: <KeyRound size={16} />,
  },
  {
    id: 'oauth',
    label: 'OAuth login',
    description: 'Browser login with callback',
    icon: <LockKeyhole size={16} />,
  },
];

function fileName(filePath: string): string {
  return filePath.split(/[\\/]/).pop() ?? filePath;
}

function jsonTextPreview(jsonText: string): string {
  const trimmed = jsonText.trim();
  if (trimmed.length === 0) {
    return 'No JSON content pasted yet.';
  }

  try {
    const parsed = JSON.parse(trimmed) as Record<string, unknown>;
    const keys = Object.keys(parsed).slice(0, 5);
    return keys.length > 0 ? `Object keys: ${keys.join(', ')}` : 'Valid empty JSON object.';
  } catch {
    return 'JSON will be checked before import.';
  }
}

function accountSubtitle(account: CodexAccountView): string {
  return account.email ?? account.accountId ?? account.userId ?? account.id;
}

function formatExpiresAt(expiresAt: number): string {
  return new Intl.DateTimeFormat(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  }).format(new Date(expiresAt * 1000));
}

export function ImportDrawer() {
  const {
    open,
    closeDrawer,
    source,
    setSource,
    importing,
    selectingFiles,
    previewingBatch,
    jsonText,
    filePaths,
    batchPreview,
    batchSelectedItemIds,
    tokenFields,
    apiKeyFields,
    oauth,
    resultAccounts,
    failedImports,
    error,
    setJsonText,
    setTokenField,
    setApiKeyField,
    setOAuthCallbackUrl,
    startOAuthLogin,
    submitOAuthCallbackUrl,
    pollOAuthLoginStatus,
    cancelOAuthLogin,
    chooseFiles,
    clearFiles,
    toggleBatchItem,
    setAllBatchItemsSelected,
    importSelected,
  } = useImportFlowStore();
  const upsertAccounts = useCodexAccountsStore((state) => state.upsertAccounts);
  const refreshAccountQuota = useCodexAccountsStore((state) => state.refreshAccountQuota);

  const confirmDisabled = useMemo(() => {
    if (importing || selectingFiles || oauth.starting || oauth.submittingCallback || oauth.cancelling) {
      return true;
    }
    if (source === 'jsonFile' || source === 'batchFiles') {
      if (filePaths.length === 0 || previewingBatch) {
        return true;
      }
      return batchPreview !== null && batchSelectedItemIds.length === 0;
    }
    if (source === 'jsonText') {
      return jsonText.trim().length === 0;
    }
    if (source === 'token') {
      return tokenFields.idToken.trim().length === 0 || tokenFields.accessToken.trim().length === 0;
    }
    if (source === 'apiKey') {
      return apiKeyFields.apiKey.trim().length === 0;
    }
    if (source === 'oauth') {
      return oauth.login === null || oauth.step !== 'callbackSubmitted';
    }
    return false;
  }, [
    apiKeyFields.apiKey,
    filePaths.length,
    importing,
    jsonText,
    oauth.cancelling,
    oauth.login,
    oauth.starting,
    oauth.step,
    oauth.submittingCallback,
    previewingBatch,
    selectingFiles,
    source,
    batchPreview,
    batchSelectedItemIds.length,
    tokenFields.accessToken,
    tokenFields.idToken,
  ]);

  function finalizeSuccessfulAdd(accounts: CodexAccountView[]) {
    upsertAccounts(accounts);
    if (open) {
      closeDrawer();
    }
    for (const account of accounts) {
      if (account.authMode === 'oauth') {
        void refreshAccountQuota(account.id);
      }
    }
  }

  async function handleImport() {
    const result = await importSelected();
    if (result) {
      if (result.imported.length > 0 && result.failed.length === 0) {
        finalizeSuccessfulAdd(result.imported);
      } else {
        upsertAccounts(result.imported);
      }
    }
  }

  useEffect(() => {
    if (!open || source !== 'oauth' || !oauth.login || oauth.step !== 'started' || importing) {
      return undefined;
    }

    let cancelled = false;
    async function poll() {
      const result = await pollOAuthLoginStatus();
      if (!cancelled && result && result.imported.length > 0) {
        finalizeSuccessfulAdd(result.imported);
      }
    }

    const interval = window.setInterval(() => {
      void poll();
    }, 1000);
    void poll();

    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, [importing, oauth.login, oauth.step, open, pollOAuthLoginStatus, source]);

  useEffect(() => {
    if (open && source === 'oauth' && oauth.step === 'completed' && resultAccounts.length > 0) {
      finalizeSuccessfulAdd(resultAccounts);
    }
  }, [oauth.step, open, resultAccounts, source]);

  if (!open) {
    return null;
  }

  return (
    <div className="drawer-backdrop" role="presentation">
      <aside className="import-drawer" role="dialog" aria-modal="true" aria-labelledby="import-title">
        <header className="drawer-header">
          <div>
            <h2 id="import-title">Add account</h2>
            <p>Add Codex accounts from local auth, JSON, tokens, or API keys.</p>
          </div>
          <IconButton label="Close add account drawer" icon={<X size={18} />} onClick={closeDrawer} />
        </header>

        <div className="drawer-body">
          {error ? <ErrorBanner error={error} /> : null}

          <section className="drawer-section source-tabs-section">
            <h3>Account type</h3>
            <div className="source-tabs" role="tablist" aria-label="Account source">
              {importSources.map((item) => (
                <button
                  aria-selected={source === item.id}
                  className={`source-tab ${source === item.id ? 'selected' : ''}`}
                  key={item.id}
                  role="tab"
                  type="button"
                  onClick={() => setSource(item.id)}
                >
                  <span className="source-option-icon" aria-hidden="true">
                    {item.icon}
                  </span>
                  <strong>{item.label}</strong>
                </button>
              ))}
            </div>
          </section>

          <section className="drawer-section">
            <div className="drawer-section-title-row">
              <h3>Details</h3>
              <span>{importSources.find((item) => item.id === source)?.description}</span>
            </div>
            {source === 'local' ? (
              <p className="muted">Codex Lite will read your current local auth file and add it to the local account list.</p>
            ) : null}

            {source === 'jsonFile' || source === 'batchFiles' ? (
              <div className="import-field-group">
                <div className="file-actions">
                  <Button variant="secondary" loading={selectingFiles} icon={<FileJson size={16} />} onClick={chooseFiles}>
                    Choose JSON
                  </Button>
                  {filePaths.length > 0 ? (
                    <Button variant="ghost" onClick={clearFiles}>
                      Clear
                    </Button>
                  ) : null}
                </div>
                {filePaths.length === 0 ? (
                  <p className="muted">Choose one or more Codex auth JSON files.</p>
                ) : (
                  <ul className="file-preview-list" aria-label="Selected import files">
                    {filePaths.map((filePath) => (
                      <li key={filePath}>
                        <strong>{fileName(filePath)}</strong>
                        <span>{filePath}</span>
                      </li>
                    ))}
                  </ul>
                )}
                {previewingBatch ? <p className="muted">Preparing batch preview...</p> : null}
              </div>
            ) : null}

            {source === 'jsonText' ? (
              <label className="import-field">
                <span>Auth JSON</span>
                <textarea
                  rows={9}
                  spellCheck={false}
                  value={jsonText}
                  placeholder='{"auth_mode":"oauth","tokens":{...}}'
                  onChange={(event) => setJsonText(event.target.value)}
                />
              </label>
            ) : null}

            {source === 'token' ? (
              <div className="import-field-group">
                <label className="import-field">
                  <span>ID token</span>
                  <textarea
                    rows={3}
                    spellCheck={false}
                    value={tokenFields.idToken}
                    placeholder="Paste id_token"
                    onChange={(event) => setTokenField('idToken', event.target.value)}
                  />
                </label>
                <label className="import-field">
                  <span>Access token</span>
                  <textarea
                    rows={3}
                    spellCheck={false}
                    value={tokenFields.accessToken}
                    placeholder="Paste access_token"
                    onChange={(event) => setTokenField('accessToken', event.target.value)}
                  />
                </label>
                <label className="import-field">
                  <span>Refresh token</span>
                  <input
                    value={tokenFields.refreshToken}
                    placeholder="Optional"
                    onChange={(event) => setTokenField('refreshToken', event.target.value)}
                  />
                </label>
              </div>
            ) : null}

            {source === 'apiKey' ? (
              <div className="import-field-group">
                <label className="import-field">
                  <span>API key</span>
                  <input
                    value={apiKeyFields.apiKey}
                    placeholder="sk-..."
                    onChange={(event) => setApiKeyField('apiKey', event.target.value)}
                  />
                </label>
                <label className="import-field">
                  <span>Display name</span>
                  <input
                    value={apiKeyFields.displayName}
                    placeholder="Optional"
                    onChange={(event) => setApiKeyField('displayName', event.target.value)}
                  />
                </label>
                <label className="import-field">
                  <span>API base URL</span>
                  <input
                    value={apiKeyFields.apiBaseUrl}
                    placeholder="Optional, e.g. https://api.openai.com/v1"
                    onChange={(event) => setApiKeyField('apiBaseUrl', event.target.value)}
                  />
                </label>
              </div>
            ) : null}

            {source === 'oauth' ? (
              <OAuthLoginPanel
                callbackUrl={oauth.callbackUrl}
                cancelling={oauth.cancelling}
                login={oauth.login}
                portInUse={oauth.portInUse}
                starting={oauth.starting}
                step={oauth.step}
                submittingCallback={oauth.submittingCallback}
                onCancel={cancelOAuthLogin}
                onCallbackUrlChange={setOAuthCallbackUrl}
                onStart={startOAuthLogin}
                onSubmitCallback={submitOAuthCallbackUrl}
              />
            ) : null}
          </section>

          <section className="drawer-section">
            <h3>Preview</h3>
            {resultAccounts.length === 0 && failedImports.length === 0 ? (
              <PreviewHint source={source} jsonText={jsonText} filePaths={filePaths} />
            ) : null}
            {resultAccounts.length > 0 || failedImports.length > 0 ? (
              <p className="import-summary">
                Added {resultAccounts.length}, failed {failedImports.length}
              </p>
            ) : null}
            {batchPreview ? (
              <BatchImportPreviewTable
                items={batchPreview.items}
                selectedItemIds={batchSelectedItemIds}
                onToggleAll={setAllBatchItemsSelected}
                onToggleItem={toggleBatchItem}
              />
            ) : null}
            {resultAccounts.length > 0 ? (
              <ul className="import-result-list" aria-label="Added accounts">
                {resultAccounts.map((account) => (
                  <li key={account.id}>
                    <CheckCircle2 size={16} aria-hidden="true" />
                    <span>
                      <strong>{account.displayName}</strong>
                      <small>{accountSubtitle(account)}</small>
                    </span>
                  </li>
                ))}
              </ul>
            ) : null}
            {failedImports.length > 0 ? (
              <ul className="import-failure-list" aria-label="Failed imports">
                {failedImports.map((failure) => (
                  <li key={`${failure.source}-${failure.error}`}>
                    <strong>{fileName(failure.source)}</strong>
                    <span>{failure.error}</span>
                  </li>
                ))}
              </ul>
            ) : null}
          </section>
        </div>

        <footer className="drawer-footer">
          <Button variant="ghost" onClick={closeDrawer}>
            Cancel
          </Button>
          <Button variant="primary" loading={importing} disabled={confirmDisabled} onClick={handleImport}>
            Add account
          </Button>
        </footer>
      </aside>
    </div>
  );
}

interface PreviewHintProps {
  source: ImportSource;
  jsonText: string;
  filePaths: string[];
}

function PreviewHint({ source, jsonText, filePaths }: PreviewHintProps) {
  if (source === 'jsonText') {
    return <p className="muted">{jsonTextPreview(jsonText)}</p>;
  }

  if (source === 'jsonFile' || source === 'batchFiles') {
    return <p className="muted">{filePaths.length > 0 ? `${filePaths.length} file(s) ready to import.` : 'No file selected yet.'}</p>;
  }

  if (source === 'token') {
    return <p className="muted">Tokens are validated locally before the account is stored.</p>;
  }

  if (source === 'apiKey') {
    return <p className="muted">The API key is stored as a Codex API key account after confirmation.</p>;
  }

  if (source === 'oauth') {
    return <p className="muted">Start login, finish authorization in your browser, and Codex Lite will add the account automatically.</p>;
  }

  return <p className="muted">Ready to add your current local Codex auth.</p>;
}

interface OAuthLoginPanelProps {
  callbackUrl: string;
  cancelling: boolean;
  login: {
    loginId: string;
    authUrl: string;
    redirectUri: string;
    expiresAt: number;
    listenerStarted: boolean;
    listenerError?: string | null;
  } | null;
  portInUse: boolean | null;
  starting: boolean;
  step: string;
  submittingCallback: boolean;
  onCancel: () => Promise<void>;
  onCallbackUrlChange: (callbackUrl: string) => void;
  onStart: () => Promise<void>;
  onSubmitCallback: () => Promise<void>;
}

function OAuthLoginPanel({
  callbackUrl,
  cancelling,
  login,
  portInUse,
  starting,
  step,
  submittingCallback,
  onCancel,
  onCallbackUrlChange,
  onStart,
  onSubmitCallback,
}: OAuthLoginPanelProps) {
  const canSubmitCallback = login !== null && callbackUrl.trim().length > 0 && step !== 'callbackSubmitted' && step !== 'completed';

  async function copyAuthUrl() {
    if (!login) {
      return;
    }
    await navigator.clipboard.writeText(login.authUrl);
  }

  return (
    <div className="oauth-panel">
      <div className="oauth-actions">
        <Button variant="secondary" loading={starting} icon={<LockKeyhole size={16} />} onClick={onStart}>
          Start Login
        </Button>
        {login ? (
          <Button variant="ghost" loading={cancelling} onClick={onCancel}>
            Cancel Login
          </Button>
        ) : null}
      </div>

      {step === 'cancelled' ? <p className="muted">OAuth login was cancelled. Start again when ready.</p> : null}
      {step === 'expired' ? <p className="oauth-warning">OAuth login expired. Start a new login.</p> : null}

      {login ? (
        <div className="oauth-session">
          <dl className="oauth-meta">
            <div>
              <dt>Redirect URI</dt>
              <dd>{login.redirectUri}</dd>
            </div>
            <div>
              <dt>Login ID</dt>
              <dd>{login.loginId}</dd>
            </div>
            <div>
              <dt>Expires</dt>
              <dd>{formatExpiresAt(login.expiresAt)}</dd>
            </div>
          </dl>

          {portInUse ? (
            <p className="oauth-warning">
              Automatic callback listener is unavailable. Paste the browser callback URL manually to continue.
              {login.listenerError ? <span>{login.listenerError}</span> : null}
            </p>
          ) : (
            <p className="oauth-ready">Automatic callback listener is running. Finish browser authorization and return here.</p>
          )}

          <div className="oauth-url-row">
            <a className="oauth-auth-link" href={login.authUrl} target="_blank" rel="noreferrer">
              <ExternalLink size={15} />
              Open auth URL
            </a>
            <Button variant="ghost" icon={<Copy size={15} />} onClick={copyAuthUrl}>
              Copy URL
            </Button>
          </div>

          <label className="import-field">
            <span>Callback URL</span>
            <textarea
              rows={4}
              spellCheck={false}
              value={callbackUrl}
              placeholder="http://localhost:1455/auth/callback?code=...&state=..."
              onChange={(event) => onCallbackUrlChange(event.target.value)}
            />
          </label>

          <div className="oauth-actions">
            <Button
              variant="secondary"
              loading={submittingCallback}
              disabled={!canSubmitCallback || submittingCallback}
              onClick={onSubmitCallback}
            >
              Submit Callback
            </Button>
            {step === 'callbackSubmitted' ? <span className="oauth-ready">Callback received. Adding account...</span> : null}
          </div>
        </div>
      ) : (
        <p className="muted">Start login and finish authorization in your browser. If automatic callback is unavailable, paste the full callback URL.</p>
      )}
    </div>
  );
}
