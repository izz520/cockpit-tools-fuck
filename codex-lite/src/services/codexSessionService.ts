import type { CodexSessionView, SessionMutationResult } from '../types/session';
import { invokeCommand } from './tauriInvoke';

export function listCodexSessions(): Promise<CodexSessionView[]> {
  return invokeCommand('list_codex_sessions');
}

export function restoreCodexSessionsVisibility(sessionIds: string[]): Promise<SessionMutationResult> {
  return invokeCommand('restore_codex_sessions_visibility', { sessionIds });
}

export function deleteCodexSessions(sessionIds: string[]): Promise<SessionMutationResult> {
  return invokeCommand('delete_codex_sessions', { sessionIds });
}
