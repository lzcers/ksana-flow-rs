import type { StateCreator } from 'zustand';
import type { StoreState, Toast } from './types';

export const createToast: StateCreator<StoreState, [], [], Toast> = (set, get) => ({
  toasts: [],
  showToast: (message, type, duration = 3000) => {
    const id = Math.random().toString(36).substring(2, 9);
    set((state) => ({
      toasts: [...state.toasts, { id, message, type, duration }],
    }));
  },
  removeToast: (id) => {
    set((state) => ({
      toasts: state.toasts.filter((t) => t.id !== id),
    }));
  },
  success: (message, duration) => get().showToast(message, 'success', duration),
  error: (message, duration) => get().showToast(message, 'error', duration),
  info: (message, duration) => get().showToast(message, 'info', duration),
});
