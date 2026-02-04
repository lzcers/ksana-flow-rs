/**
 * FlowEvent 处理器入口
 * 从各个子模块重新导出所有处理器
 */

import { produce } from 'immer';
import type { Immutable } from 'immer';
import type { FlowEventState } from '../flowEventModel';

// ===== 事件处理器 =====
export {
    processNodeMsgEvent,
} from './nodeMsgEventProcessor';

export {
    processNodeStatusEvent,
} from './nodeStatusEventProcessor';

export {
    processControlEvent,
} from './controlEventProcessor';

// ===== Run Management Processors =====

import type {
    SetCurrentRunCommand,
    UpdateWorkflowStatusCommand,
    MapRunToWorkflowCommand,
    UnmapRunCommand,
} from '../commands';

export const processSetCurrentRun = (
    state: Immutable<FlowEventState>,
    command: SetCurrentRunCommand
): Immutable<FlowEventState> => {
    const { runId, workflowId } = command.payload;

    return produce(state, (draft) => {
        draft.currentRunId = runId;
        draft.currentWorkflowId = workflowId;

        if (runId && workflowId != null) {
            draft.runIdToWorkflowId[runId] = workflowId;
            draft.workflowStatuses[workflowId] = 'running';
            draft.workflowStatus = 'running';
        }
    });
};

export const processUpdateWorkflowStatus = (
    state: Immutable<FlowEventState>,
    command: UpdateWorkflowStatusCommand
): Immutable<FlowEventState> => {
    const { workflowId, status } = command.payload;

    return produce(state, (draft) => {
        draft.workflowStatuses[workflowId] = status;

        // 如果是当前 workflow，同步更新 workflowStatus
        if (workflowId === draft.currentWorkflowId) {
            draft.workflowStatus = status;
        }
    });
};

export const processMapRunToWorkflow = (
    state: Immutable<FlowEventState>,
    command: MapRunToWorkflowCommand
): Immutable<FlowEventState> => {
    const { runId, workflowId } = command.payload;

    return produce(state, (draft) => {
        draft.runIdToWorkflowId[runId] = workflowId;
    });
};

export const processUnmapRun = (
    state: Immutable<FlowEventState>,
    command: UnmapRunCommand
): Immutable<FlowEventState> => {
    const { runId } = command.payload;

    return produce(state, (draft) => {
        delete draft.runIdToWorkflowId[runId];

        // 如果是 currentRunId，清空它
        if (draft.currentRunId === runId) {
            draft.currentRunId = null;
        }
    });
};

// ===== Node Update Processors =====

import type {
    UpdateNodeExecutionDataCommand,
    BatchUpdateNodeDataCommand,
    ClearPendingUpdatesCommand,
} from '../commands';

export const processUpdateNodeExecutionData = (
    state: Immutable<FlowEventState>,
    command: UpdateNodeExecutionDataCommand
): Immutable<FlowEventState> => {
    const { nodeId, data } = command.payload;

    return produce(state, (draft) => {
        const existing = draft.pendingNodeUpdates.get(nodeId) ?? {};
        draft.pendingNodeUpdates.set(nodeId, { ...existing, ...data });
    });
};

export const processBatchUpdateNodeData = (
    state: Immutable<FlowEventState>,
    command: BatchUpdateNodeDataCommand
): Immutable<FlowEventState> => {
    const { updates } = command.payload;

    return produce(state, (draft) => {
        updates.forEach(({ nodeId, data }) => {
            const existing = draft.pendingNodeUpdates.get(nodeId) ?? {};
            draft.pendingNodeUpdates.set(nodeId, { ...existing, ...data });
        });
    });
};

export const processClearPendingUpdates = (
    state: Immutable<FlowEventState>,
    _command: ClearPendingUpdatesCommand
): Immutable<FlowEventState> => {
    return produce(state, (draft) => {
        draft.pendingNodeUpdates.clear();
    });
};

// ===== Run Node Execution Processors =====

import type {
    SetActiveRunContextCommand,
    ClearActiveRunContextCommand,
} from '../commands';

export const processSetActiveRunContext = (
    state: Immutable<FlowEventState>,
    command: SetActiveRunContextCommand
): Immutable<FlowEventState> => {
    const payload = command.payload;

    return produce(state, (draft) => {
        draft.activeRunContext = payload;
    });
};

export const processClearActiveRunContext = (
    state: Immutable<FlowEventState>,
    _command: ClearActiveRunContextCommand
): Immutable<FlowEventState> => {
    return produce(state, (draft) => {
        draft.activeRunContext = null;
    });
};

// ===== Meta Processors =====

import type { ResetFlowEventStateCommand } from '../commands';

export const processResetFlowEventState = (
    _state: Immutable<FlowEventState>,
    _command: ResetFlowEventStateCommand
): Immutable<FlowEventState> => {
    return {
        currentRunId: null,
        currentWorkflowId: null,
        workflowStatus: 'idle',
        workflowStatuses: {},
        runIdToWorkflowId: {},
        pendingNodeUpdates: new Map(),
        activeRunContext: null,
    };
};
