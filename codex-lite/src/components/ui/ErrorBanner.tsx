import { AlertTriangle } from 'lucide-react';
import type { AppError } from '../../types/system';
import './ErrorBanner.css';

interface ErrorBannerProps {
  error: AppError;
}

export function ErrorBanner({ error }: ErrorBannerProps) {
  return (
    <section className="error-banner" role="alert">
      <AlertTriangle size={18} />
      <div>
        <strong>{error.message}</strong>
        <p>{error.action}</p>
      </div>
      <code>{error.code}</code>
    </section>
  );
}
