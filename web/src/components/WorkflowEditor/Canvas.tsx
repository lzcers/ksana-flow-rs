import React from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  type NodeTypes,
  Panel,
  useReactFlow,
  useViewport
} from '@xyflow/react';
import { WorkflowNode } from './WorkflowNode';
import type { WorkflowNode as WorkflowNodeType, WorkflowEdge } from '../../types/workflow';

interface CanvasProps {
  nodes: WorkflowNodeType[];
  edges: WorkflowEdge[];
  onNodesChange: (changes: any) => void;
  onEdgesChange: (changes: any) => void;
  onNodeDragStop: (event: any, node: any) => void;
  onConnect: (connection: any) => void;
  onAddNode: (type: string, position: { x: number; y: number }) => void;
}

const nodeTypes: NodeTypes = {
  workflow: WorkflowNode,
};

const ZoomDisplay = () => {
  const { zoom } = useViewport();
  return (
    <div className="bg-zinc-900/80 backdrop-blur px-2 py-1 rounded border border-zinc-800 text-[10px] font-bold text-zinc-400 min-w-[40px] text-center">
      {Math.round(zoom * 100)}%
    </div>
  );
};

export const Canvas: React.FC<CanvasProps> = ({
  nodes,
  edges,
  onNodesChange,
  onEdgesChange,
  onNodeDragStop,
  onConnect,
  onAddNode
}) => {
  const { screenToFlowPosition } = useReactFlow();

  const onDragOver = React.useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const onDrop = React.useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();

      const type = event.dataTransfer.getData('application/reactflow');

      // check if the dropped element is valid
      if (typeof type === 'undefined' || !type) {
        return;
      }

      const position = screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });

      // Offset the position to center the node (approximate dimensions: 120x80)
      const centeredPosition = {
        x: position.x - 60,
        y: position.y - 40,
      };

      onAddNode(type, centeredPosition);
    },
    [screenToFlowPosition, onAddNode],
  );

  return (
    <main className="flex-1 relative bg-zinc-950">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeDragStop={onNodeDragStop}
        onConnect={onConnect}
        onDrop={onDrop}
        onDragOver={onDragOver}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={{ maxZoom: 1 }}
        deleteKeyCode={['Backspace', 'Delete']}
        // Style overrides for clean look
        colorMode="dark"
        defaultEdgeOptions={{
          style: { strokeWidth: 2 },
          type: 'smoothstep',
        }}
        connectionLineStyle={{
          stroke: '#3b82f6',
          strokeWidth: 2,
        }}
      >
        <Background color="#27272a" gap={24} size={1.5} />
        <Controls showInteractive={false} className="!bg-zinc-900 !border-zinc-800 !shadow-sm !fill-zinc-400" />

        <Panel position="bottom-left" style={{ marginLeft: '48px' }}>
          <ZoomDisplay />
        </Panel>

        <Panel position="top-right" className="bg-zinc-900/80 backdrop-blur px-3 py-1.5 rounded-full border border-zinc-800 text-[10px] font-bold text-zinc-500 uppercase tracking-widest">
          React Flow Powered
        </Panel>
      </ReactFlow>
    </main>
  );
};
