import { create } from 'zustand';
import type { StoreState } from './types';
import { createWorkflowSlice } from './createWorkflowSlice';
import { createCanvasSlice } from './createCanvasSlice';
import { createExecutionSlice } from './createExecutionSlice';
import { createToastSlice } from './createToastSlice';

export const useStore = create<StoreState>((set, get, store) => ({
  ...createWorkflowSlice(set, get, store),
  ...createCanvasSlice(set, get, store),
  ...createExecutionSlice(set, get, store),
  ...createToastSlice(set, get, store),
}));
