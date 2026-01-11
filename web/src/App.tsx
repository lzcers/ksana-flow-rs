import React from 'react';
import { useWorkflow } from './hooks/useWorkflow';
import { Sidebar } from './components/WorkflowEditor/Sidebar';
import { Canvas } from './components/WorkflowEditor/Canvas';
import { PropertyPanel } from './components/WorkflowEditor/PropertyPanel';
import { ReactFlowProvider } from '@xyflow/react';

export default function App() {
  const {
    state,
    nodeTypes,
    onNodesChange,
    onEdgesChange,
    onNodeDragStop,
    onConnect,
    addNode,
    deleteNode,
    updateNodeData,
    runWorkflow
  } = useWorkflow();

  const selectedNode = state.nodes.find(n => n.id === state.selectedNodeId);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-white font-sans text-slate-800">
      <Sidebar nodeTypes={nodeTypes} onAddNode={addNode} onRun={runWorkflow} />

      <ReactFlowProvider>
        <Canvas
          nodes={state.nodes}
          edges={state.edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onNodeDragStop={onNodeDragStop}
          onConnect={onConnect}
          onAddNode={addNode}
        />
      </ReactFlowProvider>

      {selectedNode && (
        <PropertyPanel
          node={selectedNode}
          onUpdateData={updateNodeData}
          onDelete={deleteNode}
        />
      )}
    </div>
  );
}
