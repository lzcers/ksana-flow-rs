const API_BASE = 'http://localhost:3000/api';

export interface NodeMetadata {
    name: string;
    description: string;
    category: string;
    inputs: string[];
    outputs: string[];
    config: any;
}

export const fetchNodes = async (): Promise<NodeMetadata[]> => {
    const res = await fetch(`${API_BASE}/nodes`);
    return res.json();
};

export const fetchGraph = async () => {
    const res = await fetch(`${API_BASE}/graph`);
    return res.json();
};

export const addNode = async (node: any) => {
    const res = await fetch(`${API_BASE}/graph/node`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(node),
    });
    return res.json();
};

export const removeNode = async (id: string) => {
    const res = await fetch(`${API_BASE}/graph/node/${id}`, {
        method: 'DELETE',
    });
    return res.json();
};

export const updateNodePosition = async (id: string, position: { x: number; y: number }) => {
    const res = await fetch(`${API_BASE}/graph/node/${id}/position`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(position),
    });
    return res.json();
};

export const addEdge = async (edge: any) => {
    const res = await fetch(`${API_BASE}/graph/edge`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(edge),
    });
    return res.json();
};

export const removeEdge = async (id: string) => {
    const res = await fetch(`${API_BASE}/graph/edge/${id}`, {
        method: 'DELETE',
    });
    return res.json();
};

export const runFlow = async () => {
    const res = await fetch(`${API_BASE}/run`, { method: 'POST' });
    return res.json();
};
