import React from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  type NodeTypes,
  Panel
} from '@xyflow/react';
import { WorkflowNode } from './WorkflowNode';
import type { WorkflowNode as WorkflowNodeType, WorkflowEdge } from '../../types/workflow';

interface CanvasProps {
  nodes: WorkflowNodeType[];
  edges: WorkflowEdge[];
  onNodesChange: (changes: any) => void;
  onEdgesChange: (changes: any) => void;
  onConnect: (connection: any) => void;
}

const nodeTypes: NodeTypes = {
  workflow: WorkflowNode,
};

export const Canvas: React.FC<CanvasProps> = ({
  nodes,
  edges,
  onNodesChange,
  onEdgesChange,
  onConnect
}) => {
  return (
    <main className="flex-1 relative bg-white">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        nodeTypes={nodeTypes}
        fitView
        // Style overrides for clean look
        colorMode="light"
        defaultEdgeOptions={{
          style: { stroke: '#e2e8f0', strokeWidth: 1.5 },
          type: 'smoothstep',
        }}
      >
        <Background color="#f1f5f9" gap={24} size={1.5} />
        <Controls showInteractive={false} className="!bg-white !border-slate-100 !shadow-sm" />

        <Panel position="top-right" className="bg-white/80 backdrop-blur px-3 py-1.5 rounded-full border border-slate-100 text-[10px] font-bold text-slate-400 uppercase tracking-widest">
          React Flow Powered
        </Panel>
      </ReactFlow>
    </main>
  );
};
