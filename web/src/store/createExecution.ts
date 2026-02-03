import type { StateCreator, StoreApi } from 'zustand';
import { BehaviorSubject, Subject, interval, animationFrameScheduler, bufferWhen, filter, map, merge, share, timer, withLatestFrom } from 'rxjs';
import { produce } from 'immer';
import type { StoreState, Execution, WebSocketFlowMessage } from './types';
import * as api from '../api';
import { workflowModel } from './workflowModel';
import type { GraphCommand } from '../model/commands';
import { toBlueprint } from '../model/adapters';

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

        // 1. 准备数据快照和缓存
        const snapshot = workflowModel.getSnapshot();
        const nodeDataCache = new Map<string, any>();

        const getNodeData = (id: string) => {
          if (nodeDataCache.has(id)) return nodeDataCache.get(id);
          const node = snapshot.nodes.find(n => n.id === id);
          const data = node?.data ? { ...node.data } : {}; // Clone data for mutation
          nodeDataCache.set(id, data);
          return data;
        };

        const commands: GraphCommand[] = [];

        // 2. 处理 Store 状态 (currentRunId, workflowStatuses)
        // 我们仍需 setState 来更新这些非 Model 状态
        // 但我们要把 Model 更新剥离出来

        let storeUpdates: Partial<StoreState> = {};
        let runIdToWorkflowIdUpdate: Record<string, number> | undefined;
        let workflowStatusesUpdate: Record<number, string> | undefined;

        // 辅助函数：获取当前状态（优先取 update，再取 get()）
        const getCurrentRunId = () => nextCurrentRunId !== undefined ? nextCurrentRunId : get().currentRunId;
        const getWorkflowStatus = (id: number) => workflowStatusesUpdate?.[id] ?? get().workflowStatuses[id];

        // 3. 遍历批次
        for (const wrapper of batch) {
          const { runId, event: msg } = wrapper;
          const currentRunId = getCurrentRunId();

          let isCurrentRun = !runId || runId === currentRunId;
          if (
            activeRunNodeExecution &&
            runId === activeRunNodeExecution.runId
          ) {
            isCurrentRun = true;
          }

          if (!isCurrentRun) continue;

          // 更新 currentRunId
          if (runId && runId !== currentRunId) {
            nextCurrentRunId = runId;
            const wfId = get().currentWorkflowId;
            if (wfId != null) {
              if (!runIdToWorkflowIdUpdate) runIdToWorkflowIdUpdate = {};
              runIdToWorkflowIdUpdate[runId] = wfId;

              if (getWorkflowStatus(wfId) !== 'running') {
                if (!workflowStatusesUpdate) workflowStatusesUpdate = {};
                workflowStatusesUpdate[wfId] = 'running';
              }
            }
          }

          if (msg) {
            if (typeof msg === 'object') {
              if ('NodeStarted' in msg) {
                const data = getNodeData(msg.NodeStarted);
                data.status = 'running';
              } else if ('NodeStreamStarted' in msg) {
                const data = getNodeData(msg.NodeStreamStarted);
                data.isOutputStream = true;

                // 处理下游 upstreamIsStreaming
                // 这里需要边信息。
                const outEdges = snapshot.edges.filter(e => e.source === msg.NodeStreamStarted);
                for (const edge of outEdges) {
                  const tData = getNodeData(edge.target);
                  tData.upstreamIsStreaming = true;
                }
              } else if ('NodeStreamNextMessage' in msg) {
                const [nodeId, chunk] = msg.NodeStreamNextMessage;
                const data = getNodeData(nodeId);
                if (typeof data.lastMessage !== 'string') data.lastMessage = '';
                data.lastMessage += chunk;
              } else if ('NodeInMessage' in msg) {
                const [nodeId, value] = msg.NodeInMessage;
                const data = getNodeData(nodeId);
                data.lastMessage = value;
                data.lastMessageRunId = runId;
                if (typeof value === 'object' && value !== null) {
                  if (!data.inputs) data.inputs = {};
                  Object.assign(data.inputs, value);
                }
              } else if ('NodeOutMessage' in msg) {
                const [nodeId, value] = msg.NodeOutMessage;
                const data = getNodeData(nodeId);
                data.lastMessage = value;
                data.lastMessageRunId = runId;
                data.isOutputStream = false;
                if (!data.outputs) data.outputs = {};
                data.outputs['output'] = value;

                const outEdges = snapshot.edges.filter(e => e.source === nodeId);
                for (const edge of outEdges) {
                  const tData = getNodeData(edge.target);
                  tData.lastMessage = value;
                  tData.lastMessageRunId = runId;
                  tData.upstreamIsStreaming = false;

                  if (!tData.inputs) tData.inputs = {};
                  tData.inputs[edge.targetHandle || 'default'] = value;
                }
              } else if ('NodeCompleted' in msg) {
                const nodeId = msg.NodeCompleted;
                const data = getNodeData(nodeId);
                data.status = 'completed';

                if (
                  runId &&
                  activeRunNodeExecution &&
                  activeRunNodeExecution.runId === runId &&
                  activeRunNodeExecution.startNodeId === nodeId
                ) {
                  // RunNode 完成逻辑
                  const outEdges = snapshot.edges.filter(e => e.source === nodeId);
                  if (outEdges.length === 0) {
                    finalizeRunNodeRunId = runId;
                    // 重置状态
                    const wfId = activeRunNodeExecution.workflowId ?? get().currentWorkflowId;
                    if (wfId != null) {
                      if (!workflowStatusesUpdate) workflowStatusesUpdate = {};
                      workflowStatusesUpdate[wfId] = 'idle';
                    }
                    storeUpdates.workflowStatus = 'idle';
                    if (getCurrentRunId() !== null) nextCurrentRunId = null;
                    // clean runId map? 
                  }
                }
              } else if ('NodeError' in msg) {
                const [nodeId, error] = msg.NodeError;
                const data = getNodeData(nodeId);
                data.status = 'error';
                data.errorMessage = error;
                data.isOutputStream = false;

                if (
                  activeRunNodeExecution &&
                  activeRunNodeExecution.startNodeId === nodeId &&
                  runId === activeRunNodeExecution.runId
                ) {
                  finalizeRunNodeRunId = runId;
                }
              }
            } else {
              // String messages: FlowFinished, FlowStopped, etc.
              const workflowId = (runId ? (runIdToWorkflowIdUpdate?.[runId] ?? get().runIdToWorkflowId[runId]) : null) ??
                (runId && runId === getCurrentRunId() ? get().currentWorkflowId : null);

              if (msg === 'FlowFinished' || msg === 'FlowStopped') {
                if (workflowId != null) {
                  if (!workflowStatusesUpdate) workflowStatusesUpdate = {};
                  workflowStatusesUpdate[workflowId] = 'idle';
                }
                // delete runId map... handled in setState below

                if (!runId || runId === getCurrentRunId()) {
                  storeUpdates.workflowStatus = 'idle';
                  if (getCurrentRunId() !== null) nextCurrentRunId = null;
                }
              } else if (msg === 'FlowPaused') {
                if (workflowId != null) {
                  if (!workflowStatusesUpdate) workflowStatusesUpdate = {};
                  workflowStatusesUpdate[workflowId] = 'paused';
                }
                if (!runId || runId === getCurrentRunId()) {
                  storeUpdates.workflowStatus = 'paused';
                }
              } else if (msg === 'FlowResumed') {
                if (workflowId != null) {
                  if (!workflowStatusesUpdate) workflowStatusesUpdate = {};
                  workflowStatusesUpdate[workflowId] = 'running';
                }
                if (!runId || runId === getCurrentRunId()) {
                  storeUpdates.workflowStatus = 'running';
                }
              }
            }
          }
        }

        // 4. 生成 Graph Commands
        nodeDataCache.forEach((data, id) => {
          commands.push({
            type: 'UPDATE_NODE_DATA',
            payload: { id, data }
          });
        });

        if (commands.length > 0) {
          workflowModel.dispatch({
            type: 'BATCH',
            payload: { commands }
          });
        }

        // 5. 应用 Store Updates
        setState((state) => {
          return produce(state, (draft) => {
            if (nextCurrentRunId !== undefined) draft.currentRunId = nextCurrentRunId;
            if (runIdToWorkflowIdUpdate) {
              Object.assign(draft.runIdToWorkflowId, runIdToWorkflowIdUpdate);
            }
            if (workflowStatusesUpdate) {
              Object.assign(draft.workflowStatuses, workflowStatusesUpdate);
            }
            Object.assign(draft, storeUpdates);

            // Clean up runIdToWorkflowId for finished/stopped
            for (const wrapper of batch) {
              const { runId, event: msg } = wrapper;
              if ((msg === 'FlowFinished' || msg === 'FlowStopped' || (typeof msg === 'object' && 'NodeCompleted' in msg && finalizeRunNodeRunId === runId)) && runId) {
                delete draft.runIdToWorkflowId[runId];
              }
            }
          });
        });

        if (nextCurrentRunId !== undefined) {
          currentRunIdSubject.next(nextCurrentRunId);
        }

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

      const blueprint = toBlueprint(nodes, edges);

      try {
        workflowModel.dispatch({ type: 'RESET_EXECUTION_STATE', payload: {} });
        setWorkflowStatus('running');

        const res = await api.runWorkflow(currentSpaceId, blueprint as any, currentWorkflowId || -1);
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
      const blueprint = toBlueprint(nodes, edges);

      try {
        const res = await api.runNode(currentSpaceId, blueprint as any, nodeId, currentWorkflowId || -1);
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
