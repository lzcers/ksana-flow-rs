/**
 * FlowEvent Module
 * 提供 FlowEvent 的流式处理和状态管理
 */

// 类型定义
export * from './types';

// WebSocket
export * from './socket';

// Core Model
export {
  FlowEventModel,
  type FlowEventState,
  type FlowEventModelOptions,
  type FlowEventProcessor,
} from './flowEventModel';

// Commands
export {
  type FlowEventCommand,
  type NodeExecutionData,
  type ProcessFlowEventCommand,
  type SetCurrentRunCommand,
  type UpdateWorkflowStatusCommand,
  type MapRunToWorkflowCommand,
  type UnmapRunCommand,
  type UpdateNodeExecutionDataCommand,
  type BatchUpdateNodeDataCommand,
  type ClearPendingUpdatesCommand,
  type SetActiveRunContextCommand,
  type ClearActiveRunContextCommand,
  type ResetFlowEventStateCommand,
} from './commands';

// Processors
export * from './processors';

// Reactive Layer
export {
  RxFlowEvent,
  type RxFlowEventOptions,
} from './flowEventRx';

// Factory function to create RxFlowEvent instance
import { RxFlowEvent } from './flowEventRx';

export function createFlowEventModel(): RxFlowEvent {
  return new RxFlowEvent();
}
