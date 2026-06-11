import type { ButtonHTMLAttributes, ReactNode } from 'react';
import './Button.css';

type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant: ButtonVariant;
  icon?: ReactNode;
  loading?: boolean;
}

export function Button({ variant, icon, loading, children, className = '', disabled, ...props }: ButtonProps) {
  return (
    <button
      className={`button button-${variant} ${className}`}
      disabled={disabled || loading}
      type="button"
      {...props}
    >
      {loading ? <span className="button-spinner" aria-hidden="true" /> : icon}
      <span>{children}</span>
    </button>
  );
}
