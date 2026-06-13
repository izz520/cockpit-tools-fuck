import { useEffect, useState } from 'react';
import { KeyRound, Link, Tag } from 'lucide-react';
import type { CodexAccountView } from '../../types/codex';
import { Button } from '../ui/Button';
import './EditApiAccountModal.css';

interface EditApiAccountModalProps {
  account: CodexAccountView | null;
  saving: boolean;
  onCancel: () => void;
  onSave: (accountId: string, apiKey: string, apiBaseUrl: string | null, displayName: string | null) => void;
}

function getInitialApiBaseUrl(account: CodexAccountView): string {
  return account.apiBaseUrl ?? 'https://api.openai.com/v1';
}

export function EditApiAccountModal({ account, saving, onCancel, onSave }: EditApiAccountModalProps) {
  const [displayName, setDisplayName] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [apiBaseUrl, setApiBaseUrl] = useState('');

  useEffect(() => {
    if (!account) {
      return;
    }
    setDisplayName(account.displayName);
    setApiKey(account.apiKey ?? '');
    setApiBaseUrl(getInitialApiBaseUrl(account));
  }, [account]);

  if (!account) {
    return null;
  }

  const canSave = apiKey.trim().length > 0 && apiBaseUrl.trim().length > 0;

  return (
    <div className="api-edit-modal-backdrop" role="presentation">
      <section className="api-edit-modal" role="dialog" aria-modal="true" aria-labelledby="api-edit-modal-title">
        <header>
          <h2 id="api-edit-modal-title">编辑 API Key 账号</h2>
          <p>修改后会更新本地保存的 API Key 和基础地址。当前账号会同步写入 Codex 配置。</p>
        </header>

        <div className="api-edit-fields">
          <label>
            <span>
              <Tag size={15} />
              名称
            </span>
            <input
              value={displayName}
              placeholder="例如 Funcode"
              onChange={(event) => setDisplayName(event.target.value)}
            />
          </label>
          <label>
            <span>
              <KeyRound size={15} />
              API Key
            </span>
            <input
              value={apiKey}
              placeholder="sk-..."
              spellCheck={false}
              type="password"
              onChange={(event) => setApiKey(event.target.value)}
            />
          </label>
          <label>
            <span>
              <Link size={15} />
              基础地址
            </span>
            <input
              value={apiBaseUrl}
              placeholder="https://api.example.com/v1"
              spellCheck={false}
              onChange={(event) => setApiBaseUrl(event.target.value)}
            />
          </label>
        </div>

        <footer>
          <Button variant="ghost" disabled={saving} onClick={onCancel}>
            取消
          </Button>
          <Button
            variant="primary"
            loading={saving}
            disabled={!canSave}
            onClick={() => onSave(account.id, apiKey, apiBaseUrl.trim() || null, displayName.trim() || null)}
          >
            保存修改
          </Button>
        </footer>
      </section>
    </div>
  );
}
