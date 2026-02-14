import { castDraft } from "immer";
import { type StateCreator } from "zustand";
import type { StoreState, Workflow } from "./types";
import * as api from "../api";
import { applyCollapsedSubgraphUi } from "../model/workflow/utils";
import { fromBlueprint, toBlueprint } from "@/model/workflow/adapters";
import { NODE_TYPES } from "@/components/WorkflowEditor/nodeRegistry";
import { makeGraphKey, workflowManager, type GraphKey } from "@/model/workflowManager";
import type { WorkflowStatus } from "@/model/workflow/types";

export const createWorkflow: StateCreator<StoreState, [], [], Workflow> = (set, get) => {
    let autoSaveTimer: ReturnType<typeof setInterval> | null = null;
    const AUTO_SAVE_INTERVAL = 10_000;

    const workflowIdFromGraphKey = (graphKey: GraphKey): number | null => {
        const [, workflowIdRaw] = graphKey.split(":");
        const workflowId = Number(workflowIdRaw);
        return Number.isFinite(workflowId) ? workflowId : null;
    };

    const setGraphStatus = (graphKey: GraphKey, status: WorkflowStatus) => {
        const workflowId = workflowIdFromGraphKey(graphKey);
        if (workflowId == null) return;
        set(state => ({
            currentWorkflowStatus: state.activeGraphKey === graphKey ? status : state.currentWorkflowStatus,
            workflowStatuses: {
                ...state.workflowStatuses,
                [workflowId]: status,
            },
        }));
    };

    workflowManager.subscribe(event => {
        switch (event.type) {
            case "WorkflowStatusChanged": {
                setGraphStatus(event.graphKey, event.status);
                set({ currentRunId: event.runId });
                break;
            }
            case "ModelDestroyed": {
                set(state => {
                    const workflowId = workflowIdFromGraphKey(event.graphKey);
                    if (state.activeGraphKey === event.graphKey) {
                        state.activeGraphKey = null;
                        state.currentRunId = null;
                        state.currentWorkflowStatus = "idle";
                    }
                    if (workflowId) {
                        delete state.workflowStatuses[workflowId];
                    }
                    return state;
                });
                break;
            }
        }
    });

    const saveWorkflowSilent = async () => {
        const { currentSpaceId, nodes, edges, currentWorkflowId, workflows } = get();
        if (!currentSpaceId || currentWorkflowId == null || currentWorkflowId === -1) return;

        const blueprint = toBlueprint(nodes, edges);
        try {
            const currentWf = workflows.find(w => w.id === currentWorkflowId);
            const nameToUse = currentWf?.name || "Untitled";
            await api.updateWorkflow(currentSpaceId, currentWorkflowId, nameToUse, blueprint as any);
        } catch (e) {
            console.error("Auto-save failed", e);
        }
    };

    return {
        workflows: [],
        nodeTypes: [],
        activeGraphKey: null,
        currentSpaceId: null,
        currentWorkflowId: null,
        currentRunId: null,
        currentWorkflowStatus: "idle",
        workflowStatuses: {},

        setActiveGraphKey: (graphKey: GraphKey | null) => {
            const spaceId = graphKey?.split(":")[0];
            if (spaceId) workflowManager.connectWebSocket(spaceId);
            const ins = workflowManager.getModelInstance(graphKey ?? "");
            if (!ins) {
                set({
                    activeGraphKey: null,
                    currentWorkflowId: null,
                    currentRunId: null,
                    currentWorkflowStatus: "idle",
                });
                console.warn(`Failed to get model instance for graphKey: ${graphKey}`);
                return;
            }
            const currentRunId = ins.runId;
            const currentWorkflowStatus = ins.status;
            const currentWorkflowId = ins.workflowId;
            console.log("setActiveGraphKey:", graphKey, currentRunId, currentWorkflowStatus, currentWorkflowId);
            set(state => ({
                activeGraphKey: graphKey,
                currentRunId,
                currentWorkflowStatus,
                currentWorkflowId,
                workflowStatuses:
                    currentWorkflowId == null
                        ? state.workflowStatuses
                        : {
                              ...state.workflowStatuses,
                              [currentWorkflowId]: currentWorkflowStatus,
                          },
            }));
        },

        setSpaceId: id => {
            set({ currentSpaceId: id });
        },

        setWorkflows: workflows => set({ workflows }),

        setNodeTypes: nodeTypes => set({ nodeTypes }),

        setCurrentWorkflowId: id => set({ currentWorkflowId: id }),

        loadMetadata: async () => {
            const { currentSpaceId } = get();
            if (!currentSpaceId) return;
            try {
                const types = await api.fetchNodes(currentSpaceId);
                const wfList = await api.fetchWorkflows(currentSpaceId);

                const allowedTypes = new Set(NODE_TYPES.map(nt => nt.type));
                const filteredTypes = types.filter(t => allowedTypes.has(t.name));
                set({ nodeTypes: filteredTypes, workflows: wfList });
            } catch (e) {
                console.error("Failed to load metadata", e);
            }
        },

        loadWorkflow: async (id: number) => {
            const { currentSpaceId, setActiveGraphKey, switchCanvas, setNodes, setEdges, error } = get();
            if (!currentSpaceId) return;
            const graphKey = makeGraphKey(currentSpaceId, id);
            try {
                const existing = workflowManager.getModelInstance(graphKey);
                if (existing) {
                    setActiveGraphKey(graphKey);
                    switchCanvas(graphKey);
                    return;
                }
                const rxWorkflowInstance = workflowManager.getOrCreate(graphKey);
                setActiveGraphKey(graphKey);
                const wf = await api.fetchWorkflow(currentSpaceId, id);
                const { nodes, edges } = fromBlueprint(wf.blueprint as any);
                const preprocessed = applyCollapsedSubgraphUi(nodes, edges);
                setNodes(castDraft(preprocessed.nodes));
                setEdges(castDraft(preprocessed.edges));
                switchCanvas(graphKey);

                try {
                    const statusRes = await api.getWorkflowStatus(currentSpaceId, id);
                    if (statusRes) {
                        if (statusRes.events && Array.isArray(statusRes.events)) {
                            statusRes.events.forEach((message: any) => {
                                rxWorkflowInstance.applyFlowEvent(message);
                            });
                        }
                    }
                } catch (e) {
                    console.warn("Failed to fetch workflow status", e);
                }
            } catch (e) {
                console.error("Failed to load workflow", e);
                error("Failed to load workflow");
            }
        },

        saveWorkflow: async (name?: string) => {
            const { currentSpaceId, nodes, edges, currentWorkflowId, workflows, switchCanvas, setActiveGraphKey, success, error, setWorkflows } =
                get();
            if (!currentSpaceId) return;
            const blueprint = toBlueprint(nodes, edges);
            try {
                if (currentWorkflowId && currentWorkflowId !== -1) {
                    const currentWf = workflows.find(w => w.id === currentWorkflowId);
                    const nameToUse = name || currentWf?.name || "Untitled";

                    await api.updateWorkflow(currentSpaceId, currentWorkflowId, nameToUse, blueprint as any);
                    success("Workflow saved");
                    if (name && name !== currentWf?.name) {
                        setWorkflows(workflows.map(w => (w.id === currentWorkflowId ? { ...w, name } : w)));
                    }
                } else {
                    const res = await api.createWorkflow(currentSpaceId, name || "Untitled Workflow", blueprint as any);
                    if (res.id) {
                        const graphKey = makeGraphKey(currentSpaceId, res.id);
                        set({
                            currentWorkflowId: res.id,
                            workflows: [...workflows, { id: res.id, name: name || "Untitled Workflow" }],
                        });
                        const ins = workflowManager.getOrCreate(graphKey);
                        ins.model.action.setNodes(nodes);
                        ins.model.action.setEdges(edges);
                        setActiveGraphKey(graphKey);
                        switchCanvas(graphKey);
                        success("Workflow saved");
                    }
                }
            } catch (e) {
                console.error("Failed to save workflow", e);
                error("Failed to save workflow");
            }
        },

        renameWorkflow: async (id: number, newName: string) => {
            const { currentSpaceId, nodes, edges, currentWorkflowId, workflows, success, error, setWorkflows } = get();
            if (!currentSpaceId) return;
            try {
                let blueprint;
                if (id === currentWorkflowId) {
                    blueprint = toBlueprint(nodes, edges);
                } else {
                    const wf = await api.fetchWorkflow(currentSpaceId, id);
                    blueprint = wf.blueprint;
                }

                await api.updateWorkflow(currentSpaceId, id, newName, blueprint as any);
                setWorkflows(workflows.map(w => (w.id === id ? { ...w, name: newName } : w)));
                success("Workflow renamed");
            } catch (e) {
                console.error("Failed to rename workflow", e);
                error("Failed to rename workflow");
            }
        },

        deleteWorkflow: async (id: number) => {
            const {
                currentSpaceId,
                currentWorkflowId,
                workflows,
                success,
                error,
                setWorkflows,
                setCurrentWorkflowId,
                setNodes,
                setEdges,
                selectNode,
            } = get();
            if (!currentSpaceId) return;
            try {
                await api.deleteWorkflow(currentSpaceId, id);
                const graphKey = makeGraphKey(currentSpaceId, id);
                if (graphKey) workflowManager.destroy(graphKey);
                setWorkflows(workflows.filter(w => w.id !== id));
                if (currentWorkflowId === id) {
                    setCurrentWorkflowId(null);
                    setNodes([]);
                    setEdges([]);
                    selectNode([]);
                }
                success("Workflow deleted");
            } catch (e) {
                console.error("Failed to delete workflow", e);
                error("Failed to delete workflow");
            }
        },

        createNewWorkflow: async () => {
            const { currentSpaceId, setActiveGraphKey, switchCanvas, setNodes, setEdges, selectNode } = get();
            if (!currentSpaceId) return;
            const graphKey = `${currentSpaceId}:-1`;
            workflowManager.getOrCreate(graphKey);
            setActiveGraphKey(graphKey);
            switchCanvas(graphKey);
            set({ currentWorkflowId: -1 });
            setNodes([]);
            setEdges([]);
            selectNode([]);
        },

        importWorkflow: (blueprint: any) => {
            const { setNodes, setEdges, selectNode, setCurrentWorkflowId } = get();
            const { nodes, edges } = fromBlueprint(blueprint);

            const preprocessed = applyCollapsedSubgraphUi(nodes, edges);
            setNodes(castDraft(preprocessed.nodes));
            setEdges(castDraft(preprocessed.edges));
            selectNode([]);
            setCurrentWorkflowId(null);
        },

        getWorkflowBlueprint: () => {
            const { nodes, edges } = get();
            return toBlueprint(nodes, edges);
        },

        uploadFile: async (file: File) => {
            const { currentSpaceId } = get();
            if (!currentSpaceId) throw new Error("No active workspace");
            return api.uploadFile(currentSpaceId, file);
        },

        initializeWebSocket: () => {
            const { currentSpaceId } = get();
            if (currentSpaceId) {
                workflowManager.connectWebSocket(currentSpaceId);
            }
            return () => {
                workflowManager.disconnectWebSocket();
            };
        },

        // ===== Workflow Actions =====
        runWorkflow: async () => {
            const { currentSpaceId, nodes, edges, currentWorkflowId, success, error } = get();
            if (!currentSpaceId) return;

            const blueprint = toBlueprint(nodes, edges);
            try {
                const res = await api.runWorkflow(currentSpaceId, blueprint as never, currentWorkflowId || -1);
                const ins = workflowManager.getOrCreate(makeGraphKey(currentSpaceId, currentWorkflowId || -1));
                ins.setRunId(res.run_id);
                if (res && res.error) {
                    throw new Error(res.error);
                }
                if (res && res.run_id) {
                    success("Workflow started");
                }
            } catch (e) {
                console.error("Failed to run workflow", e);
                error("Failed to run workflow: " + (e instanceof Error ? e.message : String(e)));
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

        runNode: async (nodeIds: string[]) => {
            const { currentSpaceId, nodes, edges, currentWorkflowId, success, error } = get();
            if (!currentSpaceId) return;
            const blueprint = toBlueprint(nodes, edges);
            try {
                const res = await api.runNode(currentSpaceId, blueprint as never, nodeIds, currentWorkflowId || -1);
                const ins = workflowManager.getOrCreate(makeGraphKey(currentSpaceId, currentWorkflowId || -1));
                ins.setRunId(res.run_id);
                if (res && res.error) {
                    throw new Error(res.error);
                }
                if (res && res.run_id) {
                    success(`Node ${nodeIds} execution started`);
                }
            } catch (e) {
                console.error(`Failed to run node ${nodeIds}`, e);
                error(`Failed to run node: ` + (e instanceof Error ? e.message : String(e)));
            }
        },

        startAutoSave: () => {
            if (autoSaveTimer) return;
            autoSaveTimer = setInterval(saveWorkflowSilent, AUTO_SAVE_INTERVAL);
        },

        stopAutoSave: () => {
            if (autoSaveTimer) {
                clearInterval(autoSaveTimer);
                autoSaveTimer = null;
            }
        },
    };
};
