/**
 * FlowEvent Module
 * 提供 FlowEvent 的流式处理和状态管理
 */
// 类型定义
export * from './types';
// WebSocket
export * from './socket';

import { RxFlowEvent } from './flowEventRx';

export function createFlowEventModel(): RxFlowEvent {
  return new RxFlowEvent();
}
