import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from 'react';
import './Button.css';

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
  icon: ReactNode;
  active?: boolean;
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(
  ({ label, icon, active = false, className = '', ...props }, ref) => {
    return (
      <button
        ref={ref}
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
);

IconButton.displayName = 'IconButton';
