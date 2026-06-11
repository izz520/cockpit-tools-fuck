import type { AppSettings, LogSnapshot, SystemSnapshot } from '../types/system';
import { invokeCommand } from './tauriInvoke';

export function getSettings(): Promise<AppSettings> {
  return invokeCommand('get_settings');
}

export function saveSettings(settings: AppSettings): Promise<AppSettings> {
  return invokeCommand('save_settings', { settings });
}

export function detectCodexPaths(): Promise<AppSettings> {
  return invokeCommand('detect_codex_paths');
}

export function openDataDir(): Promise<void> {
  return invokeCommand('open_data_dir');
}

export function openLogDir(): Promise<void> {
  return invokeCommand('open_log_dir');
}

export function getLogSnapshot(maxLines: number): Promise<LogSnapshot> {
  return invokeCommand('get_log_snapshot', { maxLines });
}

export function getSystemSnapshot(): Promise<SystemSnapshot> {
  return invokeCommand('get_system_snapshot');
}
