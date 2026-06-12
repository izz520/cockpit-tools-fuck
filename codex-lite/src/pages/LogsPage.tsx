import { useEffect, useState } from 'react';
import { Button } from '../components/ui/Button';
import { getLogSnapshot, openLogDir } from '../services/systemService';
import type { LogEntry } from '../types/system';

type LogFilter = 'all' | LogEntry['level'];

const logFilters: Array<{ label: string; value: LogFilter }> = [
  { label: '全部', value: 'all' },
  { label: '错误', value: 'error' },
  { label: '警告', value: 'warn' },
  { label: '信息', value: 'info' },
];

export function LogsPage() {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [filter, setFilter] = useState<LogFilter>('all');
  const filteredEntries = filter === 'all' ? entries : entries.filter((entry) => entry.level === filter);

  async function refreshLogs() {
    setLoading(true);
    try {
      const snapshot = await getLogSnapshot(200);
      setEntries(snapshot.entries);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refreshLogs();
  }, []);

  return (
    <div className="content">
      <section className="panel">
        <div className="page-section-header">
          <div>
            <h2 className="section-title">日志</h2>
            <p className="muted">查看近期本地日志，敏感字段会保持脱敏显示。</p>
          </div>
          <div className="toolbar-actions">
            <Button variant="secondary" loading={loading} onClick={() => void refreshLogs()}>
              刷新
            </Button>
            <Button variant="secondary" onClick={() => void openLogDir()}>
              打开日志
            </Button>
          </div>
        </div>
        {error ? <p className="muted">{error}</p> : null}
        <div className="log-filter-tabs" role="tablist" aria-label="Log level filter">
          {logFilters.map((item) => (
            <button
              aria-selected={filter === item.value}
              className={filter === item.value ? 'active' : ''}
              key={item.value}
              role="tab"
              type="button"
              onClick={() => setFilter(item.value)}
            >
              {item.label}
            </button>
          ))}
        </div>
        {filteredEntries.length === 0 ? (
          <pre className="mono">{entries.length === 0 ? '还没有加载日志。' : '当前筛选条件下没有日志。'}</pre>
        ) : (
          <div className="log-list">
            {filteredEntries.map((entry, index) => (
              <div className={`log-entry log-${entry.level}`} key={`${entry.timestamp}-${index}`}>
                <span>{entry.level.toUpperCase()}</span>
                <code>{entry.message}</code>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
