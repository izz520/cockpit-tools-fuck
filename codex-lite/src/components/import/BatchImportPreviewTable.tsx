import { AlertTriangle, CheckCircle2, CircleMinus } from 'lucide-react';
import type { BatchImportItem } from '../../types/import';
import './BatchImportPreviewTable.css';

interface BatchImportPreviewTableProps {
  items: BatchImportItem[];
  selectedItemIds: string[];
  onToggleItem: (itemId: string) => void;
  onToggleAll: (selected: boolean) => void;
}

function itemSubtitle(item: BatchImportItem): string {
  return item.email ?? item.accountId ?? item.userId ?? item.source;
}

function itemLabel(item: BatchImportItem): string {
  return item.displayName ?? item.email ?? item.accountId ?? item.source;
}

function statusIcon(status: BatchImportItem['status']) {
  if (status === 'importable') {
    return <CheckCircle2 size={15} aria-hidden="true" />;
  }
  if (status === 'existing') {
    return <CircleMinus size={15} aria-hidden="true" />;
  }
  return <AlertTriangle size={15} aria-hidden="true" />;
}

export function BatchImportPreviewTable({
  items,
  selectedItemIds,
  onToggleItem,
  onToggleAll,
}: BatchImportPreviewTableProps) {
  const importableItems = items.filter((item) => item.status === 'importable');
  const allImportableSelected =
    importableItems.length > 0 && importableItems.every((item) => selectedItemIds.includes(item.id));

  return (
    <div className="batch-preview">
      <div className="batch-preview-toolbar">
        <div>
          <strong>{items.length} file result(s)</strong>
          <span>{selectedItemIds.length} selected</span>
        </div>
        <label className="batch-select-all">
          <input
            checked={allImportableSelected}
            disabled={importableItems.length === 0}
            type="checkbox"
            onChange={(event) => onToggleAll(event.target.checked)}
          />
          <span>Select importable</span>
        </label>
      </div>

      <div className="batch-preview-table" role="table" aria-label="Batch import preview">
        <div className="batch-preview-row batch-preview-head" role="row">
          <span role="columnheader">Use</span>
          <span role="columnheader">Account</span>
          <span role="columnheader">Status</span>
        </div>
        {items.map((item) => {
          const selectable = item.selectable && item.status === 'importable';
          return (
            <div className="batch-preview-row" role="row" key={item.id}>
              <label className="batch-preview-check">
                <input
                  checked={selectedItemIds.includes(item.id)}
                  disabled={!selectable}
                  type="checkbox"
                  onChange={() => onToggleItem(item.id)}
                />
                <span className="sr-only">Select {itemLabel(item)}</span>
              </label>
              <div className="batch-preview-account">
                <strong title={itemLabel(item)}>{itemLabel(item)}</strong>
                <small title={itemSubtitle(item)}>{itemSubtitle(item)}</small>
              </div>
              <div className={`batch-preview-status status-${item.status}`}>
                {statusIcon(item.status)}
                <span>{item.status}</span>
                {item.reason ? <small title={item.reason}>{item.reason}</small> : null}
                {item.quotaWarning ? <small title={item.quotaWarning}>{item.quotaWarning}</small> : null}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
