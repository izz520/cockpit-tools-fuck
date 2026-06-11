import type { ButtonHTMLAttributes, ReactNode } from 'react';
import './Button.css';

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  icon: ReactNode;
  active?: boolean;
}

export function IconButton({ label, icon, active = false, className = '', ...props }: IconButtonProps) {
  return (
    <button
      aria-label={label}
      className={`icon-button ${active ? 'active' : ''} ${className}`}
      title={label}
      type="button"
      {...props}
    >
      {icon}
    </button>
  );
}
