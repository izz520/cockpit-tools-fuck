import type { ReactNode } from 'react';
import './Panel.css';

interface PanelProps {
  children: ReactNode;
  className?: string;
}

export function Panel({ children, className = '' }: PanelProps) {
  return <section className={`panel ${className}`}>{children}</section>;
}
