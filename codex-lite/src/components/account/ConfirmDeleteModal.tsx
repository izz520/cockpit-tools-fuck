import type { CodexAccountView } from '../../types/codex';
import { Button } from '../ui/Button';
import './ConfirmDeleteModal.css';

interface ConfirmDeleteModalProps {
  account: CodexAccountView | null;
  deleting: boolean;
  onCancel: () => void;
  onConfirm: (accountId: string) => void;
}

function accountSubtitle(account: CodexAccountView): string {
  return account.email ?? account.accountId ?? account.id;
}

export function ConfirmDeleteModal({ account, deleting, onCancel, onConfirm }: ConfirmDeleteModalProps) {
  if (!account) {
    return null;
  }

  return (
    <div className="delete-modal-backdrop" role="presentation">
      <section className="delete-modal" role="dialog" aria-modal="true" aria-labelledby="delete-modal-title">
        <header>
          <h2 id="delete-modal-title">Remove this account?</h2>
          <p>This removes the saved account from Codex Lite. It does not sign you out elsewhere.</p>
        </header>

        <div className="delete-modal-account">
          <span>{account.authMode === 'api_key' ? 'API Key' : 'OAuth'}</span>
          <strong title={account.displayName}>{account.displayName}</strong>
          <code title={accountSubtitle(account)}>{accountSubtitle(account)}</code>
        </div>

        {account.isCurrent ? (
          <p className="delete-modal-warning" role="alert">
            This is the current account. Removing it here leaves the active local Codex auth file in place; switch to
            another account if you no longer want to use it.
          </p>
        ) : null}

        <footer>
          <Button variant="ghost" disabled={deleting} onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="danger" loading={deleting} onClick={() => onConfirm(account.id)}>
            Remove account
          </Button>
        </footer>
      </section>
    </div>
  );
}
