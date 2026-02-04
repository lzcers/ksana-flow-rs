/**
 * 流程控制事件处理器
 * 处理: FlowPaused, FlowResumed, FlowStopped, FlowFinished
 */

import { produce } from 'immer';
import type { Immutable } from 'immer';
import type { FlowEventState } from '../flowEventModel';
import type { ProcessControlEventCommand } from '../commands';

/**
 * 处理流程控制事件 (FlowControlEvent)
 * 包括: FlowPaused, FlowResumed, FlowStopped, FlowFinished
 */
export const processControlEvent = (
  state: Immutable<FlowEventState>,
  command: ProcessControlEventCommand
): Immutable<FlowEventState> => {
  const { event } = command.payload;

  return produce(state, (draft) => {
    const { type, runId } = event;
    const workflowId = draft.runIdToWorkflowId[runId];

    switch (type) {
      case 'FlowFinished':
      case 'FlowStopped':
        if (workflowId != null) {
          draft.workflowStatuses[workflowId] = 'idle';
        }
        if (runId === draft.currentRunId) {
          draft.workflowStatus = 'idle';
          draft.currentRunId = null;
        }
        delete draft.runIdToWorkflowId[runId];
        break;

      case 'FlowPaused':
        if (workflowId != null) {
          draft.workflowStatuses[workflowId] = 'paused';
        }
        if (runId === draft.currentRunId) {
          draft.workflowStatus = 'paused';
        }
        break;

      case 'FlowResumed':
        if (workflowId != null) {
          draft.workflowStatuses[workflowId] = 'running';
        }
        if (runId === draft.currentRunId) {
          draft.workflowStatus = 'running';
        }
        break;
    }
  });
};
