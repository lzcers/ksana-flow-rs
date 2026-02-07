import React from 'react';
import {
  ReactFlow,
  Background,
  Controls,
  type NodeTypes,
  Panel,
  useReactFlow,
  useViewport,
  type FitViewOptions,
  MarkerType,
  type OnConnectStart,
  type OnConnectEnd,
  useKeyPress
} from '@xyflow/react';
import { Play, Pause, Square } from 'lucide-react';
import { WorkflowNode } from './WorkflowNode';
import { NODE_TYPES } from './nodeTypes';
import { NodeContextMenu } from './NodeContextMenu';
import { SelectionToolbar } from './SelectionToolbar';
import type { Node, Edge } from '../../model/workflow/types';
import type { WorkflowStatus } from '../../hooks/useWorkflow';
import type { NodeMetadata } from '../../api';

interface CanvasProps {
  graphKey?: string | null;
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
  onGroupNodes?: (nodeIds: string[]) => void;
  onPaste?: (nodes: Node[], edges: Edge[]) => void;
  onSave?: () => Promise<void>;
  onUndo?: () => void;
  onRedo?: () => void;
  setConnectionState?: (connecting: boolean, sourceId?: string | null) => void;
}

const nodeTypes: NodeTypes = {
  workflow: WorkflowNode,
  ...Object.fromEntries(NODE_TYPES.map(nt => [nt.type, WorkflowNode]))
};

const defaultEdgeOptions = {
  style: { strokeWidth: 2 },
  type: 'default',
  markerEnd: {
    type: MarkerType.ArrowClosed,
  },
};

const fitViewOptions: FitViewOptions = { maxZoom: 1 };

const ZoomDisplay = () => {
  const { zoom } = useViewport();
  return (
    <div className="px-2 py-1 rounded text-[10px] font-bold text-zinc-400 min-w-[40px] text-center">
      {Math.round(zoom * 100)}%
    </div>
  );
};

