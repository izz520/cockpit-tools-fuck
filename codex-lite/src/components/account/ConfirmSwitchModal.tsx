import type { CodexAccountView } from '../../types/codex';
import { Button } from '../ui/Button';
import './ConfirmSwitchModal.css';

interface ConfirmSwitchModalProps {
  account: CodexAccountView | null;
  switching: boolean;
  onCancel: () => void;
  onConfirm: (accountId: string) => void;
}

function accountSubtitle(account: CodexAccountView): string {
  return account.email ?? account.accountId ?? account.id;
}

export function ConfirmSwitchModal({ account, switching, onCancel, onConfirm }: ConfirmSwitchModalProps) {
  if (!account) {
    return null;
  }

  return (
    <div className="switch-modal-backdrop" role="presentation">
      <section className="switch-modal" role="dialog" aria-modal="true" aria-labelledby="switch-modal-title">
        <header>
          <h2 id="switch-modal-title">Switch Codex account?</h2>
          <p>This will replace the active local Codex auth file with the selected account.</p>
        </header>

        <div className="switch-modal-account">
          <span>{account.authMode === 'api_key' ? 'API Key' : 'OAuth'}</span>
          <strong title={account.displayName}>{account.displayName}</strong>
          <code title={accountSubtitle(account)}>{accountSubtitle(account)}</code>
        </div>

        <footer>
          <Button variant="ghost" disabled={switching} onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="primary" loading={switching} onClick={() => onConfirm(account.id)}>
            Confirm Switch
          </Button>
        </footer>
      </section>
    </div>
  );
}
