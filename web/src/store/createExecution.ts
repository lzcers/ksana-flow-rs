import type { StateCreator, StoreApi } from 'zustand';
import { BehaviorSubject, Subject, interval, animationFrameScheduler, bufferWhen, filter, map, merge, share, timer, withLatestFrom } from 'rxjs';
import { produce } from 'immer';
import type { StoreState, Execution, WebSocketFlowMessage } from './types';
import * as api from '../api';
import { resetWorkflowExecutionState } from '../model';

const eventSubject = new Subject<WebSocketFlowMessage>();
const stateUpdateSubject = new Subject<WebSocketFlowMessage>();
const currentRunIdSubject = new BehaviorSubject<string | null>(null);
const eventsForCurrentRun$ = eventSubject.pipe(
  withLatestFrom(currentRunIdSubject),
  filter(([wrapper, currentRunId]) => !wrapper.runId || !currentRunId || wrapper.runId === currentRunId),
  map(([wrapper]) => wrapper),
  share()
);
type ZustandSetState = StoreApi<StoreState>['setState'];

let stateUpdatePipelineInitialized = false;
let latestSetState: ZustandSetState | null = null;
let activeRunNodeExecution: { runId: string; startNodeId: string; workflowId: number | null } | null = null;

const buildExecutionBlueprint = (nodes: any[], edges: any[]) => ({
  nodes: nodes.map((n) => ({
    id: n.id,
    type: n.type,
    data: n.data,
    position: n.position,
    width: typeof n.style?.width === 'number' ? n.style.width : (typeof n.style?.width === 'string' ? parseFloat(n.style.width) : (n.width ?? n.measured?.width)),
    height: typeof n.style?.height === 'number' ? n.style.height : (typeof n.style?.height === 'string' ? parseFloat(n.style.height) : (n.height ?? n.measured?.height)),
    parentId: n.parentId,
    extent: n.extent,
    hidden: n.hidden,
  })),
  edges: edges.filter((e: any) => !e?.data?.__uiSubgraphEdge).map((e) => ({
    id: e.id,
    source: e.source,
    target: e.target,
    sourceHandle: e.sourceHandle,
    targetHandle: e.targetHandle,
    type: e.type,
  })),
});

