import React from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  type NodeTypes,
  Panel,
  useReactFlow,
  useViewport,
  type FitViewOptions
} from '@xyflow/react';
import { Play, Pause, Square } from 'lucide-react';
import { WorkflowNode } from './WorkflowNode';
import { NodeContextMenu } from './NodeContextMenu';
import type { Node, Edge } from '../../model/types';
import type { WorkflowStatus } from '../../hooks/useWorkflow';
import type { NodeMetadata } from '../../api';

interface CanvasProps {
  nodes: Node[];
  edges: Edge[];
  workflowStatus: WorkflowStatus;
  availableNodes: NodeMetadata[];
  onNodesChange: (changes: any) => void;
  onEdgesChange: (changes: any) => void;
  onNodeDragStop: (event: any, node: any) => void;
  onConnect: (connection: any) => void;
  onAddNode: (type: string, position: { x: number; y: number }) => void;
  onRun: () => void;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
}

const nodeTypes: NodeTypes = {
  workflow: WorkflowNode,
};

const defaultEdgeOptions = {
  style: { strokeWidth: 2 },
  type: 'smoothstep',
};

const fitViewOptions: FitViewOptions = { maxZoom: 1 };

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
  workflowStatus,
  availableNodes,
  onNodesChange,
  onEdgesChange,
  onNodeDragStop,
  onConnect,
  onAddNode,
  onRun,
  onPause,
  onResume,
  onStop,
}) => {
  const { screenToFlowPosition } = useReactFlow();

  const [contextMenu, setContextMenu] = React.useState<{
    visible: boolean;
    x: number;
    y: number;
    flowPosition: { x: number; y: number } | null;
  }>({
    visible: false,
    x: 0,
    y: 0,
    flowPosition: null,
  });

  const onPaneContextMenu = React.useCallback(
    (event: MouseEvent | React.MouseEvent) => {
      event.preventDefault();

      const position = screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });

      setContextMenu({
        visible: true,
        x: event.clientX,
        y: event.clientY,
        flowPosition: position,
      });
    },
    [screenToFlowPosition]
  );

  const onPaneClick = React.useCallback(() => {
    setContextMenu((prev) => ({ ...prev, visible: false }));
  }, []);

  const handleSelectNode = React.useCallback((type: string) => {
    if (contextMenu.flowPosition) {
      const centeredPosition = {
        x: contextMenu.flowPosition.x - 60,
        y: contextMenu.flowPosition.y - 40,
      };
      onAddNode(type, centeredPosition);
    }
    setContextMenu((prev) => ({ ...prev, visible: false }));
  }, [contextMenu.flowPosition, onAddNode]);

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
        onPaneContextMenu={onPaneContextMenu}
        onPaneClick={onPaneClick}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeDragStop={onNodeDragStop}
        onConnect={onConnect}
        onDrop={onDrop}
        onDragOver={onDragOver}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={fitViewOptions}
        deleteKeyCode={['Backspace', 'Delete']}
        // Style overrides for clean look
        colorMode="dark"
        defaultEdgeOptions={defaultEdgeOptions}
        connectionLineStyle={{
          stroke: '#3b82f6',
          strokeWidth: 2,
        }}
        onlyRenderVisibleElements={true}
        minZoom={0.1}
        maxZoom={2}
      >
        <Background color="#27272a" gap={24} size={1.5} />
        <Controls showInteractive={false} className="!bg-zinc-900 !border-zinc-800 !shadow-sm !fill-zinc-400" />

        <Panel position="bottom-left" style={{ marginLeft: '48px' }}>
          <ZoomDisplay />
        </Panel>

        <Panel position="bottom-center" className="mb-8">
          <div className="flex items-center gap-2 bg-zinc-900/90 backdrop-blur border border-zinc-800 p-1.5 rounded-lg shadow-xl">
            {workflowStatus === 'idle' ? (
              <button onClick={onRun} className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-md text-sm font-medium transition-colors">
                <Play size={16} fill="currentColor" />
                Run Workflow
              </button>
            ) : (
              <>
                {workflowStatus === 'running' ? (
                  <button onClick={onPause} className="flex items-center gap-2 px-3 py-2 hover:bg-zinc-800 text-yellow-500 rounded-md transition-colors" title="Pause">
                    <Pause size={18} fill="currentColor" />
                    <span className="text-sm font-medium">Pause</span>
                  </button>
                ) : (
                  <button onClick={onResume} className="flex items-center gap-2 px-3 py-2 hover:bg-zinc-800 text-green-500 rounded-md transition-colors" title="Resume">
                    <Play size={18} fill="currentColor" />
                    <span className="text-sm font-medium">Resume</span>
                  </button>
                )}
                <div className="w-px h-6 bg-zinc-800 mx-1"></div>
                <button onClick={onStop} className="flex items-center gap-2 px-3 py-2 hover:bg-zinc-800 text-red-500 rounded-md transition-colors" title="Stop">
                  <Square size={18} fill="currentColor" />
                  <span className="text-sm font-medium">Stop</span>
                </button>
              </>
            )}
          </div>
        </Panel>

        <NodeContextMenu
          visible={contextMenu.visible}
          position={{ x: contextMenu.x, y: contextMenu.y }}
          nodeTypes={availableNodes}
          onSelect={handleSelectNode}
          onClose={() => setContextMenu(prev => ({ ...prev, visible: false }))}
        />
      </ReactFlow>
    </main>
  );
};
