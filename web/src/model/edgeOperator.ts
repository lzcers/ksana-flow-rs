import { produce } from 'immer';
import type { WorkflowState, Edge, Connection, EdgeChange } from './types';
import { addEdgeXyflow, applyEdgeChangesXyflow } from './utils';


export const addEdge = (state: WorkflowState, edge: Edge): WorkflowState => {
  return produce(state, (draft) => {
    draft.edges.push(edge);
  });
};


export const onConnect = (state: WorkflowState, connection: Connection): WorkflowState => {
  return produce(state, (draft) => {
    draft.edges = addEdgeXyflow(connection, draft.edges);
  });
};


export const applyEdgeChanges = (
  state: WorkflowState,
  changes: EdgeChange<Edge>[]
): WorkflowState => {
  return produce(state, (draft) => {
    const updatedEdges = applyEdgeChangesXyflow(changes, draft.edges);
    draft.edges = updatedEdges;
  });
};


export const setEdges = (state: WorkflowState, edges: Edge[]): WorkflowState => {
  return produce(state, (draft) => {
    draft.edges = edges;
  });
};
