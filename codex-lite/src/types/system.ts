export interface AppError {
  code: string;
  message: string;
  action: string;
  details?: unknown;
  retryable: boolean;
}

export interface AppSettings {
  schemaVersion: string;
  codexHomePath?: string | null;
  authFilePath?: string | null;
  theme: 'system' | 'light' | 'dark';
  quotaRefreshOnStart: boolean;
}

export interface LogEntry {
  level: 'error' | 'warn' | 'info';
  message: string;
  timestamp: number;
}

export interface LogSnapshot {
  entries: LogEntry[];
}

export interface SystemSnapshot {
  appDataDir: string;
  logsDir: string;
  accountsFilePath: string;
  settingsFilePath: string;
  defaultCodexHome: string;
  defaultCodexAuthFile: string;
  codexAuthFileExists: boolean;
}
