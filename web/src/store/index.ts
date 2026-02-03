import { create, type StoreApi } from 'zustand';
import type { Immutable } from 'immer';
import type { StoreState } from './types';
import { createWorkflow } from './createWorkflow';
import { createCanvas } from './createCanvas';
import { createExecution } from './createExecution';
import { createToast } from './createToast';
import { createWorkflowModel } from '../model';
import type { WorkflowState } from '../model/types';

export const useStore = create<StoreState>((set, get, store) => ({
  ...createWorkflow(set, get, store),
  ...createCanvas(set, get, store),
  ...createExecution(set, get, store),
  ...createToast(set, get, store),
}));


export const workflowModel = createWorkflowModel();

let detach: (() => void) | null = null;

export function attachWorkflowModelToStore(storeApi: StoreApi<StoreState>): () => void {
  if (detach) return detach;

  const subscription = workflowModel.viewState$.subscribe((state: Immutable<WorkflowState>) => {
    storeApi.setState({
      nodes: state.nodes,
      edges: state.edges,
      selectedNodeId: state.selectedNodeId,
    } as Partial<StoreState>);
  });

  detach = () => {
    subscription.unsubscribe();
    detach = null;
  };

  return detach;
}
attachWorkflowModelToStore(useStore);