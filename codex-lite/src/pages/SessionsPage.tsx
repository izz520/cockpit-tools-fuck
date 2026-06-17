import { useEffect, useMemo, useState } from 'react';
import { Archive, CheckCircle2, CheckSquare, ChevronDown, ChevronRight, EyeOff, Folder, RefreshCw, RotateCcw, Search, Trash2 } from 'lucide-react';
import { Button } from '../components/ui/Button';
import { ErrorBanner } from '../components/ui/ErrorBanner';
import { useCodexSessionsStore } from '../stores/useCodexSessionsStore';
import type { CodexSessionView } from '../types/session';
import './SessionsPage.css';

function formatTime(value?: number | null): string {
  if (!value) {
    return '-';
  }
  const date = new Date(value);
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const hour = String(date.getHours()).padStart(2, '0');
  const minute = String(date.getMinutes()).padStart(2, '0');
  return `${month}/${day} ${hour}:${minute}`;
}

function matchesQuery(session: CodexSessionView, query: string): boolean {
  const haystack = [
    session.title,
    session.project,
    session.cwd,
    session.provider,
    session.targetProvider,
    session.preview,
    session.id,
  ]
    .filter(Boolean)
    .join(' ')
    .toLowerCase();
  return haystack.includes(query);
}

interface SessionGroup {
  key: string;
  project: string;
  cwd: string;
  sessions: CodexSessionView[];
}

interface SessionsPageProps {
  onBack: () => void;
}

