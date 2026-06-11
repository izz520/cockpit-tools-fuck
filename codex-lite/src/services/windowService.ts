import { invokeCommand } from './tauriInvoke';

export function startWindowDragging(): Promise<void> {
  return invokeCommand('window_start_dragging');
}