export const createExecution: StateCreator<StoreState, [], [], Execution> = (set, get) => {
  latestSetState = set;

  if (!stateUpdatePipelineInitialized) {
    stateUpdatePipelineInitialized = true;

    stateUpdateSubject
      .pipe(
        bufferWhen(() => merge(interval(0, animationFrameScheduler), timer(16))),
        filter((batch) => batch.length > 0)
      )
      .subscribe((batch) => {
        const setState = latestSetState;
        if (!setState) return;

        let finalizeRunNodeRunId: string | null = null;
        let nextCurrentRunId: string | null | undefined;
        setState((state) => {
          return produce(state, (draft) => {
            const nodeIndex = new Map<string, any>();
            for (const node of draft.nodes) nodeIndex.set(node.id, node);

            const outEdgeMap = new Map<string, { target: string; targetHandle?: string | null }[]>();
            for (const e of draft.edges) {
              const arr = outEdgeMap.get(e.source);
              const info = { target: e.target, targetHandle: e.targetHandle };
              if (arr) arr.push(info);
              else outEdgeMap.set(e.source, [info]);
            }

            const getNodeData = (nodeId: string) => {
              const node = nodeIndex.get(nodeId);
              if (!node) return null;
              if (!node.data) node.data = {};
              return node.data as any;
            };

            const setInput = (nodeId: string, key: string, value: any) => {
              const data = getNodeData(nodeId);
              if (!data) return;
              if (!data.inputs) data.inputs = {};
              if (data.inputs[key] !== value) data.inputs[key] = value;
            };

            const setOutput = (nodeId: string, key: string, value: any) => {
              const data = getNodeData(nodeId);
              if (!data) return;
              if (!data.outputs) data.outputs = {};
              if (data.outputs[key] !== value) data.outputs[key] = value;
            };

            for (const wrapper of batch) {
              const { runId, event: msg } = wrapper;

              let isCurrentRun = !runId || runId === draft.currentRunId;
              if (
                !isCurrentRun &&
                runId &&
                draft.currentRunId === null &&
                draft.workflowStatus === 'running'
              ) {
                draft.currentRunId = runId;
                nextCurrentRunId = runId;
                const wfId = draft.currentWorkflowId;
                if (wfId != null) {
                  draft.runIdToWorkflowId[runId] = wfId;
                  if (draft.workflowStatuses[wfId] !== 'running') {
                    draft.workflowStatuses[wfId] = 'running';
                  }
                }
                isCurrentRun = true;
              }

              if (isCurrentRun) {
                if (typeof msg === 'object') {
                  if ('NodeStarted' in msg) {
                    const id = msg.NodeStarted;
                    const data = getNodeData(id);
                    if (data && data.status !== 'running') data.status = 'running';
                  } else if ('NodeStreamStarted' in msg) {
                    const id = msg.NodeStreamStarted;
                    const data = getNodeData(id);
                    if (data && data.isOutputStream !== true) data.isOutputStream = true;

                    const outEdges = outEdgeMap.get(id) ?? [];
                    for (const edge of outEdges) {
                      const targetData = getNodeData(edge.target);
                      if (targetData && targetData.upstreamIsStreaming !== true) {
                        targetData.upstreamIsStreaming = true;
                      }
                    }
                  } else if ('NodeStreamNextMessage' in msg) {
                  } else if ('NodeInMessage' in msg) {
                    const [id, value] = msg.NodeInMessage;
                    const data = getNodeData(id);
                    if (data) {
                      if (data.lastMessage !== value) data.lastMessage = value;
                      if (data.lastMessageRunId !== runId) data.lastMessageRunId = runId;
                    }
                    if (typeof value === 'object' && value !== null) {
                      for (const [k, v] of Object.entries(value)) {
                        setInput(id, k, v);
                      }
                    }
                  } else if ('NodeOutMessage' in msg) {
                    const [id, value] = msg.NodeOutMessage;
                    const data = getNodeData(id);
                    if (data) {
                      if (data.lastMessage !== value) data.lastMessage = value;
                      if (data.lastMessageRunId !== runId) data.lastMessageRunId = runId;
                      if (data.isOutputStream !== false) data.isOutputStream = false;
                    }
                    setOutput(id, 'output', value);

                    const outEdges = outEdgeMap.get(id) ?? [];
                    for (const edge of outEdges) {
                      const targetData = getNodeData(edge.target);
                      if (targetData) {
                        if (targetData.lastMessage !== value) targetData.lastMessage = value;
                        if (targetData.lastMessageRunId !== runId) targetData.lastMessageRunId = runId;
                        if (targetData.upstreamIsStreaming !== false) targetData.upstreamIsStreaming = false;
                      }
                      setInput(edge.target, edge.targetHandle || 'default', value);
                    }
                  } else if ('NodeCompleted' in msg) {
                    const id = msg.NodeCompleted;
                    const data = getNodeData(id);
                    if (data && data.status !== 'completed') data.status = 'completed';

                    if (
                      runId &&
                      activeRunNodeExecution &&
                      activeRunNodeExecution.runId === runId &&
                      activeRunNodeExecution.startNodeId === id
                    ) {
                      const outEdges = outEdgeMap.get(id) ?? [];
                      if (outEdges.length === 0) {
                        const wfId = activeRunNodeExecution.workflowId ?? draft.currentWorkflowId;
                        if (wfId != null) {
                          if (draft.workflowStatuses[wfId] !== 'idle') draft.workflowStatuses[wfId] = 'idle';
                        }

                        if (draft.workflowStatus !== 'idle') draft.workflowStatus = 'idle';
                        if (draft.currentRunId !== null) {
                          draft.currentRunId = null;
                          nextCurrentRunId = null;
                        }
                        if (runId in draft.runIdToWorkflowId) delete draft.runIdToWorkflowId[runId];
                        finalizeRunNodeRunId = runId;
                      }
                    }
                  } else if ('NodeError' in msg) {
                    const [id, error] = msg.NodeError;
                    const data = getNodeData(id);
                    if (data) {
                      if (data.status !== 'error') data.status = 'error';
                      if (data.errorMessage !== error) data.errorMessage = error;
                      if (data.isOutputStream !== false) data.isOutputStream = false;
                    }
                  }
                }
              }

              const workflowId =
                (runId ? draft.runIdToWorkflowId[runId] : null) ??
                (runId && runId === draft.currentRunId ? draft.currentWorkflowId : null);

              if (msg === 'FlowFinished' || msg === 'FlowStopped') {
                if (workflowId != null) {
                  if (draft.workflowStatuses[workflowId] !== 'idle') draft.workflowStatuses[workflowId] = 'idle';
                }
                if (runId && runId in draft.runIdToWorkflowId) delete draft.runIdToWorkflowId[runId];

                if (!runId || runId === draft.currentRunId) {
                  if (draft.workflowStatus !== 'idle') draft.workflowStatus = 'idle';
                  if (draft.currentRunId !== null) {
                    draft.currentRunId = null;
                    nextCurrentRunId = null;
                  }
                }
              } else if (msg === 'FlowPaused') {
                if (workflowId != null) {
                  if (draft.workflowStatuses[workflowId] !== 'paused') draft.workflowStatuses[workflowId] = 'paused';
                }
                if (!runId || runId === draft.currentRunId) {
                  if (draft.workflowStatus !== 'paused') draft.workflowStatus = 'paused';
                }
              } else if (msg === 'FlowResumed') {
                if (workflowId != null) {
                  if (draft.workflowStatuses[workflowId] !== 'running') draft.workflowStatuses[workflowId] = 'running';
                }
                if (!runId || runId === draft.currentRunId) {
                  if (draft.workflowStatus !== 'running') draft.workflowStatus = 'running';
                }
              }
            }
          });
        });
        if (nextCurrentRunId !== undefined) currentRunIdSubject.next(nextCurrentRunId);
        if (finalizeRunNodeRunId && activeRunNodeExecution?.runId === finalizeRunNodeRunId) {
          activeRunNodeExecution = null;
        }
      });
  }

  const isEventForNode = (event: any, nodeId: string) => {
    if (!event || typeof event !== 'object') return false;
    if ('NodeStarted' in event) return event.NodeStarted === nodeId;
    if ('NodeCompleted' in event) return event.NodeCompleted === nodeId;
    if ('NodeStreamStarted' in event) return event.NodeStreamStarted === nodeId;
    if ('NodeError' in event) return Array.isArray(event.NodeError) && event.NodeError[0] === nodeId;
    if ('NodeInMessage' in event) return Array.isArray(event.NodeInMessage) && event.NodeInMessage[0] === nodeId;
    if ('NodeOutMessage' in event) return Array.isArray(event.NodeOutMessage) && event.NodeOutMessage[0] === nodeId;
    if ('NodeStreamNextMessage' in event) return Array.isArray(event.NodeStreamNextMessage) && event.NodeStreamNextMessage[0] === nodeId;
    return false;
  };

  return {
    workflowStatus: 'idle',
    workflowStatuses: {},
    runIdToWorkflowId: {},
    currentRunId: null,
    events$: eventSubject.asObservable(),
    eventsForCurrentRun$,
    eventsForNode$: (nodeId: string) => eventsForCurrentRun$.pipe(filter((wrapper) => isEventForNode(wrapper.event, nodeId))),

    setWorkflowStatus: (status) => set({ workflowStatus: status }),
    setWorkflowStatuses: (statuses) => set({ workflowStatuses: statuses }),
    setCurrentRunId: (currentRunId) => {
      currentRunIdSubject.next(currentRunId);
      set({ currentRunId });
    },

    initializeWebSocket: () => {
      let ws: WebSocket | null = null;
      let reconnectTimeout: number | null = null;

      const connect = () => {
        const { currentSpaceId } = get();
        if (!currentSpaceId) return;

        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const host = import.meta.env.PROD ? window.location.host : 'localhost:3000';
        ws = new WebSocket(`${protocol}//${host}/ws?workspace_id=${currentSpaceId}`);

        ws.onmessage = (event) => {
          try {
            const wrapper = JSON.parse(event.data);
            const { handleWebSocketMessage } = get();
            handleWebSocketMessage(wrapper);
          } catch (e) {
            console.error("WS parse error", e);
          }
        };

        ws.onclose = () => {
          console.log('WS closed, reconnecting...');
          reconnectTimeout = window.setTimeout(connect, 2000);
        };

        ws.onerror = (err) => {
          console.error('WS error', err);
          ws?.close();
        };
      };

      connect();

      return () => {
        if (ws) ws.close();
        if (reconnectTimeout) clearTimeout(reconnectTimeout);
      };
    },

    handleWebSocketMessage: (wrapper: WebSocketFlowMessage) => {
      eventSubject.next(wrapper);
      stateUpdateSubject.next(wrapper);
    },

    runWorkflow: async () => {
      const { currentSpaceId, nodes, edges, currentWorkflowId, success, error, setWorkflowStatus, setCurrentRunId, setWorkflowStatuses } = get();
      if (!currentSpaceId) return;

      const blueprint = buildExecutionBlueprint(nodes, edges);

      try {
        set(state => ({ ...state, ...resetWorkflowExecutionState(state) }));
        setWorkflowStatus('running');

        const res = await api.runWorkflow(currentSpaceId, blueprint, currentWorkflowId || -1);
        if (res && res.error) {
          throw new Error(res.error);
        }
        if (res && res.run_id) {
          setCurrentRunId(res.run_id);
          if (currentWorkflowId != null) {
            setWorkflowStatuses({ ...get().workflowStatuses, [currentWorkflowId]: 'running' });
            set(state => ({ runIdToWorkflowId: { ...state.runIdToWorkflowId, [res.run_id]: currentWorkflowId } }));
          }
        }
        success('Workflow started');
      } catch (e) {
        console.error("Failed to run workflow", e);
        error('Failed to run workflow: ' + (e instanceof Error ? e.message : String(e)));
        setWorkflowStatus('idle');
        setCurrentRunId(null);
        if (currentWorkflowId != null) {
          setWorkflowStatuses({ ...get().workflowStatuses, [currentWorkflowId]: 'idle' });
        }
      }
    },

    pauseWorkflow: async () => {
      const { currentSpaceId, currentRunId, error } = get();
      if (!currentRunId || !currentSpaceId) return;
      try {
        await api.pauseWorkflow(currentSpaceId, currentRunId);
      } catch (e) {
        console.error("Failed to pause workflow", e);
        error("Failed to pause workflow");
      }
    },

    resumeWorkflow: async () => {
      const { currentSpaceId, currentRunId, error } = get();
      if (!currentRunId || !currentSpaceId) return;
      try {
        await api.resumeWorkflow(currentSpaceId, currentRunId);
      } catch (e) {
        console.error("Failed to resume workflow", e);
        error("Failed to resume workflow");
      }
    },

    stopWorkflow: async () => {
      const { currentSpaceId, currentRunId, error } = get();
      if (!currentRunId || !currentSpaceId) return;
      try {
        await api.stopWorkflow(currentSpaceId, currentRunId);
      } catch (e) {
        console.error("Failed to stop workflow", e);
        error("Failed to stop workflow");
      }
    },

    runNode: async (nodeId: string) => {
      const { currentSpaceId, nodes, edges, currentWorkflowId, success, error, setWorkflowStatus, setCurrentRunId, setWorkflowStatuses } = get();
      if (!currentSpaceId) return;
      const blueprint = buildExecutionBlueprint(nodes, edges);

      try {
        const res = await api.runNode(currentSpaceId, blueprint, nodeId, currentWorkflowId || -1);
        if (res && res.error) {
          throw new Error(res.error);
        }
        if (res && res.run_id) {
          setCurrentRunId(res.run_id);
          setWorkflowStatus('running');
          activeRunNodeExecution = { runId: res.run_id, startNodeId: res.start_node ?? nodeId, workflowId: currentWorkflowId };
          if (currentWorkflowId != null) {
            setWorkflowStatuses({ ...get().workflowStatuses, [currentWorkflowId]: 'running' });
            set(state => ({ runIdToWorkflowId: { ...state.runIdToWorkflowId, [res.run_id]: currentWorkflowId } }));
          }
        }
        success(`Node ${nodeId} execution started`);
      } catch (e) {
        console.error(`Failed to run node ${nodeId}`, e);
        error(`Failed to run node: ` + (e instanceof Error ? e.message : String(e)));
        setWorkflowStatus('idle');
      }
    }
  };
};
