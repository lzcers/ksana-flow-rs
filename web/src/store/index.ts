import { create, type StoreApi } from 'zustand';
import type { Immutable } from 'immer';
import type { StoreState } from './types';
import { createWorkflow } from './createWorkflow';
import { createCanvas } from './createCanvas';
import { createExecution } from './createExecution';
import { createToast } from './createToast';
import { createWorkflowModel } from '../model/workflow';
import type { WorkflowState } from '../model/workflow/types';

// Workflow Model 实例
export const rxWorkflowModel = createWorkflowModel();


export const useStore = create<StoreState>((set, get, store) => ({
  ...createWorkflow(set, get, store),
  ...createCanvas(set, get, store),
  ...createExecution(set, get, store),
  ...createToast(set, get, store),
}));


// 连接 WorkflowModel 到 Store
let detachWorkflow: (() => void) | null = null;
export function attachWorkflowModelToStore(storeApi: StoreApi<StoreState>): () => void {
  if (detachWorkflow) return detachWorkflow;

  const subscription = rxWorkflowModel.viewState$.subscribe((state: Immutable<WorkflowState>) => {
    storeApi.setState({
      nodes: state.nodes,
      edges: state.edges,
      selectedNodeId: state.selectedNodeId,
    } as Partial<StoreState>);
  });

  detachWorkflow = () => {
    subscription.unsubscribe();
    detachWorkflow = null;
  };

  return detachWorkflow;
}

// 初始化连接
attachWorkflowModelToStore(useStore);
