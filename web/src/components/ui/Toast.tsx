import React, { useEffect } from 'react';
import { X, CheckCircle, AlertCircle, Info } from 'lucide-react';
import { cn } from '../../utils/cn';

export type ToastType = 'success' | 'error' | 'info';

export interface ToastProps {
  id: string;
  message: string;
  type: ToastType;
  duration?: number;
  onClose: (id: string) => void;
}

export const Toast: React.FC<ToastProps> = ({
  id,
  message,
  type,
  duration = 3000,
  onClose,
}) => {
  useEffect(() => {
    if (duration > 0) {
      const timer = setTimeout(() => {
        onClose(id);
      }, duration);
      return () => clearTimeout(timer);
    }
  }, [id, duration, onClose]);

  const icons = {
    success: <CheckCircle className="w-5 h-5 text-green-400/90" />,
    error: <AlertCircle className="w-5 h-5 text-red-400/90" />,
    info: <Info className="w-5 h-5 text-blue-400/90" />,
  };

  const styles = {
    success: 'bg-zinc-900/95 border-zinc-800 shadow-xl shadow-black/20 backdrop-blur-md',
    error: 'bg-zinc-900/95 border-zinc-800 shadow-xl shadow-black/20 backdrop-blur-md',
    info: 'bg-zinc-900/95 border-zinc-800 shadow-xl shadow-black/20 backdrop-blur-md',
  };

  return (
    <div
      className={cn(
        "flex items-center gap-3 px-4 py-3 rounded-xl border transition-all duration-300 animate-in slide-in-from-top-2 fade-in max-w-sm w-full",
        styles[type]
      )}
      role="alert"
    >
      <div className="flex-shrink-0">{icons[type]}</div>
      <p className="text-sm font-medium text-zinc-200">{message}</p>
      <button
        onClick={() => onClose(id)}
        className="ml-auto text-zinc-500 hover:text-zinc-300 transition-colors p-1 hover:bg-zinc-800 rounded-full"
      >
        <X size={14} />
      </button>
    </div>
  );
};
