import type { Edge } from "@xyflow/react";

const API_BASE = import.meta.env.PROD ? '/api' : 'http://localhost:3000/api';

export interface NodeMetadata {
    name: string;
    description: string;
    category: string;
    inputs: string[];
    outputs: string[];
    config: Record<string, unknown>;
}

export interface Workflow {
    id: number;
    name: string;
    blueprint: {
        nodes: Node[];
        edges: Edge[];
    };
}

export const fetchNodes = async (spaceId: string): Promise<NodeMetadata[]> => {
    // spaceId is no longer required by the backend for getting nodes
    void spaceId;
    const res = await fetch(`${API_BASE}/nodes`);
    return res.json();
};

export const fetchWorkflows = async (spaceId: string): Promise<{ id: number; name: string }[]> => {
    const res = await fetch(`${API_BASE}/workflows?space_id=${spaceId}`);
    return res.json();
};

export const fetchWorkflow = async (spaceId: string, id: number): Promise<Workflow> => {
    const res = await fetch(`${API_BASE}/workflows/${id}?space_id=${spaceId}`);
    return res.json();
};

export const createWorkflow = async (spaceId: string, name: string, blueprint: Record<string, unknown>) => {
    const res = await fetch(`${API_BASE}/workflows`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ space_id: spaceId, name, blueprint }),
    });
    return res.json();
};

export const updateWorkflow = async (spaceId: string, id: number, name: string | undefined, blueprint: Record<string, unknown>) => {
    const res = await fetch(`${API_BASE}/workflows/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ space_id: spaceId, name, blueprint }),
    });
    return res.json();
};

export const deleteWorkflow = async (spaceId: string, id: number) => {
    const res = await fetch(`${API_BASE}/workflows/${id}?space_id=${spaceId}`, {
        method: 'DELETE',
    });
    return res.json();
};

export const runWorkflow = async (spaceId: string, blueprint: Record<string, unknown>, workflowId: number) => {
    const res = await fetch(`${API_BASE}/workflow/run`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ space_id: spaceId, blueprint, workflow_id: workflowId }),
    });
    return res.json();
};

export const getWorkflowStatus = async (_spaceId: string, id: number) => {
    const res = await fetch(`${API_BASE}/workflow/${id}/status`);
    return res.json();
};

export const runNode = async (spaceId: string, blueprint: Record<string, unknown>, nodeId: string, workflowId: number) => {
    const res = await fetch(`${API_BASE}/workflow/run_node`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ space_id: spaceId, blueprint, node_id: nodeId, workflow_id: workflowId }),
    });
    return res.json();
};

export const pauseWorkflow = async (_spaceId: string, runId: string) => {
    const res = await fetch(`${API_BASE}/workflow/${runId}/pause`, {
        method: 'POST',
    });
    return res.json();
};

export const resumeWorkflow = async (_spaceId: string, runId: string) => {
    const res = await fetch(`${API_BASE}/workflow/${runId}/resume`, {
        method: 'POST',
    });
    return res.json();
};

export const stopWorkflow = async (_spaceId: string, runId: string) => {
    const res = await fetch(`${API_BASE}/workflow/${runId}/stop`, {
        method: 'POST',
    });
    return res.json();
};

export const uploadFile = async (spaceId: string, file: File) => {
    const formData = new FormData();
    formData.append('space_id', spaceId);
    formData.append('file', file);
    const res = await fetch(`${API_BASE}/upload`, {
        method: 'POST',
        body: formData,
    });
    if (!res.ok) {
        throw new Error('Upload failed');
    }
    return res.json();
};
