export type FlowNodeMsgEventType = 'NodeError' | 'NodeInMessage' | 'NodeOutMessage' | 'NodeStreamNextMessage';
export type FlowNodeStatusEventType = 'NodeStarted' | 'NodeStreamStarted' | 'NodeCompleted';
export type FlowControlEventType = 'FlowPaused' | 'FlowResumed' | 'FlowStopped' | 'FlowFinished';

export interface FlowNodeMsgEvent {
  type: FlowNodeMsgEventType;
  nodeId: string;
  msg: any;
}

export interface FlowNodeStatusEvent {
  type: FlowNodeStatusEventType;
  nodeId: string;
}

export interface FlowControlEvent {
  type: FlowControlEventType;
  runId: string;
}

export type FlowEvent = FlowNodeMsgEvent | FlowNodeStatusEvent | FlowControlEvent;

export interface WebSocketFlowMessage {
  runId?: string;
  event: FlowEvent;
}

/**
 * 节点执行数据
 * 用于存储节点的运行时状态
 */
export interface NodeExecutionData {
  status?: 'idle' | 'running' | 'completed' | 'error';
  lastMessage?: any;
  lastMessageRunId?: string;
  isOutputStream?: boolean;
  upstreamIsStreaming?: boolean;
  errorMessage?: string;
  inputs?: Record<string, any>;
  outputs?: Record<string, any>;
}
