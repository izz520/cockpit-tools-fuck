import { create } from 'zustand';
import {
  addCodexAccountWithApiKey,
  addCodexAccountWithToken,
  chooseCodexAuthFiles,
  cancelCodexOAuthLogin,
  confirmCodexBatchImport,
  completeCodexOAuthLogin,
  getCodexOAuthLoginStatus,
  importCodexFromFiles,
  importCodexFromJson,
  importCodexFromLocal,
  startCodexBatchImportFromFiles,
  startCodexOAuthLogin,
  submitCodexOAuthCallbackUrl,
} from '../services/codexImportService';
import type { CodexAccountView } from '../types/codex';
import type { BatchImportSession, ImportResult, OAuthStartResult } from '../types/import';
import type { AppError } from '../types/system';
import { normalizeInvokeError } from '../services/tauriInvoke';

export type ImportSource = 'local' | 'jsonFile' | 'jsonText' | 'batchFiles' | 'oauth' | 'token' | 'apiKey';

interface TokenFields {
  idToken: string;
  accessToken: string;
  refreshToken: string;
}

interface ApiKeyFields {
  apiKey: string;
  apiBaseUrl: string;
  displayName: string;
}

export type OAuthFlowStep = 'idle' | 'started' | 'callbackSubmitted' | 'completed' | 'cancelled' | 'expired';

interface OAuthFlowState {
  step: OAuthFlowStep;
  login: OAuthStartResult | null;
  callbackUrl: string;
  portInUse: boolean | null;
  starting: boolean;
  submittingCallback: boolean;
  cancelling: boolean;
}

interface ImportFlowState {
  open: boolean;
  source: ImportSource;
  importing: boolean;
  selectingFiles: boolean;
  jsonText: string;
  filePaths: string[];
  batchPreview: BatchImportSession | null;
  batchSelectedItemIds: string[];
  previewingBatch: boolean;
  tokenFields: TokenFields;
  apiKeyFields: ApiKeyFields;
  oauth: OAuthFlowState;
  resultAccounts: CodexAccountView[];
  failedImports: ImportResult['failed'];
  error: AppError | null;
  openDrawer: () => void;
  openOAuthLogin: () => Promise<void>;
  closeDrawer: () => void;
  setSource: (source: ImportSource) => void;
  setJsonText: (jsonText: string) => void;
  setTokenField: (field: keyof TokenFields, value: string) => void;
  setApiKeyField: (field: keyof ApiKeyFields, value: string) => void;
  setOAuthCallbackUrl: (callbackUrl: string) => void;
  startOAuthLogin: () => Promise<void>;
  submitOAuthCallbackUrl: () => Promise<void>;
  pollOAuthLoginStatus: () => Promise<ImportResult | null>;
  cancelOAuthLogin: () => Promise<void>;
  chooseFiles: () => Promise<void>;
  clearFiles: () => void;
  toggleBatchItem: (itemId: string) => void;
  setAllBatchItemsSelected: (selected: boolean) => void;
  importSelected: () => Promise<ImportResult | null>;
}

const emptyTokenFields: TokenFields = {
  idToken: '',
  accessToken: '',
  refreshToken: '',
};

const emptyApiKeyFields: ApiKeyFields = {
  apiKey: '',
  apiBaseUrl: '',
  displayName: '',
};

const emptyOAuthFlow: OAuthFlowState = {
  step: 'idle',
  login: null,
  callbackUrl: '',
  portInUse: null,
  starting: false,
  submittingCallback: false,
  cancelling: false,
};

function appError(code: string, message: string, action: string): AppError {
  return {
    code,
    message,
    action,
    retryable: false,
  };
}

function singleAccountResult(account: CodexAccountView): ImportResult {
  return {
    imported: [account],
    skipped: [],
    failed: [],
  };
}

function validateJsonText(jsonText: string): AppError | null {
  const trimmed = jsonText.trim();
  if (trimmed.length === 0) {
    return appError('IMPORT_JSON_EMPTY', 'JSON content is empty.', 'Paste a Codex auth JSON payload.');
  }

  try {
    const parsed = JSON.parse(trimmed) as unknown;
    if (typeof parsed !== 'object' || parsed === null) {
      return appError(
        'IMPORT_JSON_VALUE_REQUIRED',
        'JSON content must be an object or array.',
        'Paste a Codex auth JSON object, CPA export, sub2api export, or an array of exported accounts.',
      );
    }
  } catch (error) {
    return appError(
      'IMPORT_JSON_INVALID',
      error instanceof Error ? error.message : 'JSON content is not valid.',
      'Fix the JSON syntax before importing.',
    );
  }

  return null;
}

