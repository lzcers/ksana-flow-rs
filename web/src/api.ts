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

export const fetchNodes = async (): Promise<NodeMetadata[]> => {
    const res = await fetch(`${API_BASE}/nodes`);
    return res.json();
};

export const fetchWorkflows = async (): Promise<{ id: number; name: string }[]> => {
    const res = await fetch(`${API_BASE}/workflows`);
    return res.json();
};

export const fetchWorkflow = async (id: number): Promise<Workflow> => {
    const res = await fetch(`${API_BASE}/workflows/${id}`);
    return res.json();
};

export const createWorkflow = async (name: string, blueprint: any) => {
    const res = await fetch(`${API_BASE}/workflows`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, blueprint }),
    });
    return res.json();
};

export const updateWorkflow = async (id: number, name: string | undefined, blueprint: any) => {
    const res = await fetch(`${API_BASE}/workflows/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, blueprint }),
    });
    return res.json();
};

export const deleteWorkflow = async (id: number) => {
    const res = await fetch(`${API_BASE}/workflows/${id}`, {
        method: 'DELETE',
    });
    return res.json();
};

export const runWorkflow = async (blueprint: any, workflowId: number) => {
    const res = await fetch(`${API_BASE}/workflow/run`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ blueprint, workflow_id: workflowId }),
    });
    return res.json();
};

export const getWorkflowStatus = async (id: number) => {
    const res = await fetch(`${API_BASE}/workflow/${id}/status`);
    return res.json();
};

export const runNode = async (blueprint: any, nodeId: string) => {
    const res = await fetch(`${API_BASE}/workflow/run_node`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ blueprint, node_id: nodeId }),
    });
    return res.json();
};

export const pauseWorkflow = async (runId: string) => {
    const res = await fetch(`${API_BASE}/workflow/${runId}/pause`, {
        method: 'POST',
    });
    return res.json();
};

export const resumeWorkflow = async (runId: string) => {
    const res = await fetch(`${API_BASE}/workflow/${runId}/resume`, {
        method: 'POST',
    });
    return res.json();
};

export const stopWorkflow = async (runId: string) => {
    const res = await fetch(`${API_BASE}/workflow/${runId}/stop`, {
        method: 'POST',
    });
    return res.json();
};
