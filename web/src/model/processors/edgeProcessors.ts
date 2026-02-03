/**
 * Edge 处理器函数
 * 纯函数，接收 state 和 command，返回新的 state
 */

import { produce } from 'immer';
import type { WorkflowState, EdgeChange } from '../types';
import type {
  AddEdgeCommand,
  RemoveEdgeCommand,
  OnConnectCommand,
  UpdateEdgeCommand,
  SetEdgesCommand,
  ApplyEdgeChangesCommand,
} from '../commands';
import { addEdge as addEdgeXyflow } from '@xyflow/react';
import { applyEdgeChangesXyflow } from '../utils';

// ===== 处理器函数 =====

export const processAddEdge = (
  state: WorkflowState,
  command: AddEdgeCommand
): WorkflowState => {
  const { edge } = command.payload;

  return produce(state, (draft) => {
    draft.edges.push(edge as any);
  });
};

export const processRemoveEdge = (
  state: WorkflowState,
  command: RemoveEdgeCommand
): WorkflowState => {
  const { id } = command.payload;

  return produce(state, (draft) => {
    draft.edges = draft.edges.filter((e) => e.id !== id);
  });
};

export const processOnConnect = (
  state: WorkflowState,
  command: OnConnectCommand
): WorkflowState => {
  const connection = command.payload;

  return produce(state, (draft) => {
    draft.edges = addEdgeXyflow(connection, draft.edges);
  });
};

export const processUpdateEdge = (
  state: WorkflowState,
  command: UpdateEdgeCommand
): WorkflowState => {
  const { id, updates } = command.payload;

  return produce(state, (draft) => {
    const edge = draft.edges.find((e) => e.id === id);
    if (edge) {
      Object.assign(edge, updates);
    }
  });
};

export const processSetEdges = (
  state: WorkflowState,
  command: SetEdgesCommand
): WorkflowState => {
  const { edges } = command.payload;

  return produce(state, (draft) => {
    draft.edges = edges as any[];
  });
};

export const processApplyEdgeChanges = (
  state: WorkflowState,
  command: ApplyEdgeChangesCommand
): WorkflowState => {
  const { changes } = command.payload;

  return produce(state, (draft) => {
    const updatedEdges = applyEdgeChangesXyflow(changes as EdgeChange[], draft.edges);
    draft.edges = updatedEdges;
  });
};
