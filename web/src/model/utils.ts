import type { WorkflowState, Node, Edge, Connection } from './types';
export { applyNodeChanges as applyNodeChangesXyflow, applyEdgeChanges as applyEdgeChangesXyflow, addEdge as addEdgeXyflow } from '@xyflow/react';

export const getNode = (state: WorkflowState, nodeId: string): Node | undefined => {
  return state.nodes.find((n) => n.id === nodeId);
};


export const getConnectedEdges = (state: WorkflowState, nodeId: string): Edge[] => {
  return state.edges.filter((e) => e.source === nodeId || e.target === nodeId);
};


export const isValidConnection = (_connection: Connection, _state: WorkflowState): boolean => {
  return true;
};
