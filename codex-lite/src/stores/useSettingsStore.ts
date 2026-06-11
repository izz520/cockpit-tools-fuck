import { create } from 'zustand';
import { detectCodexPaths, getSystemSnapshot } from '../services/systemService';
import type { AppError, SystemSnapshot } from '../types/system';
import { normalizeInvokeError } from '../services/tauriInvoke';

interface SettingsState {
  snapshot: SystemSnapshot | null;
  loading: boolean;
  detecting: boolean;
  error: AppError | null;
  loadSnapshot: () => Promise<void>;
  detectPaths: () => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  snapshot: null,
  loading: false,
  detecting: false,
  error: null,
  async loadSnapshot() {
    set({ loading: true, error: null });
    try {
      set({ snapshot: await getSystemSnapshot(), loading: false });
    } catch (error) {
      set({ error: normalizeInvokeError(error), loading: false });
    }
  },
  async detectPaths() {
    set({ detecting: true, error: null });
    try {
      await detectCodexPaths();
      set({ snapshot: await getSystemSnapshot(), detecting: false });
    } catch (error) {
      set({ error: normalizeInvokeError(error), detecting: false });
    }
  },
}));
