import { create, type StoreApi } from 'zustand';
import type { Immutable } from 'immer';
import type { StoreState } from './types';
import { createWorkflow } from './createWorkflow';
import { createCanvas } from './createCanvas';
import { createExecution } from './createExecution';
import { createToast } from './createToast';
import { createWorkflowModel } from '../model/workflow';
import { createFlowEventModel } from '../model/flowEvent';
import type { WorkflowState } from '../model/workflow/types';
import type { GraphCommand } from '../model/workflow/commands';
import type { NodeExecutionData } from '../model/flowEvent/commands';

export const useStore = create<StoreState>((set, get, store) => ({
  ...createWorkflow(set, get, store),
  ...createCanvas(set, get, store),
  ...createExecution(set, get, store),
  ...createToast(set, get, store),
}));

// Workflow Model 实例
export const rxWorkflowModel = createWorkflowModel();

// FlowEvent Model 实例
export const rxFlowEventModel = createFlowEventModel();

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

// 连接 FlowEventModel 到 Store
let detachFlowEvent: (() => void) | null = null;
export function attachFlowEventModelToStore(storeApi: StoreApi<StoreState>): () => void {
  if (detachFlowEvent) return detachFlowEvent;

  // 订阅 FlowEvent 状态变化，同步到 zustand
  const subscription = rxFlowEventModel.state$.subscribe((state: Immutable<{
    currentRunId: string | null;
    currentWorkflowId: number | null;
    workflowStatus: import('./types').WorkflowStatus;
    workflowStatuses: Record<number, import('./types').WorkflowStatus>;
    runIdToWorkflowId: Record<string, number>;
  }>) => {
    storeApi.setState({
      workflowStatus: state.workflowStatus,
      workflowStatuses: state.workflowStatuses,
      currentRunId: state.currentRunId,
      runIdToWorkflowId: state.runIdToWorkflowId,
    } as Partial<StoreState>);
  });

  // 订阅批量节点更新，转换为 WorkflowModel Commands
  const batchSubscription = rxFlowEventModel.batchedNodeUpdates$.subscribe((updates: Map<string, NodeExecutionData>) => {
    const commands: GraphCommand[] = [];

    updates.forEach((data: NodeExecutionData, nodeId: string) => {
      // 将 NodeExecutionData 转换为 NodeData
      commands.push({
        type: 'UPDATE_NODE_DATA',
        payload: {
          id: nodeId,
          data: {
            ...data,
            // 确保 status 类型兼容
            status: data.status as 'idle' | 'running' | 'completed' | 'error',
          }
        }
      });
    });

    if (commands.length > 0) {
      rxWorkflowModel.dispatch({
        type: 'BATCH',
        payload: { commands }
      });
    }

    // 清空待处理更新
    rxFlowEventModel.clearPendingUpdates();
  });

  detachFlowEvent = () => {
    subscription.unsubscribe();
    batchSubscription.unsubscribe();
    detachFlowEvent = null;
  };

  return detachFlowEvent;
}

// 初始化连接
attachWorkflowModelToStore(useStore);
attachFlowEventModelToStore(useStore);
