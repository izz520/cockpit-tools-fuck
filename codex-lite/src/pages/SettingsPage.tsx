import { useEffect } from 'react';
import { Button } from '../components/ui/Button';
import { Panel } from '../components/ui/Panel/Panel';
import { openDataDir, openLogDir } from '../services/systemService';
import { useSettingsStore } from '../stores/useSettingsStore';
import type { SystemSnapshot } from '../types/system';

const pathRows: Array<{ label: string; key: keyof SystemSnapshot }> = [
  { label: '应用数据', key: 'appDataDir' },
  { label: '日志目录', key: 'logsDir' },
  { label: '账号文件', key: 'accountsFilePath' },
  { label: '设置文件', key: 'settingsFilePath' },
  { label: 'Codex Home', key: 'defaultCodexHome' },
  { label: 'Codex Auth', key: 'defaultCodexAuthFile' },
];

export function SettingsPage() {
  const { snapshot, error, loading, detecting, loadSnapshot, detectPaths } = useSettingsStore();

  useEffect(() => {
    void loadSnapshot();
  }, [loadSnapshot]);

  return (
    <div className="content">
      <Panel>
        <h2 className="section-title">设置</h2>
        <p className="muted">本地路径和隐私相关配置，仅在当前设备生效。</p>
        {error ? <p className="muted">{error.message}</p> : null}
        {snapshot ? (
          <div className="settings-grid">
            {pathRows.map((row) => (
              <div className="settings-row" key={row.key}>
                <span>{row.label}</span>
                <code>{String(snapshot[row.key])}</code>
              </div>
            ))}
            <div className="settings-row">
              <span>Auth 文件状态</span>
              <strong>{snapshot.codexAuthFileExists ? '已找到' : '缺失'}</strong>
            </div>
          </div>
        ) : (
          <p className="muted">{loading ? '正在加载本地路径...' : '还没有加载本地路径快照。'}</p>
        )}
        <div className="toolbar-actions">
          <Button variant="secondary" loading={detecting} onClick={() => void detectPaths()}>
            检测 Codex 路径
          </Button>
          <Button variant="secondary" onClick={() => void openDataDir()}>
            打开数据目录
          </Button>
          <Button variant="secondary" onClick={() => void openLogDir()}>
            打开日志
          </Button>
        </div>
      </Panel>
    </div>
  );
}
