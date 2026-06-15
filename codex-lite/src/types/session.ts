export interface CodexSessionView {
  id: string;
  title: string;
  project: string;
  cwd: string;
  provider: string;
  targetProvider: string;
  visible: boolean;
  archived: boolean;
  updatedAt?: number | null;
  createdAt?: number | null;
  rolloutPath?: string | null;
  preview?: string | null;
}

export interface SessionMutationResult {
  updatedCount: number;
  deletedCount: number;
}

export interface SessionRepairSummary {
  repaired: boolean;
  instanceCount: number;
  rolloutFileCount: number;
  sqliteRowCount: number;
  indexEntryCount: number;
  backupPath?: string | null;
  targetProvider?: string | null;
}
