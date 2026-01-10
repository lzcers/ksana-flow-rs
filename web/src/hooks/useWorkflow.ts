import { useCallback } from 'react';
import { useImmer } from 'use-immer';
import {
  addEdge,
  applyNodeChanges,
  applyEdgeChanges,
  type NodeChange,
  type EdgeChange,
  type Connection
} from '@xyflow/react';
import type { WorkflowState, NodeType, WorkflowNodeData } from '../types/workflow';

const INITIAL_STATE: WorkflowState = {
  nodes: [
    {
      id: '1',
      type: 'workflow',
      position: { x: 100, y: 100 },
      data: { label: '开始', type: 'start', description: '工作流起点' },
    },
  ],
  edges: [],
  selectedNodeId: null,
};

export function useWorkflow() {
  const [state, updateState] = useImmer<WorkflowState>(INITIAL_STATE);

  const onNodesChange = useCallback((changes: NodeChange[]) => {
    updateState(draft => {
      draft.nodes = applyNodeChanges(changes, draft.nodes) as any;

      // Update selectedNodeId based on changes
      const selectChange = changes.find(c => c.type === 'select');
      if (selectChange && 'selected' in selectChange) {
        if (selectChange.selected) {
          draft.selectedNodeId = selectChange.id;
        } else if (draft.selectedNodeId === selectChange.id) {
          draft.selectedNodeId = null;
        }
      }
    });
  }, [updateState]);

  const onEdgesChange = useCallback((changes: EdgeChange[]) => {
    updateState(draft => {
      draft.edges = applyEdgeChanges(changes, draft.edges) as any;
    });
  }, [updateState]);

  const onConnect = useCallback((connection: Connection) => {
    updateState(draft => {
      draft.edges = addEdge(connection, draft.edges);
    });
  }, [updateState]);

  const addNode = useCallback((type: NodeType) => {
    updateState(draft => {
      const id = Math.random().toString(36).substr(2, 9);
      draft.nodes.push({
        id,
        type: 'workflow',
        position: { x: 300, y: 200 },
        data: {
          label: `新${type === 'task' ? '任务' : type === 'condition' ? '条件' : '节点'}`,
          type,
          description: ''
        },
      });
      draft.selectedNodeId = id;
    });
  }, [updateState]);

  const deleteNode = useCallback((id: string) => {
    updateState(draft => {
      draft.nodes = draft.nodes.filter(n => n.id !== id);
      draft.edges = draft.edges.filter(e => e.source !== id && e.target !== id);
      if (draft.selectedNodeId === id) draft.selectedNodeId = null;
    });
  }, [updateState]);

  const updateNodeData = useCallback((id: string, data: Partial<WorkflowNodeData>) => {
    updateState(draft => {
      const node = draft.nodes.find(n => n.id === id);
      if (node) {
        node.data = { ...node.data, ...data };
      }
    });
  }, [updateState]);

  return {
    state,
    onNodesChange,
    onEdgesChange,
    onConnect,
    addNode,
    deleteNode,
    updateNodeData,
  };
}
