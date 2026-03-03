import { produce, type Immutable } from 'immer';
import type { WorkflowState, EdgeChange } from '../types';
import type {
  UpdateEdgesCommand,
  SetEdgesCommand,
} from '../commands';
import { addEdge as addEdgeXyflow } from '@xyflow/react';
import { applyEdgeChangesXyflow } from '../utils';
import { validateConnection } from '../utils/connection';


export const processUpdateEdges = (
  state: Immutable<WorkflowState>,
  command: UpdateEdgesCommand
): Immutable<WorkflowState> => {
  const { add, remove, update, changes, connect } = command.payload;

  return produce(state, (draft) => {
    // 处理连接
    if (connect) {
      // 验证连接并获取边数据
      const validation = validateConnection(connect, draft.nodes as any);

      if (validation.valid) {
        // 使用验证后的边数据创建边
        const edgeWithData = {
          ...connect,
          data: validation.edgeData || {},
        };
        draft.edges = addEdgeXyflow(edgeWithData, draft.edges);
      } else {
        // 连接无效，记录警告但不阻止（或可以选择静默忽略）
        console.warn(`连接验证失败: ${validation.error}`);
        // 仍然创建边，但不包含端口信息（向后兼容）
        draft.edges = addEdgeXyflow(connect, draft.edges);
      }
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
