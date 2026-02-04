/**
 * Edge 处理器函数
 * 纯函数，接收 state 和 command，返回新的 state
 */

import { produce, type Immutable } from 'immer';
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
  state: Immutable<WorkflowState>,
  command: AddEdgeCommand
): Immutable<WorkflowState> => {
  const { edge } = command.payload;

  return produce(state, (draft) => {
    draft.edges.push(edge);
  });
};

export const processRemoveEdge = (
  state: Immutable<WorkflowState>,
  command: RemoveEdgeCommand
): Immutable<WorkflowState> => {
  const { id } = command.payload;

  return produce(state, (draft) => {
    draft.edges = draft.edges.filter((e) => e.id !== id);
  });
};

export const processOnConnect = (
  state: Immutable<WorkflowState>,
  command: OnConnectCommand
): Immutable<WorkflowState> => {
  const connection = command.payload;

  return produce(state, (draft) => {
    draft.edges = addEdgeXyflow(connection, draft.edges);
  });
};

export const processUpdateEdge = (
  state: Immutable<WorkflowState>,
  command: UpdateEdgeCommand
): Immutable<WorkflowState> => {
  const { id, updates } = command.payload;

  return produce(state, (draft) => {
    const edge = draft.edges.find((e) => e.id === id);
    if (edge) {
      Object.assign(edge, updates);
    }
  });
};

export const processSetEdges = (
  state: Immutable<WorkflowState>,
  command: SetEdgesCommand
): Immutable<WorkflowState> => {
  const { edges } = command.payload;

  return produce(state, (draft) => {
    draft.edges = edges;
  });
};

export const processApplyEdgeChanges = (
  state: Immutable<WorkflowState>,
  command: ApplyEdgeChangesCommand
): Immutable<WorkflowState> => {
  const { changes } = command.payload;

  return produce(state, (draft) => {
    const updatedEdges = applyEdgeChangesXyflow(changes as EdgeChange[], draft.edges);
    draft.edges = updatedEdges;
  });
};
