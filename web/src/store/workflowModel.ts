import type { StoreApi } from 'zustand';
import { createWorkflowModel } from '../model';
import type { WorkflowState } from '../model/types';
import type { StoreState } from './types';

export const workflowModel = createWorkflowModel();

let detach: (() => void) | null = null;

export function attachWorkflowModelToStore(storeApi: StoreApi<StoreState>): () => void {
  if (detach) return detach;

  const subscription = workflowModel.viewState$.subscribe((state: WorkflowState) => {
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