export function SessionsPage({ onBack }: SessionsPageProps) {
  const { sessions, loading, restoring, deleting, error, loadSessions, restoreSessions, deleteSessions } =
    useCodexSessionsStore();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [collapsedGroupKeys, setCollapsedGroupKeys] = useState<string[]>([]);

  useEffect(() => {
    void loadSessions();
  }, [loadSessions]);

  const filteredSessions = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    if (query.length === 0) {
      return sessions;
    }
    return sessions.filter((session) => matchesQuery(session, query));
  }, [searchQuery, sessions]);
  const selectedSet = useMemo(() => new Set(selectedIds), [selectedIds]);
  const collapsedGroupSet = useMemo(() => new Set(collapsedGroupKeys), [collapsedGroupKeys]);
  const selectedCount = selectedIds.length;
  const visibleCount = useMemo(() => sessions.filter((session) => session.visible).length, [sessions]);
  const hiddenCount = sessions.length - visibleCount;
  const allFilteredSelected =
    filteredSessions.length > 0 && filteredSessions.every((session) => selectedSet.has(session.id));
  const groups = useMemo(() => {
    const grouped = new Map<string, SessionGroup>();
    for (const session of filteredSessions) {
      const key = `${session.project}\n${session.cwd}`;
      const existing = grouped.get(key);
      if (existing) {
        existing.sessions.push(session);
      } else {
        grouped.set(key, {
          key,
          project: session.project,
          cwd: session.cwd,
          sessions: [session],
        });
      }
    }
    return Array.from(grouped.values()).sort((left, right) => left.project.localeCompare(right.project));
  }, [filteredSessions]);
  const allGroupsExpanded = groups.length > 0 && groups.every((group) => !collapsedGroupSet.has(group.key));

  function toggleSession(sessionId: string) {
    setSelectedIds((current) =>
      current.includes(sessionId) ? current.filter((id) => id !== sessionId) : [...current, sessionId],
    );
  }

  function toggleFilteredSelection() {
    if (allFilteredSelected) {
      const filteredIds = new Set(filteredSessions.map((session) => session.id));
      setSelectedIds((current) => current.filter((id) => !filteredIds.has(id)));
      return;
    }
    setSelectedIds((current) => Array.from(new Set([...current, ...filteredSessions.map((session) => session.id)])));
  }

  function toggleGroup(groupKey: string) {
    setCollapsedGroupKeys((current) =>
      current.includes(groupKey) ? current.filter((key) => key !== groupKey) : [...current, groupKey],
    );
  }

  function toggleAllGroups() {
    const groupKeys = groups.map((group) => group.key);
    if (allGroupsExpanded) {
      setCollapsedGroupKeys((current) => Array.from(new Set([...current, ...groupKeys])));
      return;
    }
    const visibleGroupKeys = new Set(groupKeys);
    setCollapsedGroupKeys((current) => current.filter((key) => !visibleGroupKeys.has(key)));
  }

  function toggleGroupSelection(group: SessionGroup) {
    const everySelected = group.sessions.every((session) => selectedSet.has(session.id));
    if (everySelected) {
      const groupIds = new Set(group.sessions.map((session) => session.id));
      setSelectedIds((current) => current.filter((id) => !groupIds.has(id)));
      return;
    }
    setSelectedIds((current) => Array.from(new Set([...current, ...group.sessions.map((session) => session.id)])));
  }

  async function restoreSelected() {
    await restoreSessions(selectedIds);
    setSelectedIds([]);
  }

  async function deleteSelected() {
    const confirmed = window.confirm(`确定删除选中的 ${selectedCount} 个会话吗？这会删除本机 Codex 会话记录。`);
    if (!confirmed) {
      return;
    }
    await deleteSessions(selectedIds);
    setSelectedIds([]);
  }

  return (
    <>
      {error ? <ErrorBanner error={error} /> : null}

      <div className="sessions-stats" aria-label="Session statistics">
        <article>
          <strong>{sessions.length}</strong>
          <span>全部会话</span>
        </article>
        <article>
          <strong>{visibleCount}</strong>
          <span>当前可见</span>
        </article>
        <article>
          <strong>{hiddenCount}</strong>
          <span>待恢复</span>
        </article>
        <article>
          <strong>{selectedCount}</strong>
          <span>已选择</span>
        </article>
      </div>

        <div className="sessions-toolbar">
          <label className="session-search">
            <Search size={16} aria-hidden="true" />
            <input
              aria-label="Search sessions"
              placeholder="搜索项目、标题、路径或 provider"
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
            />
          </label>
          <div className="sessions-actions">
            <Button variant="secondary" icon={<RefreshCw size={16} />} loading={loading} onClick={() => void loadSessions()}>
              刷新
            </Button>
            <Button
              variant="secondary"
              icon={allGroupsExpanded ? <ChevronRight size={16} /> : <ChevronDown size={16} />}
              disabled={groups.length === 0}
              onClick={toggleAllGroups}
            >
              {allGroupsExpanded ? '一键收起' : '一键展开'}
            </Button>
            <Button
              variant="secondary"
              icon={<CheckSquare size={16} />}
              disabled={filteredSessions.length === 0}
              onClick={toggleFilteredSelection}
            >
              {allFilteredSelected ? '取消全选' : '全选'}
            </Button>
            <Button
              variant="secondary"
              icon={<RotateCcw size={16} />}
              loading={restoring}
              disabled={selectedCount === 0}
              onClick={() => void restoreSelected()}
            >
              恢复可见
            </Button>
            <Button
              variant="danger"
              icon={<Trash2 size={16} />}
              loading={deleting}
              disabled={selectedCount === 0}
              onClick={() => void deleteSelected()}
            >
              删除
            </Button>
          </div>
        </div>

        <div className="sessions-group-list">
          {!loading && groups.length === 0 ? <div className="sessions-empty">没有找到匹配的会话。</div> : null}
          {loading ? <div className="sessions-empty">正在加载会话...</div> : null}
          {groups.map((group) => {
            const collapsed = collapsedGroupSet.has(group.key);
            const selectedInGroup = group.sessions.filter((session) => selectedSet.has(session.id)).length;
            const groupSelected = selectedInGroup === group.sessions.length;
            const groupVisibleCount = group.sessions.filter((session) => session.visible).length;
            return (
              <section className="session-group" key={group.key}>
                <div className="session-group-header">
                  <button className="session-group-toggle" type="button" onClick={() => toggleGroup(group.key)}>
                    {collapsed ? <ChevronRight size={16} /> : <ChevronDown size={16} />}
                    <span className="session-group-icon">
                      <Folder size={16} />
                    </span>
                    <span className="session-group-title">
                      <strong>{group.project}</strong>
                      <span>{group.cwd || 'Unknown path'}</span>
                    </span>
                  </button>
                  <div className="session-group-meta">
                    <span>{group.sessions.length} 个会话</span>
                    <span>{groupVisibleCount} 个可见</span>
                    {selectedInGroup > 0 ? <span className="session-group-selected-count">{selectedInGroup} 个已选</span> : null}
                  </div>
                </div>
                {!collapsed ? (
                  <div className="sessions-table-panel">
                    <table className="sessions-table">
                      <colgroup>
                        <col className="session-col-select" />
                        <col className="session-col-main" />
                        <col className="session-col-status" />
                        <col className="session-col-time" />
                      </colgroup>
                      <thead>
                        <tr>
                          <th className="select-cell">
                            <input
                              aria-label={`Select all sessions in ${group.project}`}
                              checked={groupSelected}
                              type="checkbox"
                              onChange={() => toggleGroupSelection(group)}
                            />
                          </th>
                          <th>会话</th>
                          <th>状态</th>
                          <th>更新时间</th>
                        </tr>
                      </thead>
                      <tbody>
                        {group.sessions.map((session) => (
                          <tr key={session.id} className={selectedSet.has(session.id) ? 'selected' : ''}>
                            <td className="select-cell">
                              <input
                                aria-label={`Select ${session.title}`}
                                checked={selectedSet.has(session.id)}
                                type="checkbox"
                                onChange={() => toggleSession(session.id)}
                              />
                            </td>
                            <td>
                              <div className="session-title-cell">
                                <strong>{session.title}</strong>
                                <span>{session.preview || session.id}</span>
                              </div>
                            </td>
                            <td>
                              <div className="session-status-stack">
                                <span className={session.visible ? 'session-status visible' : 'session-status hidden'}>
                                  {session.visible ? <CheckCircle2 size={14} /> : <EyeOff size={14} />}
                                  {session.visible ? '可见' : '需恢复'}
                                </span>
                                {session.archived ? (
                                  <span className="session-archive">
                                    <Archive size={13} />
                                    archived
                                  </span>
                                ) : null}
                              </div>
                            </td>
                            <td className="session-time-cell">{formatTime(session.updatedAt)}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                ) : null}
              </section>
            );
          })}
        </div>
    </>
  );
}