function validateTokenFields(fields: TokenFields): AppError | null {
  if (fields.idToken.trim().length === 0) {
    return appError('IMPORT_ID_TOKEN_EMPTY', 'ID token is required.', 'Paste the id_token value from a Codex auth file.');
  }

  if (fields.accessToken.trim().length === 0) {
    return appError(
      'IMPORT_ACCESS_TOKEN_EMPTY',
      'Access token is required.',
      'Paste the access_token value from a Codex auth file.',
    );
  }

  return null;
}

function validateApiKeyFields(fields: ApiKeyFields): AppError | null {
  if (fields.apiKey.trim().length === 0) {
    return appError('IMPORT_API_KEY_EMPTY', 'API key is required.', 'Paste a valid API key.');
  }

  if (fields.apiBaseUrl.trim().length > 0) {
    try {
      new URL(fields.apiBaseUrl.trim());
    } catch {
      return appError('IMPORT_API_BASE_URL_INVALID', 'API base URL is invalid.', 'Use a full URL such as https://api.openai.com/v1.');
    }
  }

  return null;
}

function isOAuthExpired(login: OAuthStartResult): boolean {
  return login.expiresAt <= Math.floor(Date.now() / 1000);
}

function oauthRequiredError(): AppError {
  return appError('IMPORT_OAUTH_NOT_STARTED', 'OAuth login has not started.', 'Start OAuth login before importing.');
}

function defaultBatchSelection(session: BatchImportSession): string[] {
  return session.items.filter((item) => item.selected && item.selectable && item.status === 'importable').map((item) => item.id);
}

function shouldCloseAfterSuccessfulImport(result: ImportResult): boolean {
  return result.imported.length > 0 && result.failed.length === 0;
}

function emptyImportDraft() {
  return {
    jsonText: '',
    filePaths: [],
    batchPreview: null,
    batchSelectedItemIds: [],
    tokenFields: emptyTokenFields,
    apiKeyFields: emptyApiKeyFields,
  };
}

