import { create } from 'zustand';
import {
  deleteCodexSessions,
  listCodexSessions,
  restoreCodexSessionsVisibility,
} from '../services/codexSessionService';
import type { CodexSessionView } from '../types/session';
import type { AppError } from '../types/system';

interface CodexSessionsState {
  sessions: CodexSessionView[];
  loading: boolean;
  restoring: boolean;
  deleting: boolean;
  error: AppError | null;
  loadSessions: () => Promise<void>;
  restoreSessions: (sessionIds: string[]) => Promise<void>;
  deleteSessions: (sessionIds: string[]) => Promise<void>;
}

export const useCodexSessionsStore = create<CodexSessionsState>((set) => ({
  sessions: [],
  loading: false,
  restoring: false,
  deleting: false,
  error: null,
  async loadSessions() {
    set({ loading: true, error: null });
    try {
      const sessions = await listCodexSessions();
      set({ sessions, loading: false });
    } catch (error) {
      set({ error: error as AppError, loading: false });
    }
  },
  async restoreSessions(sessionIds) {
    set({ restoring: true, error: null });
    try {
      await restoreCodexSessionsVisibility(sessionIds);
      const sessions = await listCodexSessions();
      set({ sessions, restoring: false });
    } catch (error) {
      set({ error: error as AppError, restoring: false });
    }
  },
  async deleteSessions(sessionIds) {
    set({ deleting: true, error: null });
    try {
      await deleteCodexSessions(sessionIds);
      const sessions = await listCodexSessions();
      set({ sessions, deleting: false });
    } catch (error) {
      set({ error: error as AppError, deleting: false });
    }
  },
}));
