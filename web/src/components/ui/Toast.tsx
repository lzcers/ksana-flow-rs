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
    success: <CheckCircle className="w-5 h-5 text-green-500" />,
    error: <AlertCircle className="w-5 h-5 text-red-500" />,
    info: <Info className="w-5 h-5 text-blue-500" />,
  };

  const styles = {
    success: 'bg-white border-green-100 shadow-green-50/50',
    error: 'bg-white border-red-100 shadow-red-50/50',
    info: 'bg-white border-blue-100 shadow-blue-50/50',
  };

  return (
    <div
      className={cn(
        "flex items-center gap-3 px-4 py-3 rounded-lg shadow-lg border transition-all duration-300 animate-in slide-in-from-top-2 fade-in",
        styles[type]
      )}
      role="alert"
    >
      <div className="flex-shrink-0">{icons[type]}</div>
      <p className="text-sm font-medium text-slate-700">{message}</p>
      <button
        onClick={() => onClose(id)}
        className="ml-auto text-slate-400 hover:text-slate-600 transition-colors"
      >
        <X size={16} />
      </button>
    </div>
  );
};