export const useImportFlowStore = create<ImportFlowState>((set) => ({
  open: false,
  source: 'local',
  importing: false,
  selectingFiles: false,
  jsonText: '',
  filePaths: [],
  batchPreview: null,
  batchSelectedItemIds: [],
  previewingBatch: false,
  tokenFields: emptyTokenFields,
  apiKeyFields: emptyApiKeyFields,
  oauth: emptyOAuthFlow,
  resultAccounts: [],
  failedImports: [],
  error: null,
  openDrawer() {
    set({
      open: true,
      error: null,
      resultAccounts: [],
      failedImports: [],
      ...emptyImportDraft(),
    });
  },
  async openOAuthLogin() {
    const { oauth } = useImportFlowStore.getState();
    if (oauth.login && oauth.step !== 'completed') {
      void cancelCodexOAuthLogin(oauth.login.loginId);
    }
    set({
      open: true,
      source: 'oauth',
      importing: false,
      selectingFiles: false,
      previewingBatch: false,
      error: null,
      resultAccounts: [],
      failedImports: [],
      ...emptyImportDraft(),
      oauth: emptyOAuthFlow,
    });
    await useImportFlowStore.getState().startOAuthLogin();
  },
  closeDrawer() {
    const { oauth } = useImportFlowStore.getState();
    if (oauth.login && oauth.step !== 'completed') {
      void cancelCodexOAuthLogin(oauth.login.loginId);
    }
    set({
      open: false,
      importing: false,
      previewingBatch: false,
      error: null,
      oauth: emptyOAuthFlow,
      ...emptyImportDraft(),
    });
  },
  setSource(source) {
    const { oauth } = useImportFlowStore.getState();
    if (source !== 'oauth' && oauth.login && oauth.step !== 'completed') {
      void cancelCodexOAuthLogin(oauth.login.loginId);
    }
    set({
      source,
      error: null,
      resultAccounts: [],
      failedImports: [],
      batchPreview: null,
      batchSelectedItemIds: [],
      filePaths: source === 'jsonFile' || source === 'batchFiles' ? useImportFlowStore.getState().filePaths : [],
      oauth: source === 'oauth' ? oauth : emptyOAuthFlow,
    });
  },
  setJsonText(jsonText) {
    set({ jsonText, error: null, resultAccounts: [], failedImports: [], batchPreview: null, batchSelectedItemIds: [] });
  },
  setTokenField(field, value) {
    set((state) => ({
      tokenFields: { ...state.tokenFields, [field]: value },
      error: null,
      resultAccounts: [],
      failedImports: [],
      batchPreview: null,
      batchSelectedItemIds: [],
    }));
  },
  setApiKeyField(field, value) {
    set((state) => ({
      apiKeyFields: { ...state.apiKeyFields, [field]: value },
      error: null,
      resultAccounts: [],
      failedImports: [],
      batchPreview: null,
      batchSelectedItemIds: [],
    }));
  },
  setOAuthCallbackUrl(callbackUrl) {
    set((state) => ({
      oauth: { ...state.oauth, callbackUrl },
      error: null,
      resultAccounts: [],
      failedImports: [],
    }));
  },
  async startOAuthLogin() {
    const previousLogin = useImportFlowStore.getState().oauth.login;
    set((state) => ({
      oauth: { ...state.oauth, starting: true },
      error: null,
      resultAccounts: [],
      failedImports: [],
    }));
    try {
      if (previousLogin) {
        await cancelCodexOAuthLogin(previousLogin.loginId);
      }
      const login = await startCodexOAuthLogin();
      set({
        oauth: {
          ...emptyOAuthFlow,
          step: 'started',
          login,
          portInUse: !login.listenerStarted,
        },
        error: null,
      });
    } catch (error) {
      set((state) => ({
        oauth: { ...state.oauth, starting: false },
        error: normalizeInvokeError(error),
      }));
    }
  },
  async submitOAuthCallbackUrl() {
    const { oauth } = useImportFlowStore.getState();
    if (!oauth.login) {
      set({ error: oauthRequiredError() });
      return;
    }
    if (isOAuthExpired(oauth.login)) {
      set((state) => ({
        oauth: { ...state.oauth, step: 'expired' },
        error: appError('IMPORT_OAUTH_EXPIRED', 'OAuth login has expired.', 'Start OAuth login again.'),
      }));
      return;
    }
    if (oauth.callbackUrl.trim().length === 0) {
      set({
        error: appError('IMPORT_OAUTH_CALLBACK_EMPTY', 'Callback URL is required.', 'Paste the full callback URL from the browser.'),
      });
      return;
    }

    set((state) => ({
      oauth: { ...state.oauth, submittingCallback: true },
      error: null,
      resultAccounts: [],
      failedImports: [],
    }));
    try {
      await submitCodexOAuthCallbackUrl(oauth.login.loginId, oauth.callbackUrl.trim());
      set((state) => ({ oauth: { ...state.oauth, step: 'callbackSubmitted', submittingCallback: false } }));
      await useImportFlowStore.getState().pollOAuthLoginStatus();
    } catch (error) {
      set((state) => ({
        oauth: { ...state.oauth, submittingCallback: false },
        error: normalizeInvokeError(error),
      }));
    }
  },
  async pollOAuthLoginStatus() {
    const { oauth } = useImportFlowStore.getState();
    if (!oauth.login || oauth.step === 'completed' || oauth.step === 'expired') {
      return null;
    }
    try {
      const status = await getCodexOAuthLoginStatus(oauth.login.loginId);
      if (status.step === 'expired') {
        set((state) => ({
          oauth: { ...state.oauth, step: 'expired' },
          error: appError('IMPORT_OAUTH_EXPIRED', 'OAuth login has expired.', 'Start OAuth login again.'),
        }));
        return null;
      }
      if (status.step === 'missing') {
        return null;
      }
      if (status.step === 'started') {
        return null;
      }

      set((state) => ({
        importing: true,
        oauth: { ...state.oauth, step: 'callbackSubmitted' },
        error: null,
        resultAccounts: [],
        failedImports: [],
      }));
      const result = singleAccountResult(await completeCodexOAuthLogin(oauth.login.loginId));
      set((state) => ({
        importing: false,
        oauth: { ...state.oauth, step: 'completed' },
        resultAccounts: result.imported,
        failedImports: [],
      }));
      return result;
    } catch (error) {
      set({ importing: false, error: normalizeInvokeError(error) });
      return null;
    }
  },
  async cancelOAuthLogin() {
    const loginId = useImportFlowStore.getState().oauth.login?.loginId;
    set((state) => ({
      oauth: { ...state.oauth, cancelling: true },
      error: null,
    }));
    try {
      await cancelCodexOAuthLogin(loginId);
      set({ oauth: { ...emptyOAuthFlow, step: 'cancelled' }, error: null, resultAccounts: [], failedImports: [] });
    } catch (error) {
      set((state) => ({
        oauth: { ...state.oauth, cancelling: false },
        error: normalizeInvokeError(error),
      }));
    }
  },
  async chooseFiles() {
    set({
      selectingFiles: true,
      error: null,
      resultAccounts: [],
      failedImports: [],
      batchPreview: null,
      batchSelectedItemIds: [],
    });
    try {
      const filePaths = await chooseCodexAuthFiles();
      set({ selectingFiles: false, filePaths });
      if (filePaths.length > 0) {
        set({ previewingBatch: true });
        const batchPreview = await startCodexBatchImportFromFiles(filePaths, false);
        set({
          batchPreview,
          batchSelectedItemIds: defaultBatchSelection(batchPreview),
          previewingBatch: false,
          error: null,
        });
      }
    } catch (error) {
      set({ selectingFiles: false, previewingBatch: false, error: normalizeInvokeError(error) });
    }
  },
  clearFiles() {
    set({
      filePaths: [],
      batchPreview: null,
      batchSelectedItemIds: [],
      error: null,
      resultAccounts: [],
      failedImports: [],
    });
  },
  toggleBatchItem(itemId) {
    set((state) => {
      const selected = state.batchSelectedItemIds.includes(itemId);
      return {
        batchSelectedItemIds: selected
          ? state.batchSelectedItemIds.filter((currentItemId) => currentItemId !== itemId)
          : [...state.batchSelectedItemIds, itemId],
        error: null,
      };
    });
  },
  setAllBatchItemsSelected(selected) {
    set((state) => ({
      batchSelectedItemIds:
        selected && state.batchPreview
          ? state.batchPreview.items.filter((item) => item.selectable && item.status === 'importable').map((item) => item.id)
          : [],
      error: null,
    }));
  },
  async importSelected() {
    set({ importing: true, error: null });
    const state = useImportFlowStore.getState();
    try {
      let result: ImportResult;

      if (state.source === 'local') {
        result = singleAccountResult(await importCodexFromLocal());
      } else if (state.source === 'jsonFile' || state.source === 'batchFiles') {
        if (state.filePaths.length === 0) {
          throw appError('IMPORT_FILES_EMPTY', 'No JSON file selected.', 'Choose at least one Codex auth JSON file.');
        }
        if (state.batchPreview) {
          if (state.batchSelectedItemIds.length === 0) {
            throw appError('IMPORT_BATCH_EMPTY', 'No importable item selected.', 'Select at least one importable account.');
          }
          result = await confirmCodexBatchImport(state.batchPreview.sessionId, state.batchSelectedItemIds);
        } else {
          result = await importCodexFromFiles(state.filePaths);
        }
      } else if (state.source === 'token') {
        const validationError = validateTokenFields(state.tokenFields);
        if (validationError) {
          throw validationError;
        }
        result = singleAccountResult(
          await addCodexAccountWithToken(
            state.tokenFields.idToken.trim(),
            state.tokenFields.accessToken.trim(),
            state.tokenFields.refreshToken.trim() || undefined,
          ),
        );
      } else if (state.source === 'apiKey') {
        const validationError = validateApiKeyFields(state.apiKeyFields);
        if (validationError) {
          throw validationError;
        }
        result = singleAccountResult(
          await addCodexAccountWithApiKey(
            state.apiKeyFields.apiKey.trim(),
            state.apiKeyFields.apiBaseUrl.trim() || undefined,
            state.apiKeyFields.displayName.trim() || undefined,
          ),
        );
      } else if (state.source === 'oauth') {
        if (!state.oauth.login) {
          throw oauthRequiredError();
        }
        if (isOAuthExpired(state.oauth.login)) {
          set((current) => ({ oauth: { ...current.oauth, step: 'expired' } }));
          throw appError('IMPORT_OAUTH_EXPIRED', 'OAuth login has expired.', 'Start OAuth login again.');
        }
        if (state.oauth.step !== 'callbackSubmitted' && !state.oauth.login.listenerStarted) {
          throw appError(
            'IMPORT_OAUTH_CALLBACK_NOT_SUBMITTED',
            'OAuth callback has not been submitted.',
            'Paste the callback URL and submit it before importing.',
          );
        }
        const oauthResult = await useImportFlowStore.getState().pollOAuthLoginStatus();
        if (!oauthResult) {
          throw appError(
            'IMPORT_OAUTH_CALLBACK_NOT_READY',
            'OAuth callback has not been received yet.',
            'Finish authorization in the browser, then return to the app.',
          );
        }
        result = oauthResult;
      } else if (state.source === 'jsonText') {
        const validationError = validateJsonText(state.jsonText);
        if (validationError) {
          throw validationError;
        }
        result = await importCodexFromJson(state.jsonText.trim());
      } else {
        throw appError('IMPORT_SOURCE_UNSUPPORTED', 'Import source is not supported.', 'Choose another import source.');
      }

      const shouldClose = shouldCloseAfterSuccessfulImport(result);
      set((current) => ({
        open: shouldClose ? false : current.open,
        importing: false,
        oauth: current.source === 'oauth' ? { ...current.oauth, step: 'completed' } : current.oauth,
        resultAccounts: result.imported,
        failedImports: result.failed,
        ...(shouldClose ? emptyImportDraft() : { batchPreview: null, batchSelectedItemIds: [] }),
      }));
      return result;
    } catch (error) {
      set({ importing: false, error: normalizeInvokeError(error) });
      return null;
    }
  },
}));
