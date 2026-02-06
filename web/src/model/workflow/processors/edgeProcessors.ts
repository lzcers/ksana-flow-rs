import { produce, type Immutable } from 'immer';
import type { WorkflowState, EdgeChange } from '../types';
import type {
  UpdateEdgesCommand,
  SetEdgesCommand,
} from '../commands';
import { addEdge as addEdgeXyflow } from '@xyflow/react';
import { applyEdgeChangesXyflow } from '../utils';


export const processUpdateEdges = (
  state: Immutable<WorkflowState>,
  command: UpdateEdgesCommand
): Immutable<WorkflowState> => {
  const { add, remove, update, changes, connect } = command.payload;

  return produce(state, (draft) => {
    // 处理连接
    if (connect) {
      draft.edges = addEdgeXyflow(connect, draft.edges);
    }

    // 处理批量添加
    if (add && add.length > 0) {
      draft.edges.push(...add);
    }

    // 处理删除
    if (remove && remove.length > 0) {
      const removeSet = new Set(remove);
      draft.edges = draft.edges.filter((e) => !removeSet.has(e.id));
    }

    // 处理更新
    if (update && update.length > 0) {
      update.forEach(({ id, updates }) => {
        const edge = draft.edges.find((e) => e.id === id);
        if (edge) {
          Object.assign(edge, updates);
        }
      });
    }

    // 处理 changes (xyflow 格式)
    if (changes && changes.length > 0) {
      draft.edges = applyEdgeChangesXyflow(changes as EdgeChange[], draft.edges);
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
