type FlowNodeMsgEvent = {
    type: 'NodeError' | 'NodeInMessage' | 'NodeOutMessage' | 'NodeStreamNextMessage';
    nodeId: string;
    msg: any;
}
type FlowNodeStatusEvent = {
    type: 'NodeStarted' | 'NodeStreamStarted' | 'NodeCompleted';
    nodeId: string;
}

type FlowControlEvent = {
    type: 'FlowPaused' | 'FlowResumed' | 'FlowStopped' | 'FlowFinished';
    runId: string;
}

type FlowEvent = FlowNodeMsgEvent | FlowNodeStatusEvent | FlowControlEvent

export interface WebSocketFlowMessage {
    runId?: string;
    event: FlowEvent;
}