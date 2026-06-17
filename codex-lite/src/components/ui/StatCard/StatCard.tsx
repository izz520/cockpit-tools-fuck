import type { ReactNode } from 'react';
import './StatCard.css';

interface StatCardProps {
  icon: ReactNode;
  iconColor?: 'primary' | 'purple' | 'blue' | 'green';
  label: string;
  value: string | number;
  meta?: string;
}

export function StatCard({ icon, iconColor = 'primary', label, value, meta }: StatCardProps) {
  return (
    <article className="stat-card">
      <span className={`stat-icon stat-icon-${iconColor}`} aria-hidden="true">
        {icon}
      </span>
      <span className="stat-copy">
        <span className="stat-label">{label}</span>
        <span className="stat-value-row">
          <strong>{value}</strong>
          {meta ? <span className="stat-meta">{meta}</span> : null}
        </span>
      </span>
    </article>
  );
}