export const Canvas: React.FC<CanvasProps> = ({
  graphKey,
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
  onGroupNodes,
  onPaste,
  onSave,
  onUndo,
  onRedo,
  setConnectionState,
}) => {
  const { screenToFlowPosition, getNodes, getEdges } = useReactFlow();

  const spacePressed = useKeyPress('Space');
  const mousePositionRef = React.useRef({ x: 0, y: 0 });

  const onMouseMove = React.useCallback((event: React.MouseEvent) => {
    mousePositionRef.current = { x: event.clientX, y: event.clientY };
  }, []);

  React.useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      const activeElement = document.activeElement;
      const isInputActive = activeElement instanceof HTMLInputElement || activeElement instanceof HTMLTextAreaElement;

      // Undo: Ctrl+Z
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z' && !e.shiftKey) {
        if (isInputActive) return;
        e.preventDefault();
        onUndo?.();
        return;
      }

      // Redo: Ctrl+Y or Ctrl+Shift+Z
      if ((e.ctrlKey || e.metaKey) && (e.key.toLowerCase() === 'y' || (e.key.toLowerCase() === 'z' && e.shiftKey))) {
        if (isInputActive) return;
        e.preventDefault();
        onRedo?.();
        return;
      }

      // Save: Ctrl+S or Cmd+S
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
        e.preventDefault();
        await onSave?.();
        return;
      }

      // Copy: Ctrl+C or Cmd+C
      if ((e.ctrlKey || e.metaKey) && e.key === 'c') {
        if (isInputActive) return;

        const selectedNodes = getNodes().filter(n => n.selected);
        if (selectedNodes.length === 0) return;

        const selectedNodeIds = new Set(selectedNodes.map(n => n.id));
        // Copy edges where both source and target are selected
        const selectedEdges = getEdges().filter(e =>
          selectedNodeIds.has(e.source) && selectedNodeIds.has(e.target)
        );

        const data = {
          nodes: selectedNodes,
          edges: selectedEdges
        };
        await navigator.clipboard.writeText(JSON.stringify(data));
      }

      // Cut: Ctrl+X or Cmd+X
      if ((e.ctrlKey || e.metaKey) && e.key === 'x') {
        if (isInputActive) return;

        const selectedNodes = getNodes().filter(n => n.selected);
        if (selectedNodes.length === 0) return;

        const selectedNodeIds = new Set(selectedNodes.map(n => n.id));
        const selectedEdges = getEdges().filter(e =>
          selectedNodeIds.has(e.source) && selectedNodeIds.has(e.target)
        );

        const data = {
          nodes: selectedNodes,
          edges: selectedEdges
        };
        await navigator.clipboard.writeText(JSON.stringify(data));

        // Delete selected nodes
        onNodesChange(selectedNodes.map(n => ({ type: 'remove', id: n.id })));
      }

      // Paste: Ctrl+V or Cmd+V
      if ((e.ctrlKey || e.metaKey) && e.key === 'v') {
        if (isInputActive) return;
        try {
          const text = await navigator.clipboard.readText();
          const data = JSON.parse(text);
          if (!data.nodes || !Array.isArray(data.nodes)) return;

          // Calculate center of copied nodes
          const nodes = data.nodes;
          const minX = Math.min(...nodes.map((n: any) => n.position.x));
          const minY = Math.min(...nodes.map((n: any) => n.position.y));
          const maxX = Math.max(...nodes.map((n: any) => n.position.x + (n.measured?.width || n.width || 0)));
          const maxY = Math.max(...nodes.map((n: any) => n.position.y + (n.measured?.height || n.height || 0)));
          const centerX = (minX + maxX) / 2;
          const centerY = (minY + maxY) / 2;

          // Target position (Mouse or Viewport Center)
          const targetScreen = mousePositionRef.current;
          const targetPos = screenToFlowPosition(targetScreen);

          const offsetX = targetPos.x - centerX;
          const offsetY = targetPos.y - centerY;

          const newNodes = nodes.map((n: any) => ({
            ...n,
            position: {
              x: n.position.x + offsetX,
              y: n.position.y + offsetY
            }
          }));

          onPaste?.(newNodes, data.edges || []);
        } catch {
          // Ignore invalid JSON or clipboard issues
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [getNodes, getEdges, onPaste, screenToFlowPosition, onSave, onUndo, onRedo, onNodesChange]);

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

  const onConnectStart: OnConnectStart = React.useCallback((_, { nodeId }) => {
    setConnectionState?.(true, nodeId);
  }, [setConnectionState]);

  const onConnectEnd: OnConnectEnd = React.useCallback(() => {
    setConnectionState?.(false, null);
  }, [setConnectionState]);

  return (
    <main className="w-full h-full relative bg-black" onMouseMove={onMouseMove}>
      <ReactFlow
        key={graphKey ?? 'no-graph'}
        panOnDrag={spacePressed}
        selectionOnDrag={!spacePressed}
        panOnScroll={true}
        className={spacePressed ? 'ksana-flow--panning' : undefined}
        nodes={nodes}
        edges={edges}
        onPaneContextMenu={onPaneContextMenu}
        onPaneClick={onPaneClick}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeDragStop={onNodeDragStop}
        onConnect={onConnect}
        onConnectStart={onConnectStart}
        onConnectEnd={onConnectEnd}
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
        onlyRenderVisibleElements={false}
        minZoom={0.1}
        maxZoom={2}
      >
        <Background color="#52525b" bgColor='#000' gap={24} size={1} />
        <Controls
          showInteractive={false}
          className="bg-zinc-900/80! backdrop-blur-xl! border-white/10! shadow-xl! fill-zinc-400! rounded-xl! overflow-hidden! border!"
        />

        <Panel position="bottom-left" style={{ marginLeft: '48px' }}>
          <div className="bg-zinc-900/80 backdrop-blur-md border border-white/10 rounded-lg p-1 shadow-lg">
            <ZoomDisplay />
          </div>
        </Panel>

        <Panel position="bottom-center" className="mb-12">
          <div className="flex items-center gap-3">
            {workflowStatus === 'idle' ? (
              <button
                onClick={onRun}
                className="flex items-center gap-2 px-6 py-2.5 bg-zinc-800/80 hover:bg-zinc-700/80 text-zinc-300 hover:text-zinc-100 border border-zinc-700/50 rounded-full text-sm font-medium shadow-sm hover:shadow-md transition-all backdrop-blur-sm"
              >
                <Play size={18} fill="currentColor" className="opacity-80" />
                Run Workflow
              </button>
            ) : (
              <>
                {workflowStatus === 'running' ? (
                  <button
                    onClick={onPause}
                    className="flex items-center gap-2 px-4 py-2.5 bg-yellow-500/10 hover:bg-yellow-500/20 text-yellow-500/90 hover:text-yellow-400 border border-yellow-500/20 hover:border-yellow-500/30 rounded-full transition-all shadow-sm hover:shadow-md backdrop-blur-sm"
                    title="Pause"
                  >
                    <Pause size={18} fill="currentColor" />
                    <span className="text-sm font-medium">Pause</span>
                  </button>
                ) : (
                  <button
                    onClick={onResume}
                    className="flex items-center gap-2 px-4 py-2.5 bg-green-500/10 hover:bg-green-500/20 text-green-500/90 hover:text-green-400 border border-green-500/20 hover:border-green-500/30 rounded-full transition-all shadow-sm hover:shadow-md backdrop-blur-sm"
                    title="Resume"
                  >
                    <Play size={18} fill="currentColor" />
                    <span className="text-sm font-medium">Resume</span>
                  </button>
                )}
                <button
                  onClick={onStop}
                  className="flex items-center gap-2 px-4 py-2.5 bg-red-500/10 hover:bg-red-500/20 text-red-500/90 hover:text-red-400 border border-red-500/20 hover:border-red-500/30 rounded-full transition-all shadow-sm hover:shadow-md backdrop-blur-sm"
                  title="Stop"
                >
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

        {/* Selection Toolbar - shows when multiple nodes are selected */}
        {onGroupNodes && <SelectionToolbar onGroupNodes={onGroupNodes} />}
      </ReactFlow>
    </main>
  );
};
