const API_BASE = 'http://localhost:3000/api';

export interface NodeMetadata {
    name: string;
    description: string;
    category: string;
    inputs: string[];
    outputs: string[];
    config: any;
}

export interface Workflow {
    id: number;
    name: string;
    blueprint: {
        nodes: any[];
        edges: any[];
    };
}

export const fetchNodes = async (spaceId: string): Promise<NodeMetadata[]> => {
    const res = await fetch(`${API_BASE}/space/${spaceId}/nodes`);
    return res.json();
};

export const fetchWorkflows = async (spaceId: string): Promise<{ id: number; name: string }[]> => {
    const res = await fetch(`${API_BASE}/space/${spaceId}/workflows`);
    return res.json();
};

export const fetchWorkflow = async (spaceId: string, id: number): Promise<Workflow> => {
    const res = await fetch(`${API_BASE}/space/${spaceId}/workflows/${id}`);
    return res.json();
};

export const createWorkflow = async (spaceId: string, name: string, blueprint: any) => {
    const res = await fetch(`${API_BASE}/space/${spaceId}/workflows`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, blueprint }),
    });
    return res.json();
};

export const updateWorkflow = async (spaceId: string, id: number, name: string | undefined, blueprint: any) => {
    const res = await fetch(`${API_BASE}/space/${spaceId}/workflows/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, blueprint }),
    });
    return res.json();
};

export const deleteWorkflow = async (spaceId: string, id: number) => {
    const res = await fetch(`${API_BASE}/space/${spaceId}/workflows/${id}`, {
        method: 'DELETE',
    });
    return res.json();
};

export const runWorkflow = async (spaceId: string, blueprint: any, workflowId: number) => {
    const res = await fetch(`${API_BASE}/space/${spaceId}/workflow/run`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ blueprint, workflow_id: workflowId }),
    });
    return res.json();
};

export const getWorkflowStatus = async (spaceId: string, id: number) => {
    const res = await fetch(`${API_BASE}/space/${spaceId}/workflow/${id}/status`);
    return res.json();
};

export const runNode = async (spaceId: string, blueprint: any, nodeId: string) => {
    const res = await fetch(`${API_BASE}/space/${spaceId}/workflow/run_node`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ blueprint, node_id: nodeId }),
    });
    return res.json();
};

export const pauseWorkflow = async (spaceId: string, runId: string) => {
    const res = await fetch(`${API_BASE}/space/${spaceId}/workflow/${runId}/pause`, {
        method: 'POST',
    });
    return res.json();
};

export const resumeWorkflow = async (spaceId: string, runId: string) => {
    const res = await fetch(`${API_BASE}/space/${spaceId}/workflow/${runId}/resume`, {
        method: 'POST',
    });
    return res.json();
};

export const stopWorkflow = async (spaceId: string, runId: string) => {
    const res = await fetch(`${API_BASE}/space/${spaceId}/workflow/${runId}/stop`, {
        method: 'POST',
    });
    return res.json();
};

export const uploadFile = async (spaceId: string, file: File) => {
    const formData = new FormData();
    formData.append('file', file);
    const res = await fetch(`${API_BASE}/space/${spaceId}/upload`, {
        method: 'POST',
        body: formData,
    });
    if (!res.ok) {
        throw new Error('Upload failed');
    }
    return res.json();
};
