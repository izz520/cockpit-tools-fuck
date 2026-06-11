import { invoke } from '@tauri-apps/api/core';
import type { AppError } from '../types/system';

export async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeInvokeError(error);
  }
}

export function normalizeInvokeError(error: unknown): AppError {
  if (typeof error === 'object' && error !== null) {
    const value = error as Partial<AppError>;
    if (typeof value.code === 'string' && typeof value.message === 'string') {
      return {
        code: value.code,
        message: value.message,
        action: value.action ?? 'Try again or open logs for details.',
        details: value.details,
        retryable: value.retryable ?? false,
      };
    }
  }

  return {
    code: 'UNKNOWN_ERROR',
    message: error instanceof Error ? error.message : String(error),
    action: 'Open logs for details.',
    retryable: false,
  };
}
