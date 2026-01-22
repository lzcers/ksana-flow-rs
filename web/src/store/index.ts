import { create } from 'zustand';
import type { StoreState } from './types';
import { createWorkflow } from './createWorkflow';
import { createCanvas } from './createCanvas';
import { createExecution } from './createExecution';
import { createToast } from './createToast';

export const useStore = create<StoreState>((set, get, store) => ({
  ...createWorkflow(set, get, store),
  ...createCanvas(set, get, store),
  ...createExecution(set, get, store),
  ...createToast(set, get, store),
}));
