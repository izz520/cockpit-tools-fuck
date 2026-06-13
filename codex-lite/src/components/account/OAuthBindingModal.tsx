import { Check, KeyRound, Link2, ShieldCheck, X } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { isOAuthAuthMode, type CodexAccountView } from '../../types/codex';
import { Button } from '../ui/Button';
import './OAuthBindingModal.css';

interface OAuthBindingModalProps {
  apiAccount: CodexAccountView | null;
  accounts: CodexAccountView[];
  saving: boolean;
  onCancel: () => void;
  onSave: (apiAccountId: string, boundOAuthAccountId: string | null) => void;
}

function accountLabel(account: CodexAccountView): string {
  return account.email ?? account.displayName;
}

export function OAuthBindingModal({
  apiAccount,
  accounts,
  saving,
  onCancel,
  onSave,
}: OAuthBindingModalProps) {
  const oauthAccounts = useMemo(
    () => accounts.filter((account) => isOAuthAuthMode(account.authMode)),
    [accounts],
  );
  const [selectedId, setSelectedId] = useState<string | null>(null);

  useEffect(() => {
    setSelectedId(apiAccount?.boundOauthAccountId ?? oauthAccounts[0]?.id ?? null);
  }, [apiAccount, oauthAccounts]);

  if (!apiAccount) {
    return null;
  }

  const selectedAccount = oauthAccounts.find((account) => account.id === selectedId) ?? null;
  const canSave = selectedId !== apiAccount.boundOauthAccountId;

  return (
    <div className="oauth-binding-modal-backdrop" role="presentation">
      <section className="oauth-binding-modal" role="dialog" aria-modal="true" aria-labelledby="oauth-binding-modal-title">
        <header>
          <div>
            <h2 id="oauth-binding-modal-title">绑定 OAuth 账号</h2>
            <p>登录态使用 OAuth，API 请求继续使用当前 API Key 的基础地址与配置。</p>
          </div>
          <button type="button" aria-label="Close OAuth binding modal" onClick={onCancel}>
            <X size={18} />
          </button>
        </header>

        <div className="oauth-binding-target">
          <span>
            <KeyRound size={16} />
            API Key 账号
          </span>
          <strong>{apiAccount.displayName}</strong>
          <code>{apiAccount.apiBaseUrl ?? 'https://api.openai.com/v1'}</code>
        </div>

        <div className="oauth-binding-list" role="radiogroup" aria-label="OAuth accounts">
          {oauthAccounts.length === 0 ? (
            <div className="oauth-binding-empty">
              <ShieldCheck size={20} />
              <span>还没有可绑定的 OAuth 账号，请先添加 OAuth 登录。</span>
            </div>
          ) : null}
          {oauthAccounts.map((account) => {
            const selected = account.id === selectedId;
            return (
              <button
                key={account.id}
                type="button"
                className={selected ? 'selected' : ''}
                role="radio"
                aria-checked={selected}
                onClick={() => setSelectedId(account.id)}
              >
                <span className="oauth-binding-radio">
                  {selected ? <Check size={14} /> : null}
                </span>
                <span className="oauth-binding-account">
                  <strong>{accountLabel(account)}</strong>
                  <span>{account.planType ?? 'OAuth'}</span>
                </span>
              </button>
            );
          })}
        </div>

        {selectedAccount ? (
          <div className="oauth-binding-preview">
            <Link2 size={15} />
            <span>{apiAccount.displayName} 将使用 {accountLabel(selectedAccount)} 的 OAuth 登录态。</span>
          </div>
        ) : null}

        <footer>
          <Button
            variant="ghost"
            disabled={saving || !apiAccount.boundOauthAccountId}
            onClick={() => onSave(apiAccount.id, null)}
          >
            清除绑定
          </Button>
          <div>
            <Button variant="ghost" disabled={saving} onClick={onCancel}>
              取消
            </Button>
            <Button
              variant="primary"
              loading={saving}
              disabled={!selectedId || !canSave}
              onClick={() => onSave(apiAccount.id, selectedId)}
            >
              保存绑定
            </Button>
          </div>
        </footer>
      </section>
    </div>
  );
}
