/**
 * 节点消息事件处理器
 * 处理: NodeError, NodeInMessage, NodeOutMessage, NodeStreamNextMessage
 */

import { produce } from 'immer';
import type { Immutable } from 'immer';
import type { FlowEventState } from '../flowEventModel';
import type { ProcessNodeMsgEventCommand } from '../commands';

// ===== 辅助函数 =====

const isCurrentRunEvent = (
  state: FlowEventState,
  runId: string | undefined,
  activeContext: typeof state.activeRunContext
): boolean => {
  if (!runId) return true;
  if (runId === state.currentRunId) return true;
  if (activeContext && runId === activeContext.runId) return true;
  return false;
};

const getOrCreateNodeData = (
  state: FlowEventState,
  nodeId: string
): FlowEventState['pendingNodeUpdates'] extends Map<string, infer V> ? V : never => {
  const existing = state.pendingNodeUpdates.get(nodeId);
  if (existing) return existing as never;
  return {} as never;
};

/**
 * 处理节点消息事件 (FlowNodeMsgEvent)
 * 包括: NodeError, NodeInMessage, NodeOutMessage, NodeStreamNextMessage
 */
export const processNodeMsgEvent = (
  state: Immutable<FlowEventState>,
  command: ProcessNodeMsgEventCommand
): Immutable<FlowEventState> => {
  const { event, runId } = command.payload;

  // 检查是否是当前 run 的事件
  if (!isCurrentRunEvent(state as FlowEventState, runId, state.activeRunContext)) {
    return state;
  }

  return produce(state, (draft) => {
    const nodeData = getOrCreateNodeData(draft as FlowEventState, event.nodeId);

    switch (event.type) {
      case 'NodeError':
        nodeData.status = 'error';
        nodeData.errorMessage = event.msg;
        nodeData.isOutputStream = false;
        break;

      case 'NodeInMessage':
        nodeData.inputs = typeof event.msg === 'object' && event.msg !== null
          ? event.msg
          : { value: event.msg };
        break;

      case 'NodeOutMessage':
        nodeData.outputs = { output: event.msg };
        nodeData.lastMessage = event.msg;
        nodeData.isOutputStream = false;
        break;

      case 'NodeStreamNextMessage':
        // 流式消息，追加到 lastMessage
        if (!nodeData.lastMessage) {
          nodeData.lastMessage = event.msg;
        } else if (typeof nodeData.lastMessage === 'string' && typeof event.msg === 'string') {
          nodeData.lastMessage += event.msg;
        } else {
          // 如果不是字符串，用数组存储
          if (!Array.isArray(nodeData.lastMessage)) {
            nodeData.lastMessage = [nodeData.lastMessage];
          }
          (nodeData.lastMessage as any[]).push(event.msg);
        }
        break;
    }

    // 更新 pendingNodeUpdates
    if (Object.keys(nodeData).length > 0) {
      draft.pendingNodeUpdates.set(event.nodeId, nodeData);
    }
  });
};
