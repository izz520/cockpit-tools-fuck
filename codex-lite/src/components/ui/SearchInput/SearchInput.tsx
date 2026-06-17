import { Search } from 'lucide-react';
import type { InputHTMLAttributes } from 'react';
import './SearchInput.css';

interface SearchInputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type' | 'onChange'> {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

export function SearchInput({ value, onChange, placeholder = 'Search...', ...props }: SearchInputProps) {
  return (
    <label className="search-input">
      <Search size={16} aria-hidden="true" />
      <input
        type="search"
        aria-label={placeholder}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        {...props}
      />
    </label>
  );
}
