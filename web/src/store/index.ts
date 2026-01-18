import { create } from 'zustand';
import type { StoreState } from './types';
import { createWorkflowSlice } from './createWorkflowSlice';
import { createCanvasSlice } from './createCanvasSlice';
import { createExecutionSlice } from './createExecutionSlice';

export const useStore = create<StoreState>((set, get, store) => ({
  ...createWorkflowSlice(set, get, store),
  ...createCanvasSlice(set, get, store),
  ...createExecutionSlice(set, get, store),

  notify: () => { },
  setNotificationHandler: (handler) => set({ notify: handler }),
}));
